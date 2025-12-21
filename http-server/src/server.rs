use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};

use bytes::BytesMut;
use http::{
    HeaderMap,
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    },
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Sender},
};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    request::{HttpRequest, parse_http_frame},
    response::{HttpResponse, HttpResponseModifier},
    route::Route,
};
pub type HandlerModifier = Box<dyn Fn(&mut HandlerTire, &str)>;
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

    pub fn mount(mut self, pre_fix: &'static str, handlers: Vec<HandlerModifier>) -> Self {
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
    pub fn cors(mut self) -> Self {
        self.handlers.options("/**", |_: HttpRequest| async move {
            let mut res = HttpResponse::new();
            let mut header = HeaderMap::new();
            header.insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
            header.insert(ACCESS_CONTROL_ALLOW_HEADERS, "*".parse().unwrap());
            header.insert(
                ACCESS_CONTROL_ALLOW_METHODS,
                "GET, POST, PUT, DELETE".parse().unwrap(),
            );
            header.insert(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true".parse().unwrap());
            header.modify(&mut res).await?;
            Ok(res)
        });
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
        while let Some(response) = rx.recv().await {
            if let Err(_) = response.serialize_to_socket(&mut writer).await {
                println!("sending response error!");
            }
        }
    });

    let mut buf = BytesMut::with_capacity(4096 * 100); // 4KB
    loop {
        let req = parse_http_frame(&mut reader, &mut buf).await?;
        // println!("{:?}", req);

        match guards
            .guard(&req._inner.uri().path().to_string()[..], req)
            .await
        {
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
    if let Some((_matched_url, handler)) =
        handlers.get_handler(req._inner.uri().path(), req._inner.method().clone())
    {
        req.process_routes(&_matched_url, &Route::from(req._inner.uri().path()));

        req.process_search_param();
        match handler(req).await {
            Ok(res) => {
                let _ = tx.send(res).await;
            }
            Err(mut err) => {
                let mut response = HttpResponse::new();
                if err.modify(&mut response).await.is_ok() {
                    let _ = tx.send(response).await;
                } else {
                    let _ = tx.send(HttpResponse::not_found()).await;
                }
            }
        }
    } else {
        let _ = tx.send(HttpResponse::not_found()).await;
    }
}
