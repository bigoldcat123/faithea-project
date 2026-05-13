pub mod cookie;
pub mod cors;
pub mod redirect;
pub mod sse;
pub mod stream;
use std::{pin::pin, task::Poll};

use base64::{Engine, prelude::BASE64_STANDARD};
use bytes::{BufMut, Bytes, BytesMut};
use h2::{SendStream, server::SendResponse};
use http::{
    HeaderMap, HeaderValue, Method, Response, StatusCode,
    header::{
        CONNECTION, CONTENT_LENGTH, IntoHeaderName, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY,
        UPGRADE,
    },
};
use hyper::body::{Body, Frame};
use sha1::{Digest, Sha1};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc::Receiver,
};

use crate::{
    handler::types::HttpHandlerError, request::HttpRequest, websocket::data::WebSocketDataPayLoad,
};

#[derive(Default, Debug)]
pub struct HttpResponse {
    // status_line: ResponseStatusLine,
    // headers: HttpHeader,
    // pub body: ResponseBody,
    pub(crate) _inner: Response<ResponseBody>,
}
impl HttpResponse {
    pub fn new() -> Self {
        let _innser = Response::builder()
            // .version(Version::HTTP_2)
            .body(ResponseBody::Simple(None))
            .expect("impossible!!");
        Self { _inner: _innser }
    }
    pub fn websocket_response(req: &HttpRequest) -> Self {
        if req._inner.method() == Method::CONNECT {
            HttpResponse::new()
        } else {
            let mut res = Self::new();
            *res._inner.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
            let upgrade = req.get_header(UPGRADE).unwrap();
            res._inner.headers_mut().insert(UPGRADE, upgrade.clone());
            res._inner
                .headers_mut()
                .insert(CONNECTION, "Upgrade".parse().unwrap());
            let key = req.get_header(SEC_WEBSOCKET_KEY).unwrap().to_str().unwrap();
            let accept = Self::cal_sec_websocket_accept(key);
            res._inner
                .headers_mut()
                .insert(SEC_WEBSOCKET_ACCEPT, accept.parse().unwrap());
            res
        }
    }
    fn cal_sec_websocket_accept(key: &str) -> String {
        let mut hasher = Sha1::new();
        let accept = format!("{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", key);
        hasher.update(accept.as_bytes());
        let result = hasher.finalize();
        BASE64_STANDARD.encode(result)
    }

    pub fn not_found() -> Self {
        let mut r = Self::new();
        *r._inner.status_mut() = StatusCode::NOT_FOUND;

        // r.status_line.info = "Not Found".to_string();
        r._inner
            .headers_mut()
            .insert(CONTENT_LENGTH, HeaderValue::from_static("9"));
        r.set_body(ResponseBody::Simple(Some("not found".into())));
        r
    }
    pub fn error(err_message: String) -> Self {
        let mut r = Self::new();
        *r._inner.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        let body: Bytes = err_message.into();
        r._inner.headers_mut().insert(
            CONTENT_LENGTH,
            HeaderValue::from_maybe_shared(body.len().to_string()).expect("impossible!"),
        );
        r.set_body(ResponseBody::Simple(Some(body)));
        r
    }
    pub fn set_body(&mut self, body: ResponseBody) {
        *self._inner.body_mut() = body
    }

    pub fn add_header<K: IntoHeaderName>(&mut self, key: K, value: HeaderValue) {
        self._inner.headers_mut().insert(key, value);
    }

    pub async fn write_line_header_bytes<W: AsyncWrite + Unpin>(
        &self,
        socket: &mut W,
    ) -> Result<(), std::io::Error> {
        // line
        let line_bytes = format!("{:?} {}\r\n", self._inner.version(), self._inner.status());
        socket.write_all(line_bytes.as_bytes()).await?;
        // header
        for (k, v) in self._inner.headers().iter() {
            socket.write_all(k.as_str().as_bytes()).await?;
            socket.write_all(": ".as_bytes()).await?;
            socket.write_all(v.as_bytes()).await?;
            socket.write_all("\r\n".as_bytes()).await?;
        }
        socket.write_all("\r\n".as_bytes()).await?;

        Ok(())
    }
    pub async fn serialize_to_socket_h1<W: AsyncWrite + Unpin>(
        mut self,
        socket: &mut W,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.write_line_header_bytes(socket).await?;
        self._inner.body_mut().serialize_to_h1_socket(socket).await
    }
    pub async fn serialize_to_socket_h2(
        self,
        respond: &mut SendResponse<Bytes>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut h, b) = self._inner.into_parts();
        h.headers.remove(CONTENT_LENGTH);
        h.headers.remove(CONNECTION);
        let body_stream = respond.send_response(Response::from_parts(h, ()), false)?;
        b.seriliaze_to_h2_stream(body_stream).await
    }
}

#[derive(Default, Debug)]
pub enum ResponseBody {
    /// In-memory byte data for small responses.
    Simple(Option<Bytes>),
    /// File handle for streaming large responses efficiently.
    File(File),
    /// No body content.
    #[default]
    #[deprecated]
    /// use Simple(None) instead
    Empty,
    WsBody(Receiver<WebSocketDataPayLoad>),
    Stream(Receiver<Bytes>),
}

impl ResponseBody {
    // fn is_empty_body(&self) -> bool {
    //     if let ResponseBody::Empty = self {
    //         true
    //     } else {
    //         false
    //     }
    // }

