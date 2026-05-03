use std::{error::Error, net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use hyper::{server::conn::http1};
use tokio::{
    io::{AsyncRead, AsyncWrite, split},
    net::TcpListener,
    sync::mpsc,
};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    io::TokioIo,
    request::{HttpRequest, is_websocket_upgrade},
    response::HttpResponse,
    server::{
        ServerFuncProvider,
        builder::{GlobalErrorHandler, TlsConfig},
        handle_upgrade_to_websocket, process_request,
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

        match self.tls {
            Some(ref cfg) => {
                let acceptor = cfg.tls_acceptor()?;
                loop {
                    if let Ok((socket, addr)) = server.accept().await
                        && let Ok(socket) = acceptor.clone().accept(socket).await
                    {
                        // let io = TokioIo::new(socket);
                        // let _ = http1::Builder::new()
                        //     .serve_connection(io, service_fn(crate::service::serve))
                        //     .await;
                        let _ = self
                            .deal_with(socket, addr, self.error_handler.clone())
                            .await;
                    }
                }
            }
            None => loop {
                if let Ok((socket, addr)) = server.accept().await {
                    let provider = self.fun_provider();
                    tokio::spawn(async move {
                        let io = TokioIo::new(socket);
                        let res = http1::Builder::new()
                            .serve_connection(io, my_service_fn(service::serve, provider))
                            .with_upgrades()
                            .await;
                        if let Err(e) = res {
                            log::error!("{e:?}");
                        }
                    });

                }
            },
        }
    }

    async fn deal_with<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
        &self,
        socket: IO,
        addr: SocketAddr,
        error_handler: Option<Arc<GlobalErrorHandler>>,
    ) -> Result<(), Box<dyn Error>> {
        let handlers = Arc::clone(&self.handlers);
        let guards = Arc::clone(&self.guards);
        tokio::spawn(async move {
            if let Err(e) = process(socket, handlers, guards, error_handler).await {}
        });
        Ok(())
    }
}

async fn process<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
    socket: IO,
    handlers: Arc<HandlerTire>,
    guards: Arc<GuardTire>,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) -> Result<(), String> {
    let (mut reader, mut writer) = split(socket);
    let (tx, mut rx) = mpsc::channel::<HttpResponse>(10);

    // Spawn writer task that consumes responses from the channel and writes them to the socket
    tokio::spawn(async move {
        while let Some(response) = rx.recv().await {
            if response.serialize_to_socket_h1(&mut writer).await.is_err() {
                println!("sending response error!");
            }
        }
    });

    let mut buf = BytesMut::with_capacity(4096 * 100); // 4KB
    loop {
        let (guards, handlers, tx, error_handler) = (
            guards.clone(),
            handlers.clone(),
            tx.clone(),
            error_handler.clone(),
        );
        let req = HttpRequest::parse_h1_frame(&mut reader, &mut buf).await?;
        log::info!("{:#?}", req._inner.uri());

        if is_websocket_upgrade(&req) {
            handle_upgrade_to_websocket(guards, handlers, req, tx, reader, error_handler).await;
            break;
        } else {
            //  no need to spawn a new tast, as the client side will not send a new req before receving response...
            process_request(guards, handlers, req, tx, error_handler).await;
        }
    }
    Ok(())
}
