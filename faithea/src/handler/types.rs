use std::pin::Pin;

use crate::{request::HttpRequest, response::{HttpResponse, HttpResponseModifier}, websocket::socket::WebSocket};


pub trait HttphandlerErrorTrait: HttpResponseModifier + Send + Sync {}
impl<T: HttpResponseModifier + Send + Sync> HttphandlerErrorTrait for T {}

pub type HttpHandlerError = Box<dyn HttphandlerErrorTrait>;

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

pub type WebSocketHandler = Box<
    dyn Fn(WebSocket, HttpRequest) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Send
        + Sync
        + 'static,
>;
pub enum Handler {
    Http(HttpHandler),
    WbeSocket(WebSocketHandler),
}
