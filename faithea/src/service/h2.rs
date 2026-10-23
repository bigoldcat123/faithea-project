use bytes::BytesMut;
use http::{Method, Request, Response};
use hyper::body::Incoming;

use crate::{
    request::HttpRequest,
    response::ResponseBody,
    server::{HyperIncomingBytesSource, ServerContext},
    service::{guard_request, handle_request, handle_websocket},
};

pub async fn serve_http2(
    req: Request<Incoming>,
    context: ServerContext,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    if is_websocket_upgrade_hyper(&req) {
        handle_websocket(req, context).await
    } else {
        handle_http(req, context).await
    }
}

async fn handle_http(
    req: Request<Incoming>,
    context: ServerContext,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    let (p, incoming) = req.into_parts();
    let bs = HyperIncomingBytesSource::new(incoming);
    let mut buf = BytesMut::with_capacity(4096);
    let body = crate::request::parse_body_frame(bs, &mut buf, &p.headers).await?;
    let req = HttpRequest::new(p, Some(body));
    match guard_request(context.guards().clone(), req).await {
        Ok(req) => handle_request(context.handlers(), req, context.error_handler()).await,
        Err(res) => Ok(res.inner),
    }
}

fn is_websocket_upgrade_hyper(req: &Request<Incoming>) -> bool {
    req.method() == Method::CONNECT
}
