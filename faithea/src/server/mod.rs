pub mod builder;
mod http1;
mod http2;
use std::{error::Error, sync::Arc};

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadHalf},
    sync::mpsc::Sender,
};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    request::{HttpRequest, RequestBody},
    response::{HttpResponse, HttpResponseModifier, ResponseBody},
    route::Route,
    server::{
        builder::{GlobalErrorHandler, HttpServerBuilder},
        http1::H1Server,
        http2::H2Server,
    },
    websocket::{
        Http1WebSocketIncommingMessageParser, WebSocketIncommingMessageParser,
        data::WebSocketDataPayLoad, socket::WebSocket,
    },
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

pub struct HttpServer;

impl HttpServer {
    pub fn builder() -> HttpServerBuilder {
        Default::default()
    }
}
pub async fn handle_upgrade_to_websocket<
    IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
>(
    guards: Arc<GuardTire>,
    handlers: Arc<HandlerTire>,
    mut req: HttpRequest,
    tx: Sender<HttpResponse>,
    reader: ReadHalf<IO>,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) {
    *req._inner.body_mut() = Some(RequestBody::WebSocketStreamBodyHttp1(Box::new(reader)));
    process_request(guards, handlers, req, tx, error_handler).await;
}

async fn process_request(
    guards: Arc<GuardTire>,
    handlers: Arc<HandlerTire>,
    req: HttpRequest,
    tx: Sender<HttpResponse>,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) {
    match guard_request(guards, req).await {
        Ok(req) => {
            handle_request(handlers, req, tx.clone(),error_handler).await;
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
    error_handler: Option<Arc<GlobalErrorHandler>>,
) {
    use crate::handler::types::Handler;
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
                    if let Some(x) = error_handler {
                        let mut m = x(err).await;
                        if m.modify(&mut response).await.is_ok() {
                            let _ = tx.send(response).await;
                        } else {
                            let _ = tx.send(HttpResponse::not_found()).await;
                        }
                    } else {
                        if err.modify(&mut response).await.is_ok() {
                            let _ = tx.send(response).await;
                        } else {
                            let _ = tx.send(HttpResponse::not_found()).await;
                        }
                    }
                }
            },
            Handler::WbeSocket(ws_handler) => {
                use RequestBody::*;

                if let Some(body) = req._inner.body_mut().take()
                // && let RequestBody::WebSocketStreamBody(stream_body) = body
                {
                    let mut r = match &body {
                        WebSocketStreamBodyHttp2(_) => HttpResponse::new(),
                        WebSocketStreamBodyHttp1(_) => HttpResponse::websocket_response(&req),
                        _ => unreachable!(),
                    };
                    let (outcomming_message_sender, outcomming_message_receiver) =
                        tokio::sync::mpsc::channel::<WebSocketDataPayLoad>(16);
                    r.set_body(ResponseBody::WsBody(outcomming_message_receiver));
                    tx.send(r).await.unwrap();
                    let incomming_message_receiver = match body {
                        WebSocketStreamBodyHttp2(stream_body) => {
                            let (parser, incomming_message_receiver) =
                                WebSocketIncommingMessageParser::new(
                                    stream_body,
                                    outcomming_message_sender.clone(),
                                );
                            parser.start();
                            incomming_message_receiver
                        }
                        WebSocketStreamBodyHttp1(reader) => {
                            let (parser, incomming_message_receiver) =
                                Http1WebSocketIncommingMessageParser::new(
                                    reader,
                                    outcomming_message_sender.clone(),
                                );
                            parser.start();
                            incomming_message_receiver
                        }
                        _ => {
                            unreachable!()
                        }
                    };
                    let websocket =
                        WebSocket::new(outcomming_message_sender, incomming_message_receiver);
                    ws_handler(websocket, req).await;
                }
            }
        }
    } else {
        let _ = tx.send(HttpResponse::not_found()).await;
    }
}
