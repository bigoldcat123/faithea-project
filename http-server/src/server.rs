
use std::net::{SocketAddr, ToSocketAddrs};

use bytes::BytesMut;
use tokio::{fs::File, net::{TcpListener, TcpStream}, sync::mpsc};

use crate::{request::parse_http_frame, response::{HttpResponse, ResponseBody}};

pub struct HttpServer {
    addr: SocketAddr,
}

impl HttpServer {
    pub fn new<A: ToSocketAddrs>(a: A) -> Self {
        Self { addr: a.to_socket_addrs().unwrap().next().unwrap()}
    }

    pub async fn start(self) {
        let server = TcpListener::bind(self.addr).await.unwrap();
        loop {
            let (socket, add) = server.accept().await.unwrap();
            println!("new client -> {}", add);

            tokio::spawn(async move {
                let e = process(socket).await;
                println!("{:?}", e);

                println!(" client left -> {}", add);
            });
        }
    }
}


async fn process(socket: TcpStream) -> Result<(), String> {
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
        let mut res  = HttpResponse::new();
        let f = File::open("/Users/dadigua/Desktop/graduation/http-server/src/bin/server.rs").await.unwrap();
        let len = f.metadata().await.unwrap().len();
        res.set_body(ResponseBody::File(f));
        res.add_header(("content-length",format!("{}",len).as_str()));
        let _ =_tx.send(res).await;
    }
}
