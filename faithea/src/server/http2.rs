use std::{net::SocketAddr, sync::Arc};

use h2::server::Builder;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    request::HttpRequest,
    response::HttpResponse,
    server::{builder::{GlobalErrorHandler, TlsConfig}, process_request},
};

pub struct H2Server {
    pub(crate) tls: Option<TlsConfig>,
    pub(crate) addr: SocketAddr,
    pub(crate) handlers: Arc<HandlerTire>,
    /// Shared reference to guard middleware trie
    pub(crate) guards: Arc<GuardTire>,
    pub(crate) error_handler:Option<Arc<GlobalErrorHandler>>
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
                    if let Ok((socket, addr)) = listener.accept().await
                        && let Ok(socket) = acceptor.clone().accept(socket).await
                    {
                        let _ = self.deal_with(socket, addr,self.error_handler.clone()).await;
                    }
                }
            }
            None => loop {
                if let Ok((socket, addr)) = listener.accept().await {
                    let _ = self.deal_with(socket, addr,self.error_handler.clone()).await;
                }
            },
        }
    }

    async fn deal_with<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
        &self,
        socket: IO,
        _addr: SocketAddr,
        error_handler: Option<Arc<GlobalErrorHandler>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("client {} enter", _addr);

        let guards = self.guards.clone();
        let handlers = self.handlers.clone();
        tokio::spawn(async move {
            if let Err(e) = process(socket, guards, handlers,error_handler).await {
                log::error!("{:?}", e);
            }
            log::info!("client {} left", _addr);
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
            let request = HttpRequest::parse_h2(request).await.unwrap();
            process_request(guards, handlers, request, tx,error_handler).await;
        });
    }

    Ok(())
}
