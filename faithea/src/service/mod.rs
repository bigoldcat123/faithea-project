use std::{marker::PhantomData, sync::Arc, task::Poll};

use faithea_websocket::{WebSocket, WebSocketIncomingMessageParser, WebSocketPayload};
use http::{Request, Response};
use hyper::{
    body::{Body, Incoming},
    upgrade::Upgraded,
};
use tokio::io::{AsyncWriteExt, split};

use crate::{
    guard::GuardTrie,
    handler::{
        HandlerTrie,
        types::{Handler, HttpHandler},
    },
    io::TokioIo,
    request::HttpRequest,
    response::{HttpResponse, HttpResponseModifier, ResponseBody},
    route::Route,
    server::{Http1BytesSource, ServerContext, builder::GlobalErrorHandler},
};

pub(crate) mod h1;
pub(crate) mod h2;

pub(crate) async fn guard_request(
    guards: Arc<GuardTrie>,
    req: HttpRequest,
) -> Result<HttpRequest, HttpResponse> {
    guards
        .guard(&req.inner.uri().path().to_string()[..], req)
        .await
}

pub async fn handle_websocket(
    req: Request<Incoming>,
    context: ServerContext,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    let (parts, body) = req.into_parts();
    let req2 = HttpRequest::new(parts.clone(), None);
    let http_req = HttpRequest::new(parts, None);

    let response = HttpResponse::websocket_response(&http_req);
    let mut req = match guard_request(context.guards(), http_req).await {
        Ok(req) => Request::from_parts(req.inner.into_parts().0, body),
        Err(e) => {
            return Ok(e.inner);
        }
    };
    tokio::task::spawn(async move {
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => serve_upgraded_websocket(upgraded, req2, context).await,
            Err(e) => log::error!("upgrade error: {}", e),
        };
    });
    Ok(response.inner)
}
async fn serve_upgraded_websocket(upgrade: Upgraded, mut req: HttpRequest, context: ServerContext) {
    let upgraded = TokioIo::new(upgrade);
    let (read, mut write) = split(upgraded);

    let (outgoing_message_sender, mut outgoing_message_receiver) =
        tokio::sync::mpsc::channel::<WebSocketPayload>(16);

    tokio::spawn(async move {
        while let Some(ws_msg) = outgoing_message_receiver.recv().await {
            let mut frame = ws_msg.into_frame_bytes();
            let _ = write.write_all_buf(&mut frame).await;
            let _ = write.flush().await;
        }
    });

    let (parser, incoming_message_receiver) = WebSocketIncomingMessageParser::new(
        Http1BytesSource::stream(read),
        outgoing_message_sender.clone(),
    );
    parser.start();
    let websocket = WebSocket::new(outgoing_message_sender, incoming_message_receiver);

    if let Some((_matched_url, handler)) = context
        .handlers()
        .get_handler(req.inner.uri().path(), req.inner.method().clone())
    {
        req.process_routes(&_matched_url, &Route::from(req.inner.uri().path()));

        req.process_search_param();
        match handler {
            Handler::WebSocket(ws_handler) => {
                ws_handler(websocket, req).await;
            }
            _ => unreachable!(),
        }
    }
}

async fn handle_request(
    handlers: Arc<HandlerTrie>,
    mut req: HttpRequest,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    use crate::handler::types::Handler;
    if let Some((_matched_url, handler)) =
        handlers.get_handler(req.inner.uri().path(), req.inner.method().clone())
    {
        req.process_routes(&_matched_url, &Route::from(req.inner.uri().path()));

        req.process_search_param();
        match handler {
            Handler::Http(http_handler) => {
                process_http_request(http_handler, req, error_handler).await
            }
            _ => unreachable!(),
        }
    } else {
        Ok(HttpResponse::not_found().inner)
    }
}
async fn process_http_request(
    http_handler: &HttpHandler,
    req: HttpRequest,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    match http_handler(req).await {
        Ok(res) => Ok(res.inner),
        Err(mut err) => {
            let mut response = HttpResponse::new();
            if let Some(error_handler) = error_handler {
                let mut m = error_handler(err).await;
                if m.modify(&mut response).await.is_ok() {
                    Ok(response.inner)
                } else {
                    Ok(HttpResponse::not_found().inner)
                }
            } else if err.modify(&mut response).await.is_ok() {
                Ok(response.inner)
            } else {
                Ok(HttpResponse::not_found().inner)
            }
        }
    }
}

pub fn server_service<F, R, S>(f: F, context: ServerContext) -> ServerService<F, R>
where
    F: Fn(Request<R>, ServerContext) -> S + Clone,
    S: Future,
{
    ServerService {
        f: Arc::new(f),
        context,
        _request_body: PhantomData,
    }
}
/// Service returned by [`server_service`].
pub struct ServerService<F, R> {
    f: Arc<F>,
    context: ServerContext,
    _request_body: PhantomData<fn(R)>,
}
impl<F, R> Clone for ServerService<F, R> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            context: self.context.clone(),
            _request_body: self._request_body,
        }
    }
}

impl<F, ReqBody, Ret, ResBody, E> tower::Service<Request<ReqBody>> for ServerService<F, ReqBody>
where
    F: Fn(Request<ReqBody>, ServerContext) -> Ret + Clone,
    ReqBody: Body,
    Ret: Future<Output = Result<Response<ResBody>, E>> + Send,
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
    ResBody: Body,
{
    type Response = Response<ResBody>;
    type Error = E;
    type Future = Ret;
    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        (self.f)(req, self.context.clone())
    }
    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
