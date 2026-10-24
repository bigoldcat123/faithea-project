use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use hyper::server::conn::http1;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tower::ServiceBuilder;

use crate::{
    guard::GuardTrie,
    handler::HandlerTrie,
    io::TokioIo,
    request::BodyLimits,
    server::{
        ServerContext,
        builder::{GlobalErrorHandler, TlsConfig},
        tls::serve_tls,
    },
    service::{self, server_service},
};

pub struct H1Server {
    pub(crate) tls: Option<TlsConfig>,
    pub(crate) addr: SocketAddr,
    pub(crate) handlers: Arc<HandlerTrie>,
    /// Shared reference to guard middleware trie
    pub(crate) guards: Arc<GuardTrie>,
    pub(crate) pre_body_guards: Arc<GuardTrie>,
    pub(crate) body_limits: BodyLimits,
    pub(crate) error_handler: Option<Arc<GlobalErrorHandler>>,
}
impl H1Server {
    fn server_context(&self) -> ServerContext {
        ServerContext::new(
            self.handlers.clone(),
            self.guards.clone(),
            self.error_handler.clone(),
        )
        .with_body_config(self.pre_body_guards.clone(), self.body_limits)
    }
    pub(crate) async fn run(self) -> Result<(), Box<dyn Error>> {
        log::info!(
            "HTTP{} server starting on http{}://{}",
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
        let server = TcpListener::bind(self.addr).await?;

        match self.tls.as_ref() {
            Some(cfg) => self.run_tls(server, cfg).await,
            None => self.run_plain(server).await,
        }
    }

    async fn run_tls(&self, server: TcpListener, cfg: &TlsConfig) -> Result<(), Box<dyn Error>> {
        let acceptor = cfg.tls_acceptor()?;
        let context = self.server_context();
        let s = ServiceBuilder::new().service(server_service(service::h1::serve_http1, context));
        serve_tls(server, acceptor, move |socket| {
            let s = s.clone();
            async move {
                let io = TokioIo::new(socket);
                let s = TowerToHyperService::new(s);
                let res = http1::Builder::new()
                    .serve_connection(io, s)
                    .with_upgrades()
                    .await;
                if let Err(e) = res {
                    log::error!("{e:?}");
                }
            }
        })
        .await?;
        Ok(())
    }

    async fn run_plain(&self, server: TcpListener) -> Result<(), Box<dyn Error>> {
        let context = self.server_context();
        let s = ServiceBuilder::new().service(server_service(service::h1::serve_http1, context));
        loop {
            if let Ok((socket, _addr)) = server.accept().await {
                let s = s.clone();
                tokio::spawn(async move {
                    let io = TokioIo::new(socket);
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
