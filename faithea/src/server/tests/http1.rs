use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bytes::Bytes;
use hyper::{server::conn::http1, service::service_fn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::timeout,
};

use crate::{
    data::{Json, inbound::multipart::Part},
    guard::GuardTrie,
    handler::{HandlerTrie, types::HttpHandlerResult},
    io::TokioIo,
    request::{HttpRequest, RequestBody, TryFromRequest},
    response::{HttpResponse, ResponseBody},
    server::ServerContext,
};

struct TestConnection {
    stream: TcpStream,
    server_task: JoinHandle<()>,
}

impl Drop for TestConnection {
    fn drop(&mut self) {
        self.server_task.abort();
    }
}

async fn connect(handlers: HandlerTrie) -> TestConnection {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let stream = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let context = ServerContext::new(Arc::new(handlers), Arc::new(GuardTrie::default()), None);
    let server_task = tokio::spawn(async move {
        let service = service_fn(move |req| crate::service::h1::serve_http1(req, context.clone()));
        let _ = http1::Builder::new()
            .serve_connection(TokioIo::new(socket), service)
            .await;
    });
    TestConnection {
        stream,
        server_task,
    }
}

fn response_with_body(body: impl Into<Bytes>) -> HttpResponse {
    let body = body.into();
    let mut response = HttpResponse::new();
    response.add_header("content-length", body.len().to_string().parse().unwrap());
    response.set_body(ResponseBody::Simple(Some(body)));
    response
}

async fn echo(mut req: HttpRequest) -> HttpHandlerResult {
    let body = match req.body() {
        Some(RequestBody::Simple(body)) => body.clone(),
        None => Bytes::from_static(b"NO_BODY"),
        _ => panic!("expected a simple request body"),
    };
    Ok(response_with_body(body))
}

fn echo_handlers() -> HandlerTrie {
    let mut handlers = HandlerTrie::default();
    handlers.get("/echo", echo);
    handlers.post("/echo", echo);
    handlers
}

fn request(method: &str, path: &str, headers: &str, body: &[u8], close: bool) -> Vec<u8> {
    let mut request =
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n{headers}").into_bytes();
    if close {
        request.extend_from_slice(b"Connection: close\r\n");
    }
    request.extend_from_slice(b"\r\n");
    request.extend_from_slice(body);
    request
}

fn chunked(body: &[u8], trailers: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    for chunk in body.chunks(3) {
        encoded.extend_from_slice(format!("{:X};test=value\r\n", chunk.len()).as_bytes());
        encoded.extend_from_slice(chunk);
        encoded.extend_from_slice(b"\r\n");
    }
    encoded.extend_from_slice(b"0\r\n");
    encoded.extend_from_slice(trailers);
    encoded.extend_from_slice(b"\r\n");
    encoded
}

async fn exchange(handlers: HandlerTrie, request: &[u8]) -> Vec<u8> {
    timeout(Duration::from_secs(5), async {
        let mut connection = connect(handlers).await;
        for fragment in request.chunks(11) {
            connection.stream.write_all(fragment).await.unwrap();
            tokio::task::yield_now().await;
        }
        let mut response = Vec::new();
        connection.stream.read_to_end(&mut response).await.unwrap();
        response
    })
    .await
    .expect("HTTP/1 request timed out")
}

fn response_bodies(mut wire: &[u8]) -> Vec<&[u8]> {
    let mut bodies = Vec::new();
    while !wire.is_empty() {
        let end = wire
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap()
            + 4;
        let headers = std::str::from_utf8(&wire[..end]).unwrap();
        assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"), "{headers}");
        let len = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .unwrap()
            .1
            .trim()
            .parse::<usize>()
            .unwrap();
        bodies.push(&wire[end..end + len]);
        wire = &wire[end + len..];
    }
    bodies
}

#[tokio::test]
async fn chunked_text_matches_content_length_for_get_and_post() {
    let body = b"hello\0world";
    for method in ["GET", "POST"] {
        for (headers, encoded) in [
            (format!("Content-Length: {}\r\n", body.len()), body.to_vec()),
            (
                "Transfer-Encoding: chunked\r\n".to_string(),
                chunked(body, b""),
            ),
        ] {
            let wire = request(method, "/echo", &headers, &encoded, true);
            let response = exchange(echo_handlers(), &wire).await;
            assert_eq!(response_bodies(&response), [body.as_slice()]);
        }
    }
}

