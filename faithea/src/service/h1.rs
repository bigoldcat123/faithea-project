use bytes::BytesMut;
use http::{
    Request, Response,
    header::{CONNECTION, CONTENT_LENGTH, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE},
};
use hyper::body::Incoming;

use crate::{
    error::BodyParseError,
    request::{HttpRequest},
    response::ResponseBody,
    server::{HyperIncommingBytesSource, ServerFuncProvider},
    service::{guard_request, handle_request, handle_websocket},
};

pub async fn serve_http1(
    req: Request<Incoming>,
    provider: ServerFuncProvider,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    if is_websocket_upgrade_hyper(&req) {
        handle_websocket(req, provider.clone()).await
    } else {
        handle_http(req, provider).await
    }
}
async fn handle_http(
    req: Request<Incoming>,
    provider: ServerFuncProvider,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    let (parts, body) = req.into_parts();
    let bs = HyperIncommingBytesSource::new(body);
    let mut buf = BytesMut::with_capacity(4096);

    let mut req = HttpRequest::new(parts, None);

    if req.get_header(CONTENT_LENGTH).is_some() {
        let body = crate::request::parse_body_frame(bs, &mut buf, req._inner.headers())
            .await
            .map_err(|e| BodyParseError::Other(e))?;
        *req._inner.body_mut() = Some(body);
    }

    match guard_request(provider.guards().clone(), req).await {
        Ok(req) => handle_request(provider.handlers(), req, provider.error_handler()).await,
        Err(res) => Ok(res._inner),
    }
}

fn is_websocket_upgrade_hyper(req: &Request<Incoming>) -> bool {
    req.headers().get(UPGRADE).is_some()
        && req.headers().get(CONNECTION).is_some()
        && req.headers().get(SEC_WEBSOCKET_KEY).is_some()
        && req.headers().get(SEC_WEBSOCKET_VERSION).is_some()
}
