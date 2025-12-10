use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio::{fs::File, io::AsyncWriteExt, net::tcp::OwnedWriteHalf};

use crate::HttpHeader;

#[derive(Default,Debug)]
pub struct HttpResponse {
    status_line: ResponseStatusLine,
    headers: HttpHeader,
    pub body: ResponseBody,
}
impl HttpResponse {
    pub fn new() -> Self {
        Self {
            status_line: ResponseStatusLine::new("HTTP/1.1", "200", "OK"),
            headers: Default::default(),
            body: ResponseBody::Empty,
        }
    }
    pub fn not_found() -> Self {
        let mut r = Self::new();
        r.status_line.status = "404".to_string();
        r.status_line.info = "Not Found".to_string();
        r.headers.add("Content-length", "9");
        r.set_body(ResponseBody::Simple("not found".into()));
        r
    }
    pub fn set_body(&mut self, body: ResponseBody) {
        self.body = body
    }
    pub fn add_header(&mut self, header_k_v: (&str, &str)) {
        self.headers.add(header_k_v.0, header_k_v.1);
    }
    pub fn line_header_bytes(&self) -> Bytes {
        let mut bytes = BytesMut::new();
        let line_bytes:Bytes = (&self.status_line).into();
        bytes.put(line_bytes);
        let header_bytes:Bytes = (&self.headers).into();
        bytes.put(header_bytes);
        bytes.freeze()
    }
    pub async fn serilize_to_socket(self,socket:&mut OwnedWriteHalf) {
        use self::ResponseBody::*;
        let mut line_header:Bytes = self.line_header_bytes();
        while line_header.has_remaining() {
            let _ = socket.write_buf(&mut line_header).await;
        }
        match self.body {
            Simple(mut b) => {
                    let _ = socket.write_all_buf(&mut b).await;
            }
            File(mut f) => {
                let _ = tokio::io::copy(&mut f, socket).await;
            }
            Empty => {
            }
        }
        let _ = socket.flush().await;
    }
}

#[derive(Default,Debug)]
pub enum ResponseBody {
    Simple(Bytes),
    File(File),
    #[default]
    Empty,
}

#[derive(Default,Debug)]
pub struct ResponseStatusLine {
    version: String,
    status: String,
    info: String,
}

impl ResponseStatusLine {
    pub fn new(version: &str, status: &str, info: &str) -> Self {
        Self {
            version: version.into(),
            status: status.into(),
            info: info.into(),
        }
    }
}

impl From<&ResponseStatusLine> for Bytes {
    fn from(value: &ResponseStatusLine) -> Self {
        let mut bytes = BytesMut::with_capacity(64);
        bytes.put(format!("{} {} {}\r\n", value.version, value.status, value.info).as_bytes());
        bytes.freeze()
    }
}

#[cfg(test)]
mod test {
    use bytes::{Buf, Bytes};

    use crate::response::{HttpResponse, ResponseStatusLine};

    #[test]
    fn into_bytes_test() {
        let r = ResponseStatusLine::new("Http", "200", "ok");
        let b:Bytes = (&r).into();
        assert_eq!(b"Http 200 ok\r\n",b.chunk());
    }

    #[test]
    fn line_header_test() {
        let mut r = HttpResponse::new();
        r.add_header(("Hello","World"));
        let b = r.line_header_bytes();
        println!("{:?}",b);
    }
}
