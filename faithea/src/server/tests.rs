use std::{future::Future, time::Duration};

use bytes::Bytes;
use h2::client::SendRequest;
use http::{Request, Response, StatusCode};
use hyper::{server::conn::http2, service::service_fn};
use hyper_util::rt::TokioExecutor;
use tokio::{net::TcpListener, task::JoinHandle, time::timeout};

use super::*;
use crate::{
    data::inbound::multipart::Part,
    error::BodyParseError,
    io::TokioIo,
    request::{HttpRequest, RequestBody, parse_body_frame},
    response::{HttpResponse, ResponseBody},
};

const BOUNDARY: &str = "http2-boundary";

struct TestConnection {
    sender: SendRequest<Bytes>,
    server_task: JoinHandle<()>,
    client_task: JoinHandle<()>,
}

impl Drop for TestConnection {
    fn drop(&mut self) {
        self.server_task.abort();
        self.client_task.abort();
    }
}

async fn connect_http2<F, Fut>(handle: F) -> TestConnection
where
    F: Fn(Request<Incoming>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Response<ResponseBody>, crate::error::Error>> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socket = tokio::net::TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (server_socket, _) = listener.accept().await.unwrap();
    let server_task = tokio::spawn(async move {
        let _ = http2::Builder::new(TokioExecutor::new())
            .serve_connection(TokioIo::new(server_socket), service_fn(handle))
            .await;
    });
    let (sender, connection) = h2::client::handshake(socket).await.unwrap();
    let client_task = tokio::spawn(async move {
        let _ = connection.await;
    });
    TestConnection {
        sender,
        server_task,
        client_task,
    }
}

fn multipart_request() -> Request<()> {
    Request::builder()
        .method("POST")
        .uri("http://localhost/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(())
        .unwrap()
}

#[tokio::test]
async fn http2_empty_data_frames_are_skipped_until_data_and_real_eof() {
    timeout(Duration::from_secs(5), async {
        let mut connection = connect_http2(|req| async move {
            let mut source = HyperIncomingBytesSource::new(req.into_body());
            let mut buf = BytesMut::from(&b"prefix:"[..]);

            assert_eq!(source.read_bytes(&mut buf).await.unwrap(), 5);
            assert_eq!(&buf[..], b"prefix:hello");
            assert!(!source.is_end());

            assert_eq!(source.read_bytes(&mut buf).await.unwrap(), 0);
            assert!(source.is_end());
            assert_eq!(&buf[..], b"prefix:hello");
            Ok(HttpResponse::new().inner)
        })
        .await;

        let (response, mut stream) = connection
            .sender
            .send_request(multipart_request(), false)
            .unwrap();
        stream.send_data(Bytes::new(), false).unwrap();
        stream.send_data(Bytes::new(), false).unwrap();
        stream
            .send_data(Bytes::from_static(b"hello"), false)
            .unwrap();
        stream.send_data(Bytes::new(), false).unwrap();
        stream.send_data(Bytes::new(), true).unwrap();

        assert_eq!(response.await.unwrap().status(), StatusCode::OK);
    })
    .await
    .expect("HTTP/2 byte source test timed out");
}

#[tokio::test]
async fn http2_multipart_accepts_empty_data_frames_between_body_fragments() {
    timeout(Duration::from_secs(5), async {
        let mut handlers = HandlerTrie::default();
        handlers.post("/upload", |mut req: HttpRequest| async move {
            let Some(RequestBody::Multipart(map)) = req.body() else {
                panic!("expected a multipart request");
            };
            assert_eq!(map.len(), 1);
            let parts = map.get("text").unwrap();
            assert_eq!(parts.len(), 1);
            assert!(matches!(&parts[0], Part::Lit(value) if value == "hello 世界"));
            Ok(HttpResponse::new())
        });
        let context = ServerContext::new(
            Arc::new(handlers),
            Arc::new(GuardTrie::default()),
            None,
        );
        let mut connection = connect_http2(move |req| {
            crate::service::h2::serve_http2(req, context.clone())
        })
        .await;

        let (response, mut stream) = connection
            .sender
            .send_request(multipart_request(), false)
            .unwrap();
        let body = format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"text\"\r\n\r\nhello 世界\r\n--{BOUNDARY}--\r\n"
        );
        for chunk in body.as_bytes().chunks(7) {
            // Real HTTP/2 DATA frames, including empty frames without END_STREAM.
            stream.send_data(Bytes::new(), false).unwrap();
            stream.send_data(Bytes::new(), false).unwrap();
            stream
                .send_data(Bytes::copy_from_slice(chunk), false)
                .unwrap();
        }
        stream.send_data(Bytes::new(), true).unwrap();

        assert_eq!(response.await.unwrap().status(), StatusCode::OK);
    })
    .await
    .expect("HTTP/2 multipart upload timed out");
}

#[tokio::test]
async fn http2_truncated_multipart_still_returns_unexpected_eof() {
    timeout(Duration::from_secs(5), async {
        let mut connection = connect_http2(|req| async move {
            let (parts, body) = req.into_parts();
            let source = HyperIncomingBytesSource::new(body);
            let mut buf = BytesMut::new();
            assert!(matches!(
                parse_body_frame(source, &mut buf, &parts.headers).await,
                Err(BodyParseError::UnexpectedEof)
            ));
            Ok(HttpResponse::new().inner)
        })
        .await;

        let (response, mut stream) = connection
            .sender
            .send_request(multipart_request(), false)
            .unwrap();
        stream
            .send_data(Bytes::from(format!("--{BOUNDARY}")), false)
            .unwrap();
        stream.send_data(Bytes::new(), true).unwrap();

        assert_eq!(response.await.unwrap().status(), StatusCode::OK);
    })
    .await
    .expect("HTTP/2 truncated multipart test timed out");
}
