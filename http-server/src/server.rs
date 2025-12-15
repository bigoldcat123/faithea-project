use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};

use bytes::BytesMut;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Sender},
};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    request::{HttpRequest, parse_http_frame},
    response::HttpResponse,
    route::Route,
};

pub struct HttpServerBuilder {
    handlers: HandlerTire,
    guards: GuardTire,
    addr: SocketAddr,
}
impl HttpServerBuilder {
    pub fn guard<F, O, P>(mut self, route: P, f: F) -> Self
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpRequest, HttpResponse>> + 'static + Send + Sync,
        P: AsRef<str>,
    {
        self.guards.add(route, f);
        self
    }

    pub fn mount(
        mut self,
        pre_fix: &'static str,
        handlers: Vec<Box<dyn Fn(&mut HandlerTire, &str)>>,
    ) -> Self {
        self.handlers.mount(pre_fix, handlers);
        self
    }

    pub fn port(mut self, p: u16) -> Self {
        self.addr.set_port(p);
        self
    }
    pub fn host(mut self, host: &str) -> Self {
        self.addr
            .set_ip(host.parse().expect("in correct ip host eg. 0.0.0.0"));
        self
    }
    pub fn build(self) -> HttpServer {
        HttpServer {
            addr: self.addr,
            handlers: Arc::new(self.handlers),
            guards: Arc::new(self.guards),
        }
    }
}
impl Default for HttpServerBuilder {
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            guards: Default::default(),
            addr: "127.0.0.1:8899".to_socket_addrs().unwrap().next().unwrap(),
        }
    }
}

pub struct HttpServer {
    /// Socket address the server is bound to
    addr: SocketAddr,
    /// Shared reference to handler routing trie
    handlers: Arc<HandlerTire>,
    /// Shared reference to guard middleware trie
    guards: Arc<GuardTire>,
}

impl HttpServer {

    pub fn builder() -> HttpServerBuilder {
        Default::default()
    }

    pub async fn start(self) {
        let server = TcpListener::bind(self.addr).await.unwrap();
        loop {
            let (socket, addr) = server.accept().await.unwrap();
            println!("new client -> {}", addr);
            let handlers = Arc::clone(&self.handlers);
            let guards = Arc::clone(&self.guards);
            tokio::spawn(async move {
                let e = process(socket, handlers, guards).await;
                println!("{:?}", e);
                println!(" client left -> {}", addr);
            });
        }
    }
}

async fn process(
    socket: TcpStream,
    handlers: Arc<HandlerTire>,
    guards: Arc<GuardTire>,
) -> Result<(), String> {
    let (mut reader, mut writer) = socket.into_split();
    let (tx, mut rx) = mpsc::channel::<HttpResponse>(10);

    // Spawn writer task that consumes responses from the channel and writes them to the socket
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            message.serialize_to_socket(&mut writer).await;
        }
    });

    let mut buf = BytesMut::with_capacity(4096);
    loop {
        println!("{}",buf.len());

        let req = parse_http_frame(&mut reader, &mut buf).await?;
        println!("{:?}", req);

        match guards.guard(&req.req_line.url.clone()[..], req).await {
            Ok(req) => {
                handle_request(handlers.clone(), req, tx.clone()).await;
            }
            Err(res) => {
                let _ = tx.send(res).await;
            }
        }
    }
}

async fn handle_request(
    handlers: Arc<HandlerTire>,
    mut req: HttpRequest,
    tx: Sender<HttpResponse>,
) {
    let req_url = req.req_line.url.to_string();
    if let Some((_matched_url, handler)) =
        handlers.get_handler(&req.req_line.url, &req.req_line.method)
    {
        req.process_routes(&_matched_url, &Route::from(req.req_line.url.as_str()));
        req.process_search_param(&req_url);
        match handler(req).await {
            Ok(res) => {
                let _ = tx.send(res).await;
            }
            Err(_s) => {
                let _ = tx.send(HttpResponse::error(_s)).await;
            }
        }
    } else {
        let _ = tx.send(HttpResponse::not_found()).await;
    }
}
