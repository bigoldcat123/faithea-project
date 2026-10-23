use std::pin::Pin;

use faithea_websocket::WebSocket;

use crate::{
    request::HttpRequest,
    response::{HttpResponse, HttpResponseModifier},
};

pub trait HttpHandlerErrorModifier: HttpResponseModifier + Send + Sync {}
impl<T: HttpResponseModifier + Send + Sync> HttpHandlerErrorModifier for T {}

pub type HttpHandlerError = crate::error::Error;

pub type HttpHandlerResult = Result<HttpResponse, HttpHandlerError>;
pub trait HttpHandlerFuture: Future<Output = HttpHandlerResult> + Send + 'static {}
impl<T: Future<Output = HttpHandlerResult> + Send + 'static> HttpHandlerFuture for T {}

type BoxedHttpHandlerFuture = Pin<Box<dyn HttpHandlerFuture>>;

pub trait HttpHandlerTrait:
    Fn(HttpRequest) -> BoxedHttpHandlerFuture + Send + Sync + 'static
{
}
impl<T: Fn(HttpRequest) -> BoxedHttpHandlerFuture + Send + Sync + 'static> HttpHandlerTrait for T {}
pub trait RawHttpHandlerTrait<R: HttpHandlerFuture>:
    Fn(HttpRequest) -> R + Send + Sync + 'static
{
}
impl<R: HttpHandlerFuture, T: Fn(HttpRequest) -> R + Send + Sync + 'static> RawHttpHandlerTrait<R>
    for T
{
}
pub type HttpHandler = Box<dyn HttpHandlerTrait>;

//####### WS HANDLER
//
pub trait WebSocketHandlerFuture: Future<Output = ()> + Send + 'static {}
impl<T: Future<Output = ()> + Send + 'static> WebSocketHandlerFuture for T {}

pub trait WebSocketHandlerTrait:
    Fn(WebSocket, HttpRequest) -> Pin<Box<dyn WebSocketHandlerFuture>> + Send + Sync + 'static
{
}
impl<T: Fn(WebSocket, HttpRequest) -> Pin<Box<dyn WebSocketHandlerFuture>> + Send + Sync + 'static>
    WebSocketHandlerTrait for T
{
}

pub trait RawWebSocketHandlerTrait<R: WebSocketHandlerFuture>:
    Fn(WebSocket, HttpRequest) -> R + Send + Sync + 'static
{
}

impl<R: WebSocketHandlerFuture, T: Fn(WebSocket, HttpRequest) -> R + Send + Sync + 'static>
    RawWebSocketHandlerTrait<R> for T
{
}

pub type WebSocketHandler = Box<dyn WebSocketHandlerTrait>;
pub enum Handler {
    Http(HttpHandler),
    WebSocket(WebSocketHandler),
}
