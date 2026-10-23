use bytes::BytesMut;
use http::{
    Request, Response,
    header::{CONNECTION, CONTENT_LENGTH, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE},
};
use hyper::body::{Body, Incoming};

use crate::{
    error::BodyParseError,
    request::HttpRequest,
    response::ResponseBody,
    server::{BytesSource, HyperIncomingBytesSource, ServerContext},
    service::{guard_request, handle_request, handle_websocket},
};

pub async fn serve_http1(
    req: Request<Incoming>,
    context: ServerContext,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    if is_websocket_upgrade_hyper(&req) {
        handle_websocket(req, context.clone()).await
    } else {
        handle_http(req, context).await
    }
}
async fn handle_http(
    req: Request<Incoming>,
    context: ServerContext,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    let (parts, body) = req.into_parts();
    let has_body = !body.is_end_stream() || parts.headers.contains_key(CONTENT_LENGTH);
    let mut req = HttpRequest::new(parts, None);

    if has_body {
        let mut bs = HyperIncomingBytesSource::new(body);
        let mut buf = BytesMut::with_capacity(4096);
        let body = crate::request::parse_body_frame(&mut bs, &mut buf, req.inner.headers()).await?;
        // Multipart can finish before the HTTP body, including its trailers.
        // Consume the remainder so the connection can serve the next request.
        while !bs.is_end() {
            buf.clear();
            bs.read_bytes(&mut buf)
                .await
                .map_err(BodyParseError::from)?;
        }
        *req.inner.body_mut() = Some(body);
    }

    match guard_request(context.guards().clone(), req).await {
        Ok(req) => handle_request(context.handlers(), req, context.error_handler()).await,
        Err(res) => Ok(res.inner),
    }
}

fn is_websocket_upgrade_hyper(req: &Request<Incoming>) -> bool {
    req.headers().get(UPGRADE).is_some()
        && req.headers().get(CONNECTION).is_some()
        && req.headers().get(SEC_WEBSOCKET_KEY).is_some()
        && req.headers().get(SEC_WEBSOCKET_VERSION).is_some()
}
