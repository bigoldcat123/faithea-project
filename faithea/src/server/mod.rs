pub mod builder;
mod http1;
mod http2;
use std::{error::Error, net::SocketAddr, sync::Arc};

use bytes::{BufMut, Bytes, BytesMut};
use http::{Response, header::CONNECTION};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, split},
    net::TcpListener,
    sync::mpsc::{self, Sender},
};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    request::{HttpRequest, RequestBody},
    response::{HttpResponse, ResponseBody},
    route::Route,
    server::{
        builder::{HttpServerBuilder, TlsConfig},
        http1::H1Server,
        http2::H2Server,
    },
    websocket::{WebSocketIncommingMessageParser, data::WebSocketDataPayLoad, socket::WebSocket},
};

pub type HandlerModifier = Box<dyn Fn(&mut HandlerTire, &str)>;

pub enum Server {
    H1Server(H1Server),
    H2Server(H2Server),
    // O(HttpServer)
}

impl Server {
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        match self {
            Server::H1Server(server) => server.run().await,
            // Server::O(server) => server.run().await,
            Server::H2Server(server) => server.run().await,
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
    tls: Option<TlsConfig>,
    h2: bool,
}

impl HttpServer {
    pub fn builder() -> HttpServerBuilder {
        Default::default()
    }

    pub async fn start_h1(self) -> Result<(), Box<dyn Error>> {
        println!("HTTP server starting on http://{}", self.addr);
        println!("Press Ctrl+C to stop the server");
        let server = TcpListener::bind(self.addr).await?;
        match self.tls {
            Some(ref cfg) => {
                let acceptor = cfg.tls_acceptor()?;
                loop {
                    let (socket, addr) = server.accept().await?;
                    let socket = acceptor.clone().accept(socket).await?;
                    self.process_h1(socket, addr).await;
                }
            }
            None => loop {
                let (socket, addr) = server.accept().await.unwrap();
                self.process_h1(socket, addr).await;
            },
        }
    }
    async fn start_h2(self) -> Result<(), Box<dyn Error>> {
        println!(
            "HTTP{} server starting on http{}://{}",
            if self.h2 { "S" } else { "" },
            if self.h2 { "s" } else { "" },
            self.addr,
        );
        println!("Press Ctrl+C to stop the server");
        let listener = TcpListener::bind(self.addr).await?;
        match self.tls {
            Some(ref cfg) => {
                let acceptor = cfg.tls_acceptor()?;
                loop {
                    if let Ok((socket, _addr)) = listener.accept().await
                        && let Ok(socket) = acceptor.clone().accept(socket).await
                    {
                        if let Err(e) = self.process_h2(socket).await {
                            println!("{:?}", e);
                        }
                    } else {
                        println!("搞事情?");
                    }
                }
            }
            None => loop {
                if let Ok((socket, _addr)) = listener.accept().await
                    && let Err(e) = self.process_h2(socket).await
                {
                    println!("{:?}", e);
                }
            },
        }
    }
    async fn process_h2<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
        &self,
        socket: IO,
    ) -> Result<(), Box<dyn Error>> {
        let mut h2 = h2::server::handshake(socket).await?;

        while let Some(Ok(request)) = h2.accept().await {
            let (request, mut respond) = request;
            let (p, mut b) = request.into_parts();
            let mut buf = BytesMut::new();
            while let Some(Ok(e)) = b.data().await {
                buf.put(e);
                if b.is_end_stream() {
                    break;
                }
            }
            let b = RequestBody::Simple(buf.freeze());

            let req = HttpRequest::new(p, Some(b));

            let (tx, mut rx) = tokio::sync::mpsc::channel::<HttpResponse>(64);

            tokio::spawn(async move {
                while let Some(r) = rx.recv().await {
                    let (mut p, b) = r._innser.into_parts();
                    // Send the response back to the client
                    // p.headers.remove(CONTENT_LENGTH);
                    p.headers.remove(CONNECTION);
                    println!("{:?}", p.headers);

                    let mut x = respond
                        .send_response(Response::from_parts(p, ()), false)
                        .unwrap();
                    let mut buf = BytesMut::with_capacity(4096);
                    match b {
                        crate::response::ResponseBody::File(mut f) => {
                            while let Ok(size) = f.read_buf(&mut buf).await {
                                let _ = x.send_data(buf.split_to(size).freeze(), size == 0);
                                if size == 0 {
                                    break;
                                }
                            }
                        }
                        crate::response::ResponseBody::Simple(b) => {
                            let _ = x.send_data(b, true);
                        }
                        crate::response::ResponseBody::Empty => {
                            let _ = x.send_data(Bytes::new(), true);
                        }
                        crate::response::ResponseBody::WsBody(_receiver) => {
                            unimplemented!()
                        }
                    }
                }
            });

            process_request(self.guards.clone(), self.handlers.clone(), req, tx).await;
        }
        Ok(())
    }
    async fn process_h1<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
        &self,
        socket: IO,
        addr: SocketAddr,
    ) {
        println!("new client -> {}", addr);
        let handlers = Arc::clone(&self.handlers);
        let guards = Arc::clone(&self.guards);
        tokio::spawn(async move {
            let e = process(socket, handlers, guards).await;
            println!("{:?}", e);
            println!(" client left -> {}", addr);
        });
    }

    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        if self.h2 {
            self.start_h2().await?;
        } else {
            self.start_h1().await?;
        }
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

async fn process_request(
    guards: Arc<GuardTire>,
    handlers: Arc<HandlerTire>,
    req: HttpRequest,
    tx: Sender<HttpResponse>,
) {
    match guard_request(guards, req).await {
        Ok(req) => {
            handle_request(handlers, req, tx.clone()).await;
        }
        Err(res) => {
            let _ = tx.send(res).await;
        }
    }
}

async fn guard_request(
    guards: Arc<GuardTire>,
    req: HttpRequest,
) -> Result<HttpRequest, HttpResponse> {
    guards
        .guard(&req._inner.uri().path().to_string()[..], req)
        .await
}

async fn handle_request(
    handlers: Arc<HandlerTire>,
    mut req: HttpRequest,
    tx: Sender<HttpResponse>,
) {
    use crate::handler::Handler;
    if let Some((_matched_url, handler)) =
        handlers.get_handler(req._inner.uri().path(), req._inner.method().clone())
    {
        req.process_routes(&_matched_url, &Route::from(req._inner.uri().path()));

        req.process_search_param();
        match handler {
            Handler::Http(http_handler) => match http_handler(req).await {
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
            },
            Handler::WbeSocket(ws_handler) => {
                // before_open();

                if let Some(body) = req._inner.body_mut().take()
                    && let RequestBody::WebSocketStreamBody(stream_body) = body
                {
                    let mut r = HttpResponse::new();
                    let (outcomming_message_sender, outcomming_message_receiver) =
                        tokio::sync::mpsc::channel::<WebSocketDataPayLoad>(128);
                    r.set_body(ResponseBody::WsBody(outcomming_message_receiver));
                    tx.send(r).await.unwrap();
                    let (parser, incomming_message_receiver) =
                        WebSocketIncommingMessageParser::new(stream_body);
                    parser.start();
                    let websocket = WebSocket::new(outcomming_message_sender, incomming_message_receiver);
                    ws_handler(websocket, req).await;
                }
            }
        }
    } else {
        let _ = tx.send(HttpResponse::not_found()).await;
    }
}
