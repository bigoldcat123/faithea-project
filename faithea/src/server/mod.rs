pub mod builder;
mod http1;
mod http2;
mod parser;
use std::{error::Error, future::poll_fn, pin::Pin, sync::Arc};

use bytes::{BufMut, BytesMut};
pub use faithea_io_core::BytesSource;
use faithea_websocket::{WebSocket, WebSocketDataPayLoad, WebSocketIncommingMessageParser};
use h2::RecvStream;
use hyper::body::{Body, Incoming};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadHalf},
    sync::mpsc::Sender,
};

use crate::{
    guard::GuardTire,
    handler::{
        HandlerTire,
        types::{HttpHandler, WebSocketHandler},
    },
    map_str,
    request::{HttpRequest, RequestBody},
    response::{HttpResponse, HttpResponseModifier, ResponseBody},
    route::Route,
    server::{
        builder::{GlobalErrorHandler, HttpServerBuilder},
        http1::H1Server,
        http2::H2Server,
    },
};

pub type HandlerModifier = Box<dyn Fn(&mut HandlerTire, &str)>;

pub enum Server {
    H1Server(H1Server),
    H2Server(H2Server),
    // O(HttpServer)
}
#[derive(Clone)]
pub struct ServerFuncProvider {
    handlers: Arc<HandlerTire>,
    guards: Arc<GuardTire>,
    error_handler: Option<Arc<GlobalErrorHandler>>,
}
impl ServerFuncProvider {
    pub(crate) fn new(
        handlers: Arc<HandlerTire>,
        guards: Arc<GuardTire>,
        error_handler: Option<Arc<GlobalErrorHandler>>,
    ) -> Self {
        Self {
            handlers,
            guards,
            error_handler,
        }
    }
    pub(crate) fn handlers(&self) -> Arc<HandlerTire> {
        self.handlers.clone()
    }
    pub(crate) fn guards(&self) -> Arc<GuardTire> {
        self.guards.clone()
    }
    pub(crate) fn error_handler(&self) -> Option<Arc<GlobalErrorHandler>> {
        self.error_handler.clone()
    }
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
            handle_request(handlers, req, tx.clone(), error_handler).await;
        }
        Err(res) => {
            let _ = tx.send(res).await;
        }
    }
}

pub async fn guard_request(
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
            Handler::Http(http_handler) => {
                process_http_request(http_handler, req, tx, error_handler).await
            }
            Handler::WbeSocket(ws_handler) => process_ws_request(ws_handler, req, tx).await,
        }
    } else {
        let _ = tx.send(HttpResponse::not_found()).await;
    }
}
async fn process_http_request(
    http_handler: &HttpHandler,
    req: HttpRequest,
    tx: Sender<HttpResponse>,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) {
    match http_handler(req).await {
        Ok(res) => {
            let _ = tx.send(res).await;
        }
        Err(mut err) => {
            let mut response = HttpResponse::new();
            if let Some(error_handler) = error_handler {
                let mut m = error_handler(err).await;
                if m.modify(&mut response).await.is_ok() {
                    let _ = tx.send(response).await;
                } else {
                    let _ = tx.send(HttpResponse::not_found()).await;
                }
            } else if err.modify(&mut response).await.is_ok() {
                let _ = tx.send(response).await;
            } else {
                let _ = tx.send(HttpResponse::not_found()).await;
            }
        }
    }
}
async fn process_ws_request(
    ws_handler: &WebSocketHandler,
    mut req: HttpRequest,
    tx: Sender<HttpResponse>,
) {
    if let Some(req_body) = req._inner.body_mut().take()
    // && let RequestBody::WebSocketStreamBody(stream_body) = body
    {
        let mut res = create_ws_res_from_req_body(&req, &req_body);
        let (outcomming_message_sender, outcomming_message_receiver) =
            tokio::sync::mpsc::channel::<WebSocketDataPayLoad>(16);
        res.set_body(ResponseBody::WsBody(outcomming_message_receiver));
        tx.send(res).await.unwrap();
        let incomming_message_receiver =
            get_ws_incomming_message_receiver(req_body, outcomming_message_sender.clone());
        let websocket = WebSocket::new(outcomming_message_sender, incomming_message_receiver);
        ws_handler(websocket, req).await;
    }
}

