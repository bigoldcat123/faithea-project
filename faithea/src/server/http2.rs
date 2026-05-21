use std::{error::Error, net::SocketAddr, sync::Arc};

use hyper::server::conn::http2;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tower::ServiceBuilder;

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    io::TokioIo,
    server::{
        ServerFuncProvider,
        builder::{GlobalErrorHandler, TlsConfig},
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
    fn fun_provider(&self) -> ServerFuncProvider {
        ServerFuncProvider::new(
            self.handlers.clone(),
            self.guards.clone(),
            self.error_handler.clone(),
        )
    }

    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        log::info!(
            "HTTP{} server starting on http{}://{} using http2",
            if self.tls.is_some() { "S" } else { "" },
            if self.tls.is_some() { "s" } else { "" },
            self.addr,
        );
        log::info!("Press Ctrl+C to stop the server");
        let listener = TcpListener::bind(self.addr).await?;
        match self.tls.as_ref() {
            Some(cfg) => self.run_tls(listener, cfg).await,
            None => self.run_plain(listener).await,
        }
    }

    async fn run_tls(&self, listener: TcpListener, cfg: &TlsConfig) -> Result<(), Box<dyn Error>> {
        let acceptor = cfg.tls_acceptor()?;
        let provider = self.fun_provider();
        let s = ServiceBuilder::new().service(my_service_fn(service::h2::serve_http2, provider));
        loop {
            if let Ok((socket, _addr)) = listener.accept().await
                && let Ok(socket) = acceptor.clone().accept(socket).await
            {
                let io = TokioIo::new(socket);
                let s = s.clone();
                tokio::spawn(async move {
                    let s = TowerToHyperService::new(s);

                    let _ = http2::Builder::new(TokioExecutor)
                        .enable_connect_protocol()
                        .serve_connection(io, s)
                        .await;
                });

                // let _ = self.deal_with(socket, addr,self.error_handler.clone()).await;
            }
        }
    }

    async fn run_plain(&self, listener: TcpListener) -> Result<(), Box<dyn Error>> {
        let provider = self.fun_provider();
        let s = ServiceBuilder::new().service(my_service_fn(service::h2::serve_http2, provider));
        loop {
            if let Ok((socket, _addr)) = listener.accept().await {
                let io = TokioIo::new(socket);
                let s = s.clone();
                tokio::spawn(async move {
                    let s = TowerToHyperService::new(s);
                    let _ = http2::Builder::new(TokioExecutor)
                        .enable_connect_protocol()
                        .serve_connection(io, s)
                        .await;
                });
            }
        }
    }
}
