use bytes::BytesMut;
use http::{Method, Request, Response};
use hyper::body::Incoming;

use crate::{
    request::HttpRequest,
    response::ResponseBody,
    server::{HyperIncommingBytesSource, ServerFuncProvider, guard_request},
    service::{handle_request, handle_websocket},
};

pub async fn serve_http2(
    req: Request<Incoming>,
    provider: ServerFuncProvider,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    if is_websocket_upgrade_hyper(&req) {
        handle_websocket(req, provider).await
    } else {
        handle_http(req, provider).await
    }
}

async fn handle_http(
    req: Request<Incoming>,
    provider: ServerFuncProvider,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    let (p, incomming) = req.into_parts();
    let bs = HyperIncommingBytesSource::new(incomming);
    let mut buf = BytesMut::with_capacity(4096);
    let body = crate::request::parse_body_frame(bs, &mut buf, &p.headers)
        .await
        .unwrap();
    let req = HttpRequest::new(p, Some(body));
    match guard_request(provider.guards().clone(), req).await {
        Ok(req) => handle_request(provider.handlers(), req, provider.error_handler()).await,
        Err(res) => Ok(res._inner),
    }
}

fn is_websocket_upgrade_hyper(req: &Request<Incoming>) -> bool {
    req.method() == Method::CONNECT
}