#[tokio::test]
async fn chunked_json_reaches_the_json_extractor() {
    let mut handlers = HandlerTrie::default();
    handlers.post("/json", |mut req: HttpRequest| async move {
        let value = Json::<serde_json::Value>::try_from_request(&mut req)?;
        assert_eq!(value.0, serde_json::json!({"message": "hello 世界"}));
        Ok(response_with_body("json accepted"))
    });
    let wire = request(
        "POST",
        "/json",
        "Content-Type: application/json\r\nTransfer-Encoding: chunked\r\n",
        &chunked("{\"message\":\"hello 世界\"}".as_bytes(), b""),
        true,
    );
    let response = exchange(handlers, &wire).await;
    assert_eq!(response_bodies(&response), [b"json accepted".as_slice()]);
}

#[tokio::test]
async fn chunked_multipart_file_is_readable_and_connection_is_reusable() {
    let mut handlers = echo_handlers();
    handlers.post("/upload", |mut req: HttpRequest| async move {
        let Some(RequestBody::Multipart(map)) = req.body() else {
            return Ok(response_with_body("NO_MULTIPART_BODY"));
        };
        assert!(matches!(&map["text"][0], Part::Lit(value) if value == "hello"));
        let Part::File(file) = &map["upload"][0] else {
            panic!("expected a file part");
        };
        assert_eq!(std::fs::read(&file.temp_path).unwrap(), b"file\0bytes");
        Ok(response_with_body("upload accepted"))
    });
    let body = b"--boundary\r\nContent-Disposition: form-data; name=\"text\"\r\n\r\nhello\r\n--boundary\r\nContent-Disposition: form-data; name=\"upload\"; filename=\"sample.bin\"\r\nContent-Type: application/octet-stream\r\n\r\nfile\0bytes\r\n--boundary--\r\n";
    let mut wire = request(
        "POST",
        "/upload",
        "Content-Type: multipart/form-data; boundary=boundary\r\nTransfer-Encoding: chunked\r\nTrailer: X-Checksum\r\n",
        &chunked(body, b"X-Checksum: test\r\n"),
        false,
    );
    wire.extend_from_slice(&request("GET", "/echo", "", b"", true));
    let response = exchange(handlers, &wire).await;
    assert_eq!(
        response_bodies(&response),
        [b"upload accepted".as_slice(), b"NO_BODY".as_slice()]
    );
}

#[tokio::test]
async fn empty_and_bodyless_requests_preserve_existing_body_representation() {
    for (method, headers, body, expected) in [
        (
            "GET",
            "Content-Type: multipart/form-data\r\n",
            &b""[..],
            &b"NO_BODY"[..],
        ),
        ("POST", "", &b""[..], &b"NO_BODY"[..]),
        ("POST", "Content-Length: 0\r\n", &b""[..], &b""[..]),
        (
            "POST",
            "Transfer-Encoding: chunked\r\n",
            &b"0\r\n\r\n"[..],
            &b""[..],
        ),
    ] {
        let wire = request(method, "/echo", headers, body, true);
        let response = exchange(echo_handlers(), &wire).await;
        assert_eq!(response_bodies(&response), [expected]);
    }
}

#[tokio::test]
async fn chunked_trailers_do_not_break_body_reading_or_connection_reuse() {
    for body in [b"".as_slice(), b"hello".as_slice()] {
        let mut wire = request(
            "POST",
            "/echo",
            "Transfer-Encoding: chunked\r\nTrailer: X-Checksum\r\n",
            &chunked(body, b"X-Checksum: test\r\n"),
            false,
        );
        wire.extend_from_slice(&request("GET", "/echo", "", b"", true));
        let response = exchange(echo_handlers(), &wire).await;
        assert_eq!(response_bodies(&response), [body, b"NO_BODY".as_slice()]);
    }
}

#[tokio::test]
async fn truncated_chunked_body_does_not_reach_the_handler() {
    timeout(Duration::from_secs(5), async {
        let called = Arc::new(AtomicBool::new(false));
        let handler_called = called.clone();
        let mut handlers = HandlerTrie::default();
        handlers.post("/echo", move |_| {
            let called = handler_called.clone();
            async move {
                called.store(true, Ordering::SeqCst);
                Ok(response_with_body("unexpected success"))
            }
        });
        let mut connection = connect(handlers).await;
        let wire = request(
            "POST",
            "/echo",
            "Transfer-Encoding: chunked\r\n",
            b"5\r\nhe",
            true,
        );
        connection.stream.write_all(&wire).await.unwrap();
        connection.stream.shutdown().await.unwrap();
        let mut response = Vec::new();
        if let Err(err) = connection.stream.read_to_end(&mut response).await {
            assert_eq!(err.kind(), std::io::ErrorKind::ConnectionReset);
        }
        assert!(!called.load(Ordering::SeqCst));
        assert!(!response.starts_with(b"HTTP/1.1 200 OK"));
    })
    .await
    .expect("truncated HTTP/1 chunked request timed out");
}
