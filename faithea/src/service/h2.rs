use bytes::BytesMut;
use http::{Method, Request, Response};
use hyper::{body::Incoming, upgrade::Upgraded};
use tokio::io::{AsyncWriteExt, split};

use crate::{
    handler::types::Handler,
    io::TokioIo,
    request::HttpRequest,
    response::{HttpResponse, ResponseBody},
    route::Route,
    server::{Http1BytesSource, HyperIncommingBytesSource, ServerFuncProvider, guard_request},
    service::handle_request,
    websocket::{WebSocketIncommingMessageParser, data::WebSocketDataPayLoad, socket::WebSocket},
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

async fn handle_websocket(
    req: Request<Incoming>,
    provider: ServerFuncProvider,
) -> Result<Response<ResponseBody>, crate::error::Error> {
    let (parts, body) = req.into_parts();
    let req2 = HttpRequest::new(parts.clone(), None);
    let http_req = HttpRequest::new(parts, None);

    let response = HttpResponse::new();
    let mut req = match guard_request(provider.guards(), http_req).await {
        Ok(req) => Request::from_parts(req._inner.into_parts().0, body),
        Err(e) => {
            return Ok(e._innser);
        }
    };
    tokio::task::spawn(async move {
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => {
                if let Err(e) = server_upgraded_io(upgraded, req2, provider).await {};
            }
            Err(e) => eprintln!("upgrade error: {}", e),
        }
    });
    return Ok(response._innser);
}

async fn server_upgraded_io(
    upgrade: Upgraded,
    mut req: HttpRequest,
    provider: ServerFuncProvider,
) -> Result<(), ()> {
    let upgraded = TokioIo::new(upgrade);
    let (read, mut write) = split(upgraded);

    let (outcomming_message_sender, mut outcomming_message_receiver) =
        tokio::sync::mpsc::channel::<WebSocketDataPayLoad>(16);

    tokio::spawn(async move {
        while let Some(mut ws_msg) = outcomming_message_receiver.recv().await {
            let mut head = ws_msg.generate_head_frame();
            let _ = write.write_all_buf(&mut head).await;
            let _ = write.write_all_buf(&mut ws_msg._inner).await;
        }
    });

    let (parser, incomming_message_receiver) = WebSocketIncommingMessageParser::new(
        Http1BytesSource::new(read, 0, 0),
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
            Handler::Http(_) => {
                unreachable!()
            }
            Handler::WbeSocket(ws_handler) => {
                ws_handler(websocket, req).await;
            }
        }
    }

    Ok(())
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
        Err(res) => Ok(res._innser),
    }
}

fn is_websocket_upgrade_hyper(req: &Request<Incoming>) -> bool {
    req.method() == Method::CONNECT
}
