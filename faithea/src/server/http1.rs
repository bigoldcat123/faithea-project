use std::{error::Error, net::SocketAddr, sync::Arc};

use hyper::server::conn::http1;
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

pub struct H1Server {
    pub(crate) tls: Option<TlsConfig>,
    pub(crate) addr: SocketAddr,
    pub(crate) handlers: Arc<HandlerTire>,
    /// Shared reference to guard middleware trie
    pub(crate) guards: Arc<GuardTire>,
    pub(crate) error_handler: Option<Arc<GlobalErrorHandler>>,
}
impl H1Server {
    fn fun_provider(&self) -> ServerFuncProvider {
        ServerFuncProvider::new(
            self.handlers.clone(),
            self.guards.clone(),
            self.error_handler.clone(),
        )
    }
    pub(crate) async fn run(self) -> Result<(), Box<dyn Error>> {
        log::info!(
            "HTTP{} server starting on http{}://{}",
            if self.tls.is_some() { "S" } else { "" },
            if self.tls.is_some() { "s" } else { "" },
            self.addr
        );
        log::info!("Press Ctrl+C to stop the server");
        let server = TcpListener::bind(self.addr).await?;

        match self.tls.as_ref() {
            Some(cfg) => self.run_tls(server, cfg).await,
            None => self.run_plain(server).await,
        }
    }

    async fn run_tls(&self, server: TcpListener, cfg: &TlsConfig) -> Result<(), Box<dyn Error>> {
        let acceptor = cfg.tls_acceptor()?;
        loop {
            if let Ok((socket, _addr)) = server.accept().await
                && let Ok(socket) = acceptor.clone().accept(socket).await
            {
                let provider = self.fun_provider();
                tokio::spawn(async move {
                    let io = TokioIo::new(socket);
                    let s = ServiceBuilder::new()
                        .service(my_service_fn(service::h1::serve_http1, provider));
                    let s = TowerToHyperService::new(s);
                    let res = http1::Builder::new()
                        .serve_connection(io, s)
                        .with_upgrades()
                        .await;
                    if let Err(e) = res {
                        log::error!("{e:?}");
                    }
                });
            }
        }
    }

    async fn run_plain(&self, server: TcpListener) -> Result<(), Box<dyn Error>> {
        loop {
            if let Ok((socket, _addr)) = server.accept().await {
                let provider = self.fun_provider();
                tokio::spawn(async move {
                    let io = TokioIo::new(socket);
                    let s = ServiceBuilder::new()
                        .service(my_service_fn(service::h1::serve_http1, provider));
                    let s = TowerToHyperService::new(s);

                    let res = http1::Builder::new()
                        .serve_connection(io, s)
                        .with_upgrades()
                        .await;
                    if let Err(e) = res {
                        log::error!("{e:?}");
                    }
                });
            }
        }
    }
}
