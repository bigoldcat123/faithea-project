
use std::{net::{SocketAddr, ToSocketAddrs}, sync::Arc};

use bytes::BytesMut;
use tokio::{net::{TcpListener, TcpStream}, sync::mpsc};

use crate::{handler::Handler, request::parse_http_frame, response::HttpResponse};

pub struct HttpServer {
    addr: SocketAddr,
    handlers:Arc<Handler>
}

impl HttpServer {
    pub fn new<A: ToSocketAddrs>(a: A,handler:Handler) -> Self {
        Self { addr: a.to_socket_addrs().unwrap().next().unwrap(),handlers:Arc::new(handler)}
    }

    pub async fn start(self) {
        let server = TcpListener::bind(self.addr).await.unwrap();
        loop {
            let (socket, add) = server.accept().await.unwrap();
            println!("new client -> {}", add);
            let handlers = Arc::clone(&self.handlers);
            tokio::spawn(async move {
                let e = process(socket,handlers).await;
                println!("{:?}", e);
                println!(" client left -> {}", add);
            });
        }
    }
}


async fn process(socket: TcpStream,handlers:Arc<Handler>) -> Result<(), String> {
    let (mut r, mut w) = socket.into_split();
    let (_tx, mut rx) = mpsc::channel::<HttpResponse>(10);
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            message.serilize_to_socket(&mut w).await;
        }
    });
    let mut buf = BytesMut::with_capacity(4096);
    loop {
        let req = parse_http_frame(&mut r, &mut buf).await?;
        println!("{:?}", req);

        if let Some(handle) = handlers.get(&req.req_line.url) {
            let res = handle(req).await;
            let _ =_tx.send(res).await;
        }else {
            let _ =_tx.send(HttpResponse::not_found()).await;
        }
    }
}

fn calc() {

}
