use std::{net::SocketAddr, path::Path, time::Duration};

use hyper::{server::conn::http1, service::service_fn};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use tokio::{io::AsyncReadExt, task::JoinHandle};

use super::*;
use crate::{io::TokioIo, response::HttpResponse};

struct TestServer {
    addr: SocketAddr,
    pending: Arc<Semaphore>,
    task: JoinHandle<io::Result<()>>,
}

impl TestServer {
    async fn start(limit: usize, handshake_timeout: Duration) -> Self {
        // Use the repository's development certificate; the local test client
        // ignores its trust/expiry so the tests do not depend on certificate dates.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let certs = CertificateDer::pem_file_iter(root.join("cert.pem"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let key = PrivateKeyDer::from_pem_file(root.join("key.pem")).unwrap();
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let pending = Arc::new(Semaphore::new(limit));
        let task = tokio::spawn(serve_tls_with_limits(
            listener,
            TlsAcceptor::from(Arc::new(config)),
            |socket| async move {
                let _ = http1::Builder::new()
                    .serve_connection(
                        TokioIo::new(socket),
                        service_fn(|_| async { Ok::<_, io::Error>(HttpResponse::new().inner) }),
                    )
                    .await;
            },
            handshake_timeout,
            pending.clone(),
        ));
        Self {
            addr,
            pending,
            task,
        }
    }

    async fn wait_for_permits(&self, expected: usize) {
        timeout(Duration::from_secs(3), async {
            while self.pending.available_permits() != expected {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("handshake permits were not released/acquired");
    }

    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .no_proxy()
            .http1_only()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap()
    }

    fn url(&self) -> String {
        format!("https://{}/", self.addr)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn assert_closed(socket: &mut TcpStream) {
    let result = timeout(Duration::from_secs(3), socket.read(&mut [0_u8; 1]))
        .await
        .expect("connection was not closed");
    match result {
        Ok(0) => {}
        Err(err) if err.kind() == io::ErrorKind::ConnectionReset => {}
        other => panic!("expected EOF or a reset, got {other:?}"),
    }
}

#[tokio::test]
async fn slow_handshake_does_not_block_later_https_requests() {
    let server = TestServer::start(2, Duration::from_secs(10)).await;
    let _slow = TcpStream::connect(server.addr).await.unwrap();
    server.wait_for_permits(1).await;

    let response = server.client().get(server.url()).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    server.wait_for_permits(1).await;
}

#[tokio::test]
async fn stalled_handshake_times_out_and_releases_its_permit() {
    let server = TestServer::start(1, Duration::from_millis(200)).await;
    let mut slow = TcpStream::connect(server.addr).await.unwrap();
    server.wait_for_permits(0).await;
    assert_closed(&mut slow).await;
    server.wait_for_permits(1).await;

    let response = server.client().get(server.url()).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn excess_handshakes_are_closed_and_disconnect_releases_capacity() {
    let server = TestServer::start(1, Duration::from_secs(10)).await;
    let slow = TcpStream::connect(server.addr).await.unwrap();
    server.wait_for_permits(0).await;
    let mut excess = TcpStream::connect(server.addr).await.unwrap();
    assert_closed(&mut excess).await;
    assert_eq!(server.pending.available_permits(), 0);

    drop(slow);
    server.wait_for_permits(1).await;
    let response = server.client().get(server.url()).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn established_keep_alive_connections_do_not_use_handshake_capacity() {
    let server = TestServer::start(1, Duration::from_secs(10)).await;
    let first_client = server.client();
    let response = first_client.get(server.url()).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    response.bytes().await.unwrap();
    server.wait_for_permits(1).await;

    // A separate client must open a new connection while the first stays pooled.
    let response = server.client().get(server.url()).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}
