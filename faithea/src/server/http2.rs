use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use hyper::server::conn::http2;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tower::ServiceBuilder;

use crate::{
    guard::GuardTrie,
    handler::HandlerTrie,
    io::TokioIo,
    server::{
        ServerContext,
        builder::{GlobalErrorHandler, TlsConfig},
    },
    service::{self, server_service},
};

pub struct H2Server {
    pub(crate) tls: Option<TlsConfig>,
    pub(crate) addr: SocketAddr,
    pub(crate) handlers: Arc<HandlerTrie>,
    /// Shared reference to guard middleware trie
    pub(crate) guards: Arc<GuardTrie>,
    pub(crate) error_handler: Option<Arc<GlobalErrorHandler>>,
}
use hyper_util::rt::TokioExecutor;

impl H2Server {
    fn server_context(&self) -> ServerContext {
        ServerContext::new(
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
            if self.addr == SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), self.addr.port())
            {
                SocketAddr::new(local_ip_address::local_ip().unwrap(), self.addr.port())
            } else {
                self.addr
            }
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
        let context = self.server_context();
        let s = ServiceBuilder::new().service(server_service(service::h2::serve_http2, context));
        loop {
            if let Ok((socket, _addr)) = listener.accept().await
                && let Ok(socket) = acceptor.clone().accept(socket).await
            {
                let io = TokioIo::new(socket);
                let s = s.clone();
                tokio::spawn(async move {
                    let s = TowerToHyperService::new(s);

                    let _ = http2::Builder::new(TokioExecutor::new())
                        .enable_connect_protocol()
                        .serve_connection(io, s)
                        .await;
                });

                // let _ = self.deal_with(socket, addr,self.error_handler.clone()).await;
            }
        }
    }

    async fn run_plain(&self, listener: TcpListener) -> Result<(), Box<dyn Error>> {
        let context = self.server_context();
        let s = ServiceBuilder::new().service(server_service(service::h2::serve_http2, context));
        loop {
            if let Ok((socket, _addr)) = listener.accept().await {
                let io = TokioIo::new(socket);
                let s = s.clone();
                tokio::spawn(async move {
                    let s = TowerToHyperService::new(s);
                    let _ = http2::Builder::new(TokioExecutor::new())
                        .enable_connect_protocol()
                        .serve_connection(io, s)
                        .await;
                });
            }
        }
    }
}
