use std::{
    error::Error,
    marker::PhantomData,
    pin::pin,
    sync::{Arc, mpsc::Sender},
};

use bytes::BytesMut;
use http::{Request, Response, header::CONTENT_LENGTH};
use hyper::{
    body::{Body, Incoming},
    service::Service,
};

use crate::{
    data::outbound::StaticFile,
    handler::{HandlerTire, types::HttpHandler},
    request::HttpRequest,
    response::{HttpResponse, HttpResponseModifier, ResponseBody},
    route::Route,
    server::{
        HyperIncommingBytesSource, ServerFuncProvider, builder::GlobalErrorHandler, guard_request,
    },
};

pub async fn serve(
    req: Request<Incoming>,
    provider: ServerFuncProvider,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    let (parts, body) = req.into_parts();

    let bs = HyperIncommingBytesSource::new(body);
    let mut buf = BytesMut::with_capacity(4096);

    let mut req = HttpRequest::new(parts, None);

    if let Some(_) = req.get_header(CONTENT_LENGTH) {
        let body = crate::request::parse_body_frame(bs, &mut buf, req._inner.headers())
            .await
            .map_err(|e| crate::error::Error::before_handler_invalid_param(e))?;
        *req._inner.body_mut() = Some(body);
    }

    match guard_request(provider.guards().clone(), req).await {
        Ok(req) => handle_request(provider.handlers(), req, provider.error_handler()).await,
        Err(res) => Ok(res._innser),
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
                process_http_request(http_handler, req, error_handler).await
            }
            Handler::WbeSocket(ws_handler) => unimplemented!(),
        }
    } else {
        Ok(HttpResponse::not_found()._innser)
    }
}
async fn process_http_request(
    http_handler: &HttpHandler,
    req: HttpRequest,
    error_handler: Option<Arc<GlobalErrorHandler>>,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    match http_handler(req).await {
        Ok(res) => Ok(res._innser),
        Err(mut err) => {
            let mut response = HttpResponse::new();
            if let Some(error_handler) = error_handler {
                let mut m = error_handler(err).await;
                if m.modify(&mut response).await.is_ok() {
                    Ok(response._innser)
                } else {
                    Ok(HttpResponse::not_found()._innser)
                }
            } else if err.modify(&mut response).await.is_ok() {
                Ok(response._innser)
            } else {
                Ok(HttpResponse::not_found()._innser)
            }
        }
    }
}

pub fn my_service_fn<F, R, S>(f: F, provider: ServerFuncProvider) -> MyServiceFn<F, R>
where
    F: Fn(Request<R>, ServerFuncProvider) -> S,
    S: Future,
{
    MyServiceFn {
        f,
        provider,
        _req: PhantomData,
    }
}
/// Service returned by [`service_fn`]
pub struct MyServiceFn<F, R> {
    f: F,
    provider: ServerFuncProvider,
    _req: PhantomData<fn(R)>,
}

impl<F, ReqBody, Ret, ResBody, E> Service<Request<ReqBody>> for MyServiceFn<F, ReqBody>
where
    F: Fn(Request<ReqBody>, ServerFuncProvider) -> Ret,
    ReqBody: Body,
    Ret: Future<Output = Result<Response<ResBody>, E>>,
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
    ResBody: Body,
{
    type Response = Response<ResBody>;
    type Error = E;
    type Future = Ret;

    fn call(&self, req: Request<ReqBody>) -> Self::Future {
        (self.f)(req, self.provider.clone())
    }
}