    async fn seriliaze_to_h2_stream(
        self,
        mut body_stream: SendStream<Bytes>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use ResponseBody::*;
        match self {
            Simple(Some(b)) => {
                body_stream.reserve_capacity(b.len());
                body_stream.send_data(b, true)?;
            }
            Stream(mut r) => {
                while let Some(r) = r.recv().await {
                    body_stream.reserve_capacity(r.len());
                    body_stream.send_data(r, false)?;
                }
                body_stream.send_data(Bytes::new(), true)?;
            }
            File(mut f) => {
                let mut buf = BytesMut::with_capacity(4096);
                while let Ok(n) = f.read_buf(&mut buf).await {
                    if n == 0 {
                        body_stream.send_data(buf.freeze(), true)?;
                        break;
                    }
                    body_stream.reserve_capacity(n);
                    body_stream.send_data(buf.split_to(n).freeze(), false)?;
                }
            }

            WsBody(mut receiver) => {
                tokio::spawn(async move {
                    while let Some(b) = receiver.recv().await {
                        let _ = b.serialize_to_stream(&mut body_stream).await;
                    }
                });
            }
            _ => {}
        }

        Ok(())
    }
    async fn serialize_to_h1_socket<W: AsyncWrite + Unpin>(
        &mut self,
        socket: &mut W,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use self::ResponseBody::*;
        match self {
            Simple(Some(b)) => {
                socket.write_all_buf(b).await?;
            }
            File(f) => {
                tokio::io::copy(f, socket).await?;
            }

            Stream(r) => {
                while let Some(mut r) = r.recv().await {
                    socket.write_all_buf(&mut r).await?;
                }
            }
            WsBody(receiver) => {
                while let Some(_payload) = receiver.recv().await {
                    _payload.serialize_to_socket(socket).await?;
                }
            }
            _ => {}
        }
        socket.flush().await?;
        Ok(())
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = crate::error::Error;
    // fn is_end_stream(&self) -> bool {

    // }
    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        match *self {
            ResponseBody::Simple(ref mut bytes) => {
                if let Some(bytes) = bytes.take() {
                    Poll::Ready(Some(Ok(Frame::data(bytes))))
                } else {
                    Poll::Ready(None)
                }
            }
            ResponseBody::File(ref mut file) => {
                let mut buf = BytesMut::with_capacity(2048);
                pin!(file.read_buf(&mut buf)).poll(cx).map(|res| {
                    if let Ok(size) = res {
                        if size != 0 {
                            Some(Ok(Frame::data(buf.freeze())))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            }
            ResponseBody::Stream(ref mut steam) => match steam.poll_recv(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(data) => Poll::Ready(data.map(|data| Ok(Frame::data(data)))),
            },
            ResponseBody::WsBody(ref mut steam) => {
                match steam.poll_recv(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(data) => {
                        if let Some(data) = data {
                            let head = data.generate_head_frame();
                            let body = data._inner;
                            let mut res = BytesMut::with_capacity(head.len() + body.len());
                            res.put(head);
                            res.put(body);
                            let combined: Bytes = res.freeze(); // 变成不可变的 Bytes
                            Poll::Ready(Some(Ok(Frame::data(combined))))
                        } else {
                            Poll::Ready(None)
                        }
                    }
                }
            }
            _ => std::task::Poll::Ready(None),
        }
    }
}

// impl From<&ResponseStatusLine> for Bytes {
//     fn from(value: &ResponseStatusLine) -> Self {
//         let mut bytes = BytesMut::with_capacity(64);
//         bytes.put(format!("{} {} {}\r\n", value.version, value.status, value.info).as_bytes());
//         bytes.freeze()
//     }
// }

pub trait HttpResponseModifier {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), HttpHandlerError>> + 'a + Send + Sync>>;
}

impl HttpResponseModifier for HeaderMap {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), HttpHandlerError>> + 'a + Send + Sync>>
    {
        Box::pin(async move {
            for (k, v) in self.drain() {
                if let Some(k) = k {
                    res.add_header(k, v);
                }
            }
            Ok(())
        })
    }
}
impl HttpResponseModifier for StatusCode {
    // fn modify(&self, res: &mut HttpResponse) -> Result<(), String> {
    //     res.status_line.info = self.status.to_string();
    //     res.status_line.info = self.info.to_string();
    //     Ok(())
    // }
    fn modify<'a>(
        &'a mut self,
        res: &'a mut HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), HttpHandlerError>> + 'a + Send + Sync>>
    {
        Box::pin(async move {
            *res._inner.status_mut() = *self;
            Ok(())
        })
    }
}
impl<T: HttpResponseModifier + ?Sized + Send + Sync> HttpResponseModifier for Vec<Box<T>> {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), HttpHandlerError>> + 'a + Send + Sync>>
    {
        Box::pin(async move {
            for m in self {
                let m: std::pin::Pin<
                    Box<dyn Future<Output = Result<(), HttpHandlerError>> + Send + Sync>,
                > = m.modify(res);
                m.await?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod test {

    // #[test]
    // fn fuck() {
    //     trait FutureT {
    //         fn future_fn<'a>(&'a self) -> std::pin::Pin<Box<dyn Future<Output = String> + 'a>>;
    //     }
    //     let a: Vec<Box<dyn FutureT>> = vec![];
    //     for a in a{
    //         a.future_fn();
    //     }
    //     struct A {
    //         name: String,
    //     }
    //     impl FutureT for A {
    //         fn future_fn<'a>(&'a self) -> std::pin::Pin<Box<dyn Future<Output = String> + 'a>> {

    //             Box::pin(async move {
    //                 let a = &self.name;
    //                 "a".to_string()
    //             })
    //         }
    //     }
    //     fn hello() -> impl FutureT {
    //         A {name:"".to_string()}
    //     }
    // }
}
