pub mod cookie;
pub mod cors;
use bytes::{Bytes, BytesMut};
use h2::{SendStream, server::SendResponse};
use http::{
    HeaderMap, HeaderValue, Response, StatusCode, header::{CONNECTION, CONTENT_LENGTH, IntoHeaderName}
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
};

use crate::handler::FuError;

#[derive(Default, Debug)]
pub struct HttpResponse {
    // status_line: ResponseStatusLine,
    // headers: HttpHeader,
    // pub body: ResponseBody,
    pub(crate) _innser: Response<ResponseBody>,
}
impl HttpResponse {
    pub fn new() -> Self {
        let _innser = Response::builder()
            // .version(Version::HTTP_2)
            .body(ResponseBody::Empty)
            .expect("impossible!!");
        Self { _innser }
    }

    pub fn not_found() -> Self {
        let mut r = Self::new();
        *r._innser.status_mut() = StatusCode::NOT_FOUND;

        // r.status_line.info = "Not Found".to_string();
        r._innser
            .headers_mut()
            .insert(CONTENT_LENGTH, HeaderValue::from_static("9"));
        r.set_body(ResponseBody::Simple("not found".into()));
        r
    }
    pub fn error(err_message: String) -> Self {
        let mut r = Self::new();
        *r._innser.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        let body: Bytes = err_message.into();
        r._innser.headers_mut().insert(
            CONTENT_LENGTH,
            HeaderValue::from_maybe_shared(body.len().to_string()).expect("impossible!"),
        );
        r.set_body(ResponseBody::Simple(body));
        r
    }
    pub fn set_body(&mut self, body: ResponseBody) {
        *self._innser.body_mut() = body
    }

    pub fn add_header<K: IntoHeaderName>(&mut self, key: K, value: HeaderValue) {
        self._innser.headers_mut().insert(key, value);
    }

    pub async fn write_line_header_bytes<W: AsyncWrite + Unpin>(
        &self,
        socket: &mut W,
    ) -> Result<(), std::io::Error> {
        // line
        let line_bytes = format!("{:?} {}\r\n", self._innser.version(), self._innser.status());
        socket.write_all(line_bytes.as_bytes()).await?;
        // header
        for (k, v) in self._innser.headers().iter() {
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
        self._innser.body_mut().serialize_to_h1_socket(socket).await
    }
    pub async fn serialize_to_socket_h2(
        self,
        respond: &mut SendResponse<Bytes>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut h, b) = self._innser.into_parts();
        h.headers.remove(CONTENT_LENGTH);
        h.headers.remove(CONNECTION);
        let body_stream =
            respond.send_response(Response::from_parts(h, ()), false)?;
        b.seriliaze_to_h2_stream(body_stream).await
    }
}

#[derive(Default, Debug)]
pub enum ResponseBody {
    /// In-memory byte data for small responses.
    Simple(Bytes),
    /// File handle for streaming large responses efficiently.
    File(File),
    /// No body content.
    #[default]
    Empty,
    // _WsBody
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
            Simple(b) => {
                body_stream.reserve_capacity(b.len());
                body_stream.send_data(b, true)?;
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
            Empty => {}
        }

        Ok(())
    }
    async fn serialize_to_h1_socket<W: AsyncWrite + Unpin>(
        &mut self,
        socket: &mut W,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use self::ResponseBody::*;
        match self {
            Simple(b) => {
                socket.write_all_buf(b).await?;
            }
            File(f) => {
                tokio::io::copy(f, socket).await?;
            }
            Empty => {
                // No body to write
            }
        }
        socket.flush().await?;
        Ok(())
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
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), FuError>> + 'a + Send + Sync>>;
}

impl HttpResponseModifier for HeaderMap {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), FuError>> + 'a + Send + Sync>> {
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
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), FuError>> + 'a + Send + Sync>> {
        Box::pin(async move {
            *res._innser.status_mut() = *self;
            Ok(())
        })
    }
}
impl<T: HttpResponseModifier + ?Sized + Send + Sync> HttpResponseModifier for Vec<Box<T>> {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), FuError>> + 'a + Send + Sync>> {
        Box::pin(async move {
            for m in self {
                let m: std::pin::Pin<Box<dyn Future<Output = Result<(), FuError>> + Send + Sync>> =
                    m.modify(res);
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
