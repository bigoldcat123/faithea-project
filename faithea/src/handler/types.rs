use std::pin::Pin;

use faithea_websocket::WebSocket;

use crate::{
    request::HttpRequest,
    response::{HttpResponse, HttpResponseModifier},
};

pub trait HttphandlerErrorTrait: HttpResponseModifier + Send + Sync {}
impl<T: HttpResponseModifier + Send + Sync> HttphandlerErrorTrait for T {}

pub type HttpHandlerError = crate::error::Error;

pub type HttpHandlerResultOutput = Result<HttpResponse, HttpHandlerError>;
pub trait HttpHandlerResultTrait:
    Future<Output = HttpHandlerResultOutput> + Send + 'static
{
}
impl<T: Future<Output = HttpHandlerResultOutput> + Send + 'static> HttpHandlerResultTrait for T {}

type HttpHandlerResult = Pin<Box<dyn HttpHandlerResultTrait>>;

pub trait HttpHandlerTrait: Fn(HttpRequest) -> HttpHandlerResult + Send + Sync + 'static {}
impl<T: Fn(HttpRequest) -> HttpHandlerResult + Send + Sync + 'static> HttpHandlerTrait for T {}
pub trait RawHttpHandlerTrait<R: HttpHandlerResultTrait>:
    Fn(HttpRequest) -> R + Send + Sync + 'static
{
}
impl<R: HttpHandlerResultTrait, T: Fn(HttpRequest) -> R + Send + Sync + 'static>
    RawHttpHandlerTrait<R> for T
{
}
pub type HttpHandler = Box<dyn HttpHandlerTrait>;

//####### WS HANDLER
//
pub trait WebSocketHandlerResultTrait: Future<Output = ()> + Send + 'static {}
impl<T: Future<Output = ()> + Send + 'static> WebSocketHandlerResultTrait for T {}

pub trait WebSocketHandlerTarit:
    Fn(WebSocket, HttpRequest) -> Pin<Box<dyn WebSocketHandlerResultTrait>> + Send + Sync + 'static
{
}
impl<
    T: Fn(WebSocket, HttpRequest) -> Pin<Box<dyn WebSocketHandlerResultTrait>> + Send + Sync + 'static,
> WebSocketHandlerTarit for T
{
}

pub trait RawWebSocketHandlerTarit<R: WebSocketHandlerResultTrait>:
    Fn(WebSocket, HttpRequest) -> R + Send + Sync + 'static
{
}

impl<R: WebSocketHandlerResultTrait, T: Fn(WebSocket, HttpRequest) -> R + Send + Sync + 'static>
    RawWebSocketHandlerTarit<R> for T
{
}

pub type WebSocketHandler = Box<dyn WebSocketHandlerTarit>;
pub enum Handler {
    Http(HttpHandler),
    WbeSocket(WebSocketHandler),
}
