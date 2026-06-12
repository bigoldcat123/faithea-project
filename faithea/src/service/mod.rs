use std::{marker::PhantomData, sync::Arc, task::Poll};

use faithea_websocket::{WebSocket, WebSocketDataPayLoad, WebSocketIncommingMessageParser};
use http::{Request, Response};
use hyper::{
    body::{Body, Incoming},
    upgrade::Upgraded,
};
use tokio::io::{AsyncWriteExt, split};

use crate::{
    guard::GuardTire,
    handler::{
        HandlerTire,
        types::{Handler, HttpHandler},
    },
    io::TokioIo,
    request::HttpRequest,
    response::{HttpResponse, HttpResponseModifier, ResponseBody},
    route::Route,
    server::{Http1BytesSource, ServerFuncProvider, builder::GlobalErrorHandler},
};

pub(crate) mod h1;
pub(crate) mod h2;

pub(crate) async fn guard_request(
    guards: Arc<GuardTire>,
    req: HttpRequest,
) -> Result<HttpRequest, HttpResponse> {
    guards
        .guard(&req._inner.uri().path().to_string()[..], req)
        .await
}

pub async fn handle_websocket(
    req: Request<Incoming>,
    provider: ServerFuncProvider,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    let (parts, body) = req.into_parts();
    let req2 = HttpRequest::new(parts.clone(), None);
    let http_req = HttpRequest::new(parts, None);

    let response = HttpResponse::websocket_response(&http_req);
    let mut req = match guard_request(provider.guards(), http_req).await {
        Ok(req) => Request::from_parts(req._inner.into_parts().0, body),
        Err(e) => {
            return Ok(e._inner);
        }
    };
    tokio::task::spawn(async move {
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => server_upgraded_io(upgraded, req2, provider).await,
            Err(e) => log::error!("upgrade error: {}", e),
        };
    });
    Ok(response._inner)
}
async fn server_upgraded_io(upgrade: Upgraded, mut req: HttpRequest, provider: ServerFuncProvider) {
    let upgraded = TokioIo::new(upgrade);
    let (read, mut write) = split(upgraded);

    let (outcomming_message_sender, mut outcomming_message_receiver) =
        tokio::sync::mpsc::channel::<WebSocketDataPayLoad>(16);

    tokio::spawn(async move {
        while let Some(ws_msg) = outcomming_message_receiver.recv().await {
            let mut frame = ws_msg.into_frame_bytes();
            let _ = write.write_all_buf(&mut frame).await;
            let _ = write.flush().await;
        }
    });

    let (parser, incomming_message_receiver) = WebSocketIncommingMessageParser::new(
        Http1BytesSource::stream(read),
        outcomming_message_sender.clone(),
    );
    parser.start();
    let websocket = WebSocket::new(outcomming_message_sender, incomming_message_receiver);

    if let Some((_matched_url, handler)) = provider
        .handlers()
        .get_handler(req._inner.uri().path(), req._inner.method().clone())
    {
        req.process_routes(&_matched_url, &Route::from(req._inner.uri().path()));

        req.process_search_param();
        match handler {
            Handler::WbeSocket(ws_handler) => {
                ws_handler(websocket, req).await;
            }
            _ => unreachable!(),
        }
    }
}

async fn handle_request(
    handlers: Arc<HandlerTire>,
    mut req: HttpRequest,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    use crate::handler::types::Handler;
    if let Some((_matched_url, handler)) =
        handlers.get_handler(req._inner.uri().path(), req._inner.method().clone())
    {
        req.process_routes(&_matched_url, &Route::from(req._inner.uri().path()));

        req.process_search_param();
        match handler {
            Handler::Http(http_handler) => {
                process_http_request(&http_handler, req, error_handler).await
            }
            _ => unreachable!(),
        }
    } else {
        Ok(HttpResponse::not_found()._inner)
    }
}
async fn process_http_request(
    http_handler: &HttpHandler,
    req: HttpRequest,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    match http_handler(req).await {
        Ok(res) => Ok(res._inner),
        Err(mut err) => {
            let mut response = HttpResponse::new();
            if let Some(error_handler) = error_handler {
                let mut m = error_handler(err).await;
                if m.modify(&mut response).await.is_ok() {
                    Ok(response._inner)
                } else {
                    Ok(HttpResponse::not_found()._inner)
                }
            } else if err.modify(&mut response).await.is_ok() {
                Ok(response._inner)
            } else {
                Ok(HttpResponse::not_found()._inner)
            }
        }
    }
}

pub fn my_service_fn<F, R, S>(f: F, provider: ServerFuncProvider) -> MyServiceFn<F, R>
where
    F: Fn(Request<R>, ServerFuncProvider) -> S + Clone,
    S: Future,
{
    MyServiceFn {
        f: Arc::new(f),
        provider,
        _req: PhantomData,
    }
}
/// Service returned by [`service_fn`]
pub struct MyServiceFn<F, R> {
    f: Arc<F>,
    provider: ServerFuncProvider,
    _req: PhantomData<fn(R)>,
}
impl<F, R> Clone for MyServiceFn<F, R> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            provider: self.provider.clone(),
            _req: self._req,
        }
    }
}

impl<F, ReqBody, Ret, ResBody, E> tower::Service<Request<ReqBody>> for MyServiceFn<F, ReqBody>
where
    F: Fn(Request<ReqBody>, ServerFuncProvider) -> Ret + Clone,
    ReqBody: Body,
    Ret: Future<Output = Result<Response<ResBody>, E>> + Send,
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
    ResBody: Body,
{
    type Response = Response<ResBody>;
    type Error = E;
    type Future = Ret;
    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        (self.f)(req, self.provider.clone())
    }
    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
