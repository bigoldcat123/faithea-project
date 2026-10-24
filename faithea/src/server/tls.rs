use std::{future::Future, io, sync::Arc, time::Duration};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    time::{sleep, timeout},
};
use tokio_rustls::{TlsAcceptor, server::TlsStream};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PENDING_HANDSHAKES: usize = 128;

#[cfg(test)]
mod tests;

pub(super) async fn serve_tls<F, Fut>(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    serve: F,
) -> io::Result<()>
where
    F: Fn(TlsStream<TcpStream>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    serve_tls_with_limits(
        listener,
        acceptor,
        serve,
        HANDSHAKE_TIMEOUT,
        Arc::new(Semaphore::new(MAX_PENDING_HANDSHAKES)),
    )
    .await
}

async fn serve_tls_with_limits<F, Fut>(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    serve: F,
    handshake_timeout: Duration,
    pending: Arc<Semaphore>,
) -> io::Result<()>
where
    F: Fn(TlsStream<TcpStream>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let serve = Arc::new(serve);
    loop {
        let (socket, addr) = match listener.accept().await {
            Ok(connection) => connection,
            Err(err) => {
                log::warn!("TLS connection accept failed: {err}");
                // Retry transient errors without spinning if resources are exhausted.
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        // Reject excess connections instead of spawning tasks waiting for a permit.
        let Ok(permit) = pending.clone().try_acquire_owned() else {
            log::debug!("TLS handshake limit reached; closing connection from {addr}");
            continue;
        };
        let acceptor = acceptor.clone();
        let serve = serve.clone();
        tokio::spawn(async move {
            let result = timeout(handshake_timeout, acceptor.accept(socket)).await;
            // Only pending handshakes count toward the limit, not HTTP connections.
            drop(permit);
            match result {
                Ok(Ok(socket)) => serve(socket).await,
                Ok(Err(err)) => log::debug!("TLS handshake failed for {addr}: {err}"),
                Err(_) => log::debug!("TLS handshake timed out for {addr}"),
            }
        });
    }
}
