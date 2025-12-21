pub mod cookie;

use bytes::Bytes;
use http::{
    HeaderMap, HeaderValue, Response, StatusCode, header::{ CONTENT_LENGTH, IntoHeaderName}
};
use tokio::{fs::File, io::AsyncWriteExt, net::tcp::OwnedWriteHalf};

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
            .header("Connection", "keep-alive")
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

    pub async  fn write_line_header_bytes(&self,socket: &mut OwnedWriteHalf) -> Result<(),std::io::Error> {
        // line
        let line_bytes = format!("{:?} {}\r\n",self._innser.version(),self._innser.status());
        socket.write_all(line_bytes.as_bytes()).await?;
        // header
        for (k,v) in self._innser.headers().iter() {
            socket.write_all(k.as_str().as_bytes()).await?;
            socket.write_all(": ".as_bytes()).await?;
            socket.write_all(v.as_bytes()).await?;
            socket.write_all("\r\n".as_bytes()).await?;
        }
        socket.write_all("\r\n".as_bytes()).await?;

        Ok(())
    }
    pub async fn serialize_to_socket(mut self, socket: &mut OwnedWriteHalf) -> Result<(),std::io::Error> {
        use self::ResponseBody::*;

        self.write_line_header_bytes(socket).await?;

        match self._innser.body_mut() {
            Simple( b) => {
                socket.write_all_buf( b).await?;
            }
            File( f) => {
                tokio::io::copy( f, socket).await?;
            }
            Empty => {
                // No body to write
            }
        }
        socket.flush().await?;
        Ok(())
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
            for (k,v) in self.drain() {
                if let Some(k) = k {
                    res.add_header(k,v);
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
