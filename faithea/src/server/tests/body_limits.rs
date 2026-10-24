use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bytes::Bytes;
use http::{Request, StatusCode};
use hyper::{server::conn::http1, service::service_fn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

use crate::{
    handler::HandlerTrie,
    io::TokioIo,
    request::{BodyLimits, HttpRequest, RequestBody},
    response::{HttpResponse, ResponseBody},
    server::{HttpServer, Server, ServerContext, builder::HttpServerBuilder},
};

fn context(builder: HttpServerBuilder) -> ServerContext {
    match builder.build() {
        Server::H1Server(server) => {
            ServerContext::new(server.handlers, server.guards, server.error_handler)
                .with_body_config(server.pre_body_guards, server.body_limits)
        }
        Server::H2Server(server) => {
            ServerContext::new(server.handlers, server.guards, server.error_handler)
                .with_body_config(server.pre_body_guards, server.body_limits)
        }
    }
}

async fn http1_exchange(context: ServerContext, request: &[u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut stream = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let task = tokio::spawn(async move {
        let service = service_fn(move |req| crate::service::h1::serve_http1(req, context.clone()));
        let _ = http1::Builder::new()
            .serve_connection(TokioIo::new(socket), service)
            .await;
    });
    stream.write_all(request).await.unwrap();
    let result = timeout(Duration::from_secs(2), async {
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        String::from_utf8(response).unwrap()
    })
    .await;
    task.abort();
    result.expect("server waited for a rejected or stalled request body")
}

fn limited_builder() -> HttpServerBuilder {
    HttpServer::builder().body_limits(BodyLimits {
        max_body_size: 4,
        read_timeout: Duration::from_millis(50),
        ..BodyLimits::default()
    })
}

#[tokio::test]
async fn http1_rejects_declared_and_chunked_oversized_bodies_and_closes_connection() {
    for request in [
        // No payload: reject Content-Length before asking the client to send it.
        b"POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nExpect: 100-continue\r\n\r\n".as_slice(),
        b"POST /upload HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nab\r\n3\r\ncde\r\n0\r\n\r\n".as_slice(),
    ] {
        let response = http1_exchange(context(limited_builder()), request).await;
        assert!(response.starts_with("HTTP/1.1 413"), "{response}");
        assert!(response.to_ascii_lowercase().contains("connection: close"), "{response}");
    }
}

#[tokio::test]
async fn http1_body_deadline_returns_408_without_waiting_for_eof() {
    let response = http1_exchange(
        context(limited_builder()),
        b"POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\n\r\na",
    )
    .await;
    assert!(response.starts_with("HTTP/1.1 408"), "{response}");
}

#[tokio::test]
async fn http1_pre_body_authentication_runs_before_reading_and_limit_checks() {
    let after_body = Arc::new(AtomicBool::new(false));
    let flag = after_body.clone();
    let builder = limited_builder()
        .pre_body_guard("/**", async |mut req: HttpRequest| {
            assert!(req.body().is_none());
            let mut response = HttpResponse::new();
            *response.inner.status_mut() = StatusCode::UNAUTHORIZED;
            Err(response)
        })
        .guard("/**", move |req| {
            flag.store(true, Ordering::SeqCst);
            async { Ok(req) }
        });
    let response = http1_exchange(context(builder),
        b"POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=test\r\nContent-Length: 999999\r\nExpect: 100-continue\r\n\r\n").await;
    assert!(response.starts_with("HTTP/1.1 401"), "{response}");
    assert!(!after_body.load(Ordering::SeqCst));
}

#[tokio::test]
async fn regular_guards_keep_access_to_the_body_after_pre_body_guards() {
    let builder = limited_builder()
        .pre_body_guard("/**", async |mut req: HttpRequest| {
            assert!(req.body().is_none());
            Ok(req)
        })
        .guard("/**", async |mut req: HttpRequest| {
            assert!(
                matches!(req.body(), Some(RequestBody::Simple(body)) if body.as_ref() == b"abcd")
            );
            let mut response = HttpResponse::new();
            *response.inner.status_mut() = StatusCode::ACCEPTED;
            Err(response)
        });
    let response = http1_exchange(context(builder),
        b"POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\nConnection: close\r\n\r\nabcd").await;
    assert!(response.starts_with("HTTP/1.1 202"), "{response}");
}

#[tokio::test]
async fn http2_rejects_oversized_streams_without_breaking_other_streams() {
    timeout(Duration::from_secs(3), async {
        let mut context = context(limited_builder().h2());
        let called = Arc::new(AtomicBool::new(false));
        let flag = called.clone();
        let mut handlers = HandlerTrie::default();
        handlers.post("/upload", move |_| {
            flag.store(true, Ordering::SeqCst);
            async { Ok(HttpResponse::new()) }
        });
        context.handlers = Arc::new(handlers);
        let mut connection =
            super::connect_http2(move |req| crate::service::h2::serve_http2(req, context.clone()))
                .await;
        let (response, mut stream) = connection
            .sender
            .send_request(
                Request::builder()
                    .method("POST")
                    .uri("http://localhost/upload")
                    .body(())
                    .unwrap(),
                false,
            )
            .unwrap();
        stream.send_data(Bytes::from_static(b"ab"), false).unwrap();
        stream.send_data(Bytes::from_static(b"cde"), false).unwrap();
        let mut response = response.await.unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert!(!response.headers().contains_key("connection"));
        // An early response can be followed by RST_STREAM(NO_ERROR) to stop
        // the unfinished upload. Other resets must still fail this test.
        while let Some(data) = response.body_mut().data().await {
            if let Err(error) = data {
                assert_eq!(error.reason(), Some(h2::Reason::NO_ERROR));
                break;
            }
        }
        assert!(!called.load(Ordering::SeqCst));

        let (response, mut stream) = connection
            .sender
            .send_request(
                Request::builder()
                    .method("POST")
                    .uri("http://localhost/upload")
                    .body(())
                    .unwrap(),
                false,
            )
            .unwrap();
        stream.send_data(Bytes::from_static(b"abcd"), true).unwrap();
        assert_eq!(response.await.unwrap().status(), StatusCode::OK);
        assert!(called.load(Ordering::SeqCst));
    })
    .await
    .expect("HTTP/2 stream isolation test timed out");
}

#[tokio::test]
async fn http2_pre_body_guard_rejects_before_the_client_sends_any_data() {
    let context = context(limited_builder().h2().pre_body_guard(
        "/**",
        async |mut req: HttpRequest| {
            assert!(req.body().is_none());
            let mut response = HttpResponse::new();
            *response.inner.status_mut() = StatusCode::UNAUTHORIZED;
            Err(response)
        },
    ));
    let mut connection =
        super::connect_http2(move |req| crate::service::h2::serve_http2(req, context.clone()))
            .await;
    let (response, _stream) = connection
        .sender
        .send_request(
            Request::builder()
                .method("POST")
                .uri("http://localhost/upload")
                .body(())
                .unwrap(),
            false,
        )
        .unwrap();
    let response = timeout(Duration::from_secs(1), response)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn http2_pre_body_guard_delivers_rejection_body_for_an_unfinished_upload() {
    timeout(Duration::from_secs(3), async {
        let context = context(
            HttpServer::builder()
                .h2()
                .body_limits(BodyLimits {
                    read_timeout: Duration::from_secs(2),
                    ..BodyLimits::default()
                })
                .pre_body_guard("/**", async |mut req: HttpRequest| {
                    assert!(req.body().is_none());
                    let mut response = HttpResponse::new();
                    *response.inner.status_mut() = StatusCode::UNAUTHORIZED;
                    response.set_body(ResponseBody::Simple(Some(Bytes::from_static(b"denied"))));
                    Err(response)
                }),
        );
        let mut connection =
            super::connect_http2(move |req| crate::service::h2::serve_http2(req, context.clone()))
                .await;
        let (response, mut upload) = connection
            .sender
            .send_request(
                Request::builder()
                    .method("POST")
                    .uri("http://localhost/upload")
                    .body(())
                    .unwrap(),
                false,
            )
            .unwrap();
        // Leave the upload open throughout response reading. No reset, including
        // NO_ERROR, may replace any part of the guard's rejection body.
        let mut response = response.await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let mut body = Vec::new();
        while let Some(data) = response.body_mut().data().await {
            let data = data.unwrap_or_else(|error| {
                panic!(
                    "guard rejection body was reset after {} bytes: {error}",
                    body.len()
                )
            });
            body.extend_from_slice(&data);
        }
        assert_eq!(body, b"denied");
        upload.send_data(Bytes::new(), true).unwrap();
    })
    .await
    .expect("guard response waited for the unfinished upload");
}

#[tokio::test]
async fn http2_pre_body_guard_preserves_json_large_and_streaming_rejections() {
    timeout(Duration::from_secs(5), async {
        for (expected, streaming) in [
            (Bytes::from_static(b"{\"error\":\"denied\"}"), false),
            (Bytes::from(vec![b'x'; 128 * 1024]), false),
            (Bytes::from_static(b"streamed rejection"), true),
        ] {
            let payload = expected.clone();
            let context = context(
                HttpServer::builder()
                    .h2()
                    .body_limits(BodyLimits {
                        read_timeout: Duration::from_secs(2),
                        ..BodyLimits::default()
                    })
                    .pre_body_guard("/**", move |req: HttpRequest| {
                        let payload = payload.clone();
                        async move {
                            if req.uri().path() == "/allowed" {
                                return Ok(req);
                            }
                            let mut response = HttpResponse::new();
                            *response.inner.status_mut() = StatusCode::UNAUTHORIZED;
                            if streaming {
                                let (sender, receiver) = tokio::sync::mpsc::channel(2);
                                tokio::spawn(async move {
                                    let middle = payload.len() / 2;
                                    sender.send(payload.slice(..middle)).await.unwrap();
                                    tokio::task::yield_now().await;
                                    sender.send(payload.slice(middle..)).await.unwrap();
                                });
                                response.set_body(ResponseBody::Stream(receiver));
                            } else {
                                response.set_body(ResponseBody::Simple(Some(payload)));
                            }
                            Err(response)
                        }
                    }),
            );
            let mut connection = super::connect_http2(move |req| {
                crate::service::h2::serve_http2(req, context.clone())
            })
            .await;
            let (response, mut upload) = connection
                .sender
                .send_request(
                    Request::builder()
                        .method("POST")
                        .uri("http://localhost/upload")
                        .body(())
                        .unwrap(),
                    false,
                )
                .unwrap();
            upload
                .send_data(Bytes::from_static(b"partial upload"), false)
                .unwrap();
            let mut response = response.await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            let mut received = Vec::new();
            while let Some(data) = response.body_mut().data().await {
                let data = data.expect("guard rejection was reset before response EOF");
                response
                    .body_mut()
                    .flow_control()
                    .release_capacity(data.len())
                    .unwrap();
                received.extend_from_slice(&data);
            }
            assert_eq!(received, expected);
            upload.send_data(Bytes::new(), true).unwrap();

            let (response, _) = connection
                .sender
                .send_request(
                    Request::builder()
                        .uri("http://localhost/allowed")
                        .body(())
                        .unwrap(),
                    true,
                )
                .unwrap();
            assert_eq!(response.await.unwrap().status(), StatusCode::NOT_FOUND);
        }
    })
    .await
    .expect("HTTP/2 rejection response did not finish");
}

#[tokio::test]
async fn http2_rejected_upload_cleanup_has_a_deadline() {
    let context = context(
        HttpServer::builder()
            .h2()
            .body_limits(BodyLimits {
                max_body_size: 4,
                read_timeout: Duration::from_millis(200),
                ..BodyLimits::default()
            })
            .pre_body_guard("/**", async |_: HttpRequest| {
                let mut response = HttpResponse::new();
                *response.inner.status_mut() = StatusCode::UNAUTHORIZED;
                response.set_body(ResponseBody::Simple(Some(Bytes::from_static(b"denied"))));
                Err(response)
            }),
    );
    let mut connection =
        super::connect_http2(move |req| crate::service::h2::serve_http2(req, context.clone()))
            .await;
    let (response, mut upload) = connection
        .sender
        .send_request(
            Request::builder()
                .method("POST")
                .uri("http://localhost/upload")
                .body(())
                .unwrap(),
            false,
        )
        .unwrap();
    let mut response = timeout(Duration::from_secs(1), response)
        .await
        .unwrap()
        .unwrap();
    let mut received = Vec::new();
    while let Some(data) = response.body_mut().data().await {
        received.extend_from_slice(&data.unwrap());
    }
    assert_eq!(received, b"denied");
    // An oversized, unfinished rejected upload stops being consumed and must
    // still be released at the deadline instead of retaining a task forever.
    upload
        .send_data(Bytes::from_static(b"oversized"), false)
        .unwrap();
    let reason = timeout(
        Duration::from_secs(1),
        std::future::poll_fn(|cx| upload.poll_reset(cx)),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(reason, h2::Reason::NO_ERROR);
}

#[tokio::test]
async fn http2_body_read_timeout_returns_408() {
    let context = context(limited_builder().h2());
    let mut connection =
        super::connect_http2(move |req| crate::service::h2::serve_http2(req, context.clone()))
            .await;
    let (response, _stream) = connection
        .sender
        .send_request(
            Request::builder()
                .method("POST")
                .uri("http://localhost/upload")
                .body(())
                .unwrap(),
            false,
        )
        .unwrap();
    let response = timeout(Duration::from_secs(1), response)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
}