fn create_ws_res_from_req_body(req: &HttpRequest, req_body: &RequestBody) -> HttpResponse {
    use RequestBody::*;
    match req_body {
        WebSocketStreamBodyHttp2(_) => HttpResponse::new(),
        WebSocketStreamBodyHttp1(_) => HttpResponse::websocket_response(req),
        _ => unreachable!(),
    }
}

fn get_ws_incomming_message_receiver(
    req_body: RequestBody,
    outcomming_message_sender: tokio::sync::mpsc::Sender<WebSocketDataPayLoad>,
) -> tokio::sync::mpsc::Receiver<WebSocketDataPayLoad> {
    use RequestBody::*;
    match req_body {
        WebSocketStreamBodyHttp2(stream_body) => {
            let (parser, incomming_message_receiver) = WebSocketIncommingMessageParser::new(
                Http2BytesSource::new(stream_body),
                outcomming_message_sender,
            );
            parser.start();
            incomming_message_receiver
        }
        WebSocketStreamBodyHttp1(reader) => {
            let (parser, incomming_message_receiver) = WebSocketIncommingMessageParser::new(
                Http1BytesSource::stream(reader),
                outcomming_message_sender,
            );
            parser.start();
            incomming_message_receiver
        }
        _ => {
            unreachable!()
        }
    }
}

pub struct HyperIncommingBytesSource {
    inner: Incoming,
    is_end: bool,
}
impl HyperIncommingBytesSource {
    pub(crate) fn new(incomming: Incoming) -> Self {
        HyperIncommingBytesSource {
            inner: incomming,
            is_end: false,
        }
    }
}

impl BytesSource for HyperIncommingBytesSource {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(frame) = poll_fn(|cx| Pin::new(&mut self.inner).poll_frame(cx)).await {
                if let Ok(frame) = frame {
                    let data = frame
                        .into_data()
                        .map_err(|_| "frame.into_data error".to_string())?;
                    let len = data.len();
                    buf.put(data);
                    Ok(len)
                } else {
                    return Err("(0)    ".into());
                }
            } else {
                self.is_end = true;
                return Ok(0);
            }
        })
    }
    fn is_end(&self) -> bool {
        self.is_end
    }
}

pub(crate) struct Http1BytesSource<SOURCE: AsyncRead + Unpin> {
    source: SOURCE,
    len: usize,
    current_len: usize,
    is_stream: bool,
}
impl<SOURCE: AsyncRead + Unpin> Http1BytesSource<SOURCE> {
    pub(crate) fn new(source: SOURCE, len: usize, current_len: usize) -> Self {
        Self {
            source,
            current_len,
            len,
            is_stream: false,
        }
    }

    pub(crate) fn stream(source: SOURCE) -> Self {
        Self {
            source,
            current_len: 0,
            len: usize::MAX,
            is_stream: true,
        }
    }
}
impl<SOURCE: AsyncRead + Unpin + Send> BytesSource for Http1BytesSource<SOURCE> {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            let res = AsyncReadExt::read_buf(&mut self.source, buf)
                .await
                .map_err(map_str!())?;
            if res == 0 {
                self.current_len = self.len;
                return Err("EOF ERROR".to_string());
            }
            if !self.is_stream {
                self.current_len += res;
            }
            Ok(res)
        })
    }
    fn is_end(&self) -> bool {
        self.current_len >= self.len
    }
}
pub(crate) struct Http2BytesSource {
    source: RecvStream,
}
impl Http2BytesSource {
    pub(crate) fn new(source: RecvStream) -> Self {
        Self { source }
    }
}
impl BytesSource for Http2BytesSource {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(d) = self.source.data().await {
                // println!("{:?}",d.is_err());
                let d = d.map_err(map_str!())?;
                let len = d.len();

                buf.put(d);
                self.source
                    .flow_control()
                    .release_capacity(len)
                    .map_err(map_str!())?;
                Ok(len)
            } else {
                Ok(0)
            }
        })
    }
    fn is_end(&self) -> bool {
        self.source.is_end_stream()
    }
}
