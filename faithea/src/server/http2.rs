use std::{net::SocketAddr, sync::Arc};

use h2::server::Builder;
use hyper::server::conn::http2;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    io::TokioIo,
    request::HttpRequest,
    response::HttpResponse,
    server::{
        ServerFuncProvider,
        builder::{GlobalErrorHandler, TlsConfig},
        process_request,
    },
    service::{self, my_service_fn},
};

pub struct H2Server {
    pub(crate) tls: Option<TlsConfig>,
    pub(crate) addr: SocketAddr,
    pub(crate) handlers: Arc<HandlerTire>,
    /// Shared reference to guard middleware trie
    pub(crate) guards: Arc<GuardTire>,
    pub(crate) error_handler: Option<Arc<GlobalErrorHandler>>,
}

#[derive(Clone)]
// An Executor that uses the tokio runtime.
pub struct TokioExecutor;

// Implement the `hyper::rt::Executor` trait for `TokioExecutor` so that it can be used to spawn
// tasks in the hyper runtime.
// An Executor allows us to manage execution of tasks which can help us improve the efficiency and
// scalability of the server.
impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

impl H2Server {
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!(
            "HTTP{} server starting on http{}://{} using http2",
            if self.tls.is_some() { "S" } else { "" },
            if self.tls.is_some() { "s" } else { "" },
            self.addr,
        );
        log::info!("Press Ctrl+C to stop the server");
        let listener = TcpListener::bind(self.addr).await?;
        match self.tls {
            Some(ref cfg) => {
                let acceptor = cfg.tls_acceptor()?;

                loop {
                    if let Ok((socket, _addr)) = listener.accept().await
                        && let Ok(socket) = acceptor.clone().accept(socket).await
                    {
                        let io = TokioIo::new(socket);
                        let provider = ServerFuncProvider::new(
                            self.handlers.clone(),
                            self.guards.clone(),
                            self.error_handler.clone(),
                        );
                        tokio::spawn(async move {
                            let _ = http2::Builder::new(TokioExecutor)
                                .enable_connect_protocol()
                                .serve_connection(
                                    io,
                                    my_service_fn(service::h2::serve_http2, provider),
                                )
                                .await;
                        });

                        // let _ = self.deal_with(socket, addr,self.error_handler.clone()).await;
                    }
                }
            }
            None => loop {
                if let Ok((socket, _addr)) = listener.accept().await {
                    let io = TokioIo::new(socket);
                    let provider = ServerFuncProvider::new(
                        self.handlers.clone(),
                        self.guards.clone(),
                        self.error_handler.clone(),
                    );
                    tokio::spawn(async move {
                        let _ = http2::Builder::new(TokioExecutor)
                            .enable_connect_protocol()
                            .serve_connection(io, my_service_fn(service::h2::serve_http2, provider))
                            .await;
                    });
                }
            },
        }
    }

    async fn _deal_with<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
        &self,
        socket: IO,
        _addr: SocketAddr,
        error_handler: Option<Arc<GlobalErrorHandler>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let guards = self.guards.clone();
        let handlers = self.handlers.clone();
        tokio::spawn(async move {
            if let Err(e) = process(socket, guards, handlers, error_handler).await {}
        });

        Ok(())
    }
}

async fn process<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
    socket: IO,
    guards: Arc<GuardTire>,
    handlers: Arc<HandlerTire>,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut h2 = Builder::new()
        .enable_connect_protocol()
        .handshake(socket)
        .await?;
    // let mut h2 = h2::server::handshake(socket).await?;

    while let Some(req) = h2.accept().await {
        let (request, respond) = req?;
        let guards = guards.clone();
        let handlers = handlers.clone();
        let error_handler = error_handler.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<HttpResponse>(16);

        tokio::spawn(async move {
            let mut respond = respond;
            while let Some(r) = rx.recv().await {
                let _ = r.serialize_to_socket_h2(&mut respond).await;
            }
        });
        tokio::spawn(async move {
            if let Ok(request) = HttpRequest::parse_h2_frame(request).await {
                process_request(guards, handlers, request, tx, error_handler).await;
            }
        });
    }

    Ok(())
}
