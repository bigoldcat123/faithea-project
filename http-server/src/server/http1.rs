use std::{error::Error, net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use tokio::{
    io::{AsyncRead, AsyncWrite, split},
    net::TcpListener,
    sync::mpsc,
};

use crate::{
    guard::GuardTire, handler::HandlerTire, request::HttpRequest, response::HttpResponse, server::{builder::TlsConfig, process_request}
};

pub(crate) struct H1Server {
    pub(crate) tls: Option<TlsConfig>,
    pub(crate) addr: SocketAddr,
    pub(crate) handlers: Arc<HandlerTire>,
    /// Shared reference to guard middleware trie
    pub(crate) guards: Arc<GuardTire>,
}
impl H1Server {
    pub(crate) async fn run(self) -> Result<(), Box<dyn Error>> {
        println!("HTTP server starting on http://{}", self.addr);
        println!("Press Ctrl+C to stop the server");
        let server = TcpListener::bind(self.addr).await?;
        match self.tls {
            Some(ref cfg) => {
                let acceptor = cfg.tls_acceptor()?;
                loop {
                    if let Ok((socket, addr)) = server.accept().await
                        && let Ok(socket) = acceptor.clone().accept(socket).await
                    {
                        let _ = self.deal_with(socket, addr).await;
                    }
                }
            }
            None => loop {
                if let Ok((socket, addr)) = server.accept().await {
                    let _ = self.deal_with(socket, addr).await;
                }
            },
        }
    }

    async fn deal_with<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
        &self,
        socket: IO,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        println!("new client -> {}", addr);
        let handlers = Arc::clone(&self.handlers);
        let guards = Arc::clone(&self.guards);
        tokio::spawn(async move {
            let e = process(socket, handlers, guards).await;
            println!("{:?}", e);
            println!(" client left -> {}", addr);
        });
        Ok(())
    }
}

async fn process<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
    socket: IO,
    handlers: Arc<HandlerTire>,
    guards: Arc<GuardTire>,
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
        let req = HttpRequest::parse_h1(&mut reader, &mut buf).await?;
        // println!("{:?}", req);
        process_request(guards.clone(), handlers.clone(), req, tx.clone()).await;
    }
}
