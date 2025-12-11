
use std::{net::{SocketAddr, ToSocketAddrs}, sync::Arc};

use bytes::BytesMut;
use tokio::{net::{TcpListener, TcpStream}, sync::mpsc};

use crate::{gurad::GurardTire, handler::HandlerTire, request::parse_http_frame, response::HttpResponse};

pub struct HttpServer {
    addr: SocketAddr,
    handlers:Arc<HandlerTire>,
    gurads:Arc<GurardTire>
}

impl HttpServer {
    pub fn new<A: ToSocketAddrs>(a: A,handler:HandlerTire,gurads:GurardTire) -> Self {
        Self { addr: a.to_socket_addrs().unwrap().next().unwrap(),handlers:Arc::new(handler),gurads:Arc::new(gurads)}
    }

    pub async fn start(self) {
        let server = TcpListener::bind(self.addr).await.unwrap();
        loop {
            let (socket, add) = server.accept().await.unwrap();
            println!("new client -> {}", add);
            let handlers = Arc::clone(&self.handlers);
            let guards = Arc::clone(&self.gurads);
            tokio::spawn(async move {
                let e = process(socket,handlers,guards).await;
                println!("{:?}", e);
                println!(" client left -> {}", add);
            });
        }
    }
}


async fn process(socket: TcpStream,handlers:Arc<HandlerTire>,guards:Arc<GurardTire>) -> Result<(), String> {
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

        match guards.guard(&req.req_line.url.clone()[..], req).await {
            Ok(req) => {
                if let Some((_url,handle)) = handlers.get(&req.req_line.url) {
                    println!(" {} -> \n{:?}",req.req_line.url,_url);
                    let res = handle(req).await;
                    let _ =_tx.send(res).await;
                }else {
                    let _ =_tx.send(HttpResponse::not_found()).await;
                }
            }
            Err(res) => {
                let _ =_tx.send(res).await;
            }
        }
    }
}
