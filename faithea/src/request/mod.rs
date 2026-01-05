pub mod content_type;
pub mod cookie;
// pub mod method;
pub mod path_param;
pub mod search_param;
use std::{fmt::Debug, path::PathBuf, str::FromStr};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use h2::RecvStream;
use http::{
    HeaderMap, HeaderName, HeaderValue, Method, Request, Uri, Version,
    header::{AsHeaderName, CONNECTION, CONTENT_LENGTH, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE},
};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    TryConvertFrom,
    data::inbound::multipart::{
        MultipartDataMap,
        parser::{h1::MultiPartBodyParser, h2::H2MultiPartBodyParser},
    },
    handler::types::HttpHandlerError,
    map_str,
    request::{
        content_type::ContentType, cookie::Cookie, path_param::PathParam, search_param::SearchParam,
    },
    route::{Route, RouteComponent},
};

pub enum RequestBody {
    Simple(Bytes),
    MultiPart(MultipartDataMap),
    Stream(PathBuf), // the path to a file saved on the disk
    WebSocketStreamBody(RecvStream),
    WebSocketStreamBodyHttp1(Box<dyn AsyncRead + Send + Sync + 'static + Unpin>),

}
impl Debug for RequestBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Simple(_) => write!(f, "Simple(Bytes)")?,
            Self::MultiPart(_) => write!(f, "MultiPart(MultipartDataMap)")?,
            Self::Stream(_) => write!(f, "Stream(PathBuf)")?,
            Self::WebSocketStreamBody(_) => write!(f, "WebSocketStreamBody(RecvStream)")?,
            Self::WebSocketStreamBodyHttp1(_) => write!(f, "WebSocketStreamBodyHttp1(Box<dyn AsyncRead + Send + Sync + 'static>)")?,
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct HttpRequest {
    pub(crate) _inner: Request<Option<RequestBody>>, // way too big!!!!!!!!!!!!
    pub(crate) path_param: Option<PathParam>,
    pub(crate) search_param: Option<SearchParam>,
    pub(crate) multi_seg_param: Option<String>,
    // pub(crate) cookie:Option<Cookie<'a>>
}

impl HttpRequest {
    pub fn new(parts: http::request::Parts, body: Option<RequestBody>) -> Self {
        Self {
            _inner: Request::from_parts(parts, body),
            path_param: None,
            multi_seg_param: None,
            search_param: None,
            // cookie:None
        }
    }
    pub fn from_req(req: Request<Option<RequestBody>>) -> Self {
        Self {
            _inner: req,
            path_param: None,
            multi_seg_param: None,
            search_param: None,
            // cookie:None
        }
    }
    //#[allow(unused)]
    pub fn cookies<'a>(&'a self) -> Option<Cookie<'a>> {
        if let Some(cookie) = self._inner.headers().get("cookie") {
            if let Ok(cookie) = cookie.to_str() {
                Some(Cookie::from_cookie_header(cookie))
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn fake() -> Self {
        let body = Some(RequestBody::Simple(Bytes::from("request payload")));
        Self {
            _inner: Request::builder()
                .method(http::Method::POST)
                .version(http::Version::HTTP_11)
                .uri("/api/data")
                .body(body)
                .unwrap(),
            path_param: None,
            search_param: None,
            multi_seg_param: None,
        }
    }
    fn assamble_path_param(&mut self, handler_route: &Route, incoming_route: &Route) {
        if let Ok(p) = PathParam::try_from_route(handler_route, incoming_route) {
            self.path_param = Some(p)
        }
    }
    fn assamble_multi_seg_param(&mut self, handler_route: &Route, incoming_route: &Route) {
        if handler_route
            .r
            .ends_with(&[RouteComponent::MultiSegWildCard])
        {
            let mut s = vec![];
            for i in 0..incoming_route.r.len() {
                if i >= handler_route.r.len() - 1
                    && let RouteComponent::Exact(ref p) = incoming_route.r[i]
                {
                    s.push(p.as_str());
                }
            }
            self.multi_seg_param = Some(s.join("/"))
        }
    }
    pub fn get_header<K: AsHeaderName>(&self, k: K) -> Option<&HeaderValue> {
        self._inner.headers().get(k)
    }
    pub(crate) fn process_routes(&mut self, handler_route: &Route, incoming_route: &Route) {
        self.assamble_path_param(handler_route, incoming_route);
        self.assamble_multi_seg_param(handler_route, incoming_route);
    }

    pub(crate) fn process_search_param(&mut self) {
        self.search_param = Some(SearchParam::from_query(self._inner.uri().query()));
    }

    pub fn get_pathparam<S: AsRef<str>>(&self, key: S) -> Option<&String> {
        if let Some(ref p) = self.path_param {
            p.get(key)
        } else {
            None
        }
    }

    pub fn get_search_param<S: AsRef<str>>(&self, _key: S) -> Option<&String> {
        if let Some(s) = self.search_param.as_ref() {
            s._inner.get(_key.as_ref())
        } else {
            None
        }
    }
    pub(crate) async fn parse_h1<R: AsyncRead + Unpin>(
        r: &mut R,
        buf: &mut BytesMut,
    ) -> Result<HttpRequest, String> {
        parse_http_frame(r, buf).await
    }
    pub(crate) async fn parse_h2(stream_req: Request<RecvStream>) -> Result<HttpRequest, String> {
        use ContentType::*;
        let (p, body_stream) = stream_req.into_parts();
        if p.method == Method::CONNECT {
            return Ok(HttpRequest::new(
                p,
                Some(RequestBody::WebSocketStreamBody(body_stream)),
            ));
        }
        let content_type = ContentType::try_from(&p.headers)?;
        let body = match content_type {
            MultipartFormData(boundary) => {
                H2MultiPartBodyParser::parse_h2(body_stream, boundary.as_bytes()).await?
            }
            _ => parse_simple_h2_body(body_stream).await?,
        };
        let req = HttpRequest::new(p, Some(body));

        Ok(req)
    }
}
async fn parse_simple_h2_body(mut body_stream: RecvStream) -> Result<RequestBody, String> {
    let mut buf = BytesMut::with_capacity(1024);
    while let Some(chunk) = body_stream.data().await {
        let chunk = chunk.map_err(map_str!())?;
        let s = chunk
            .chunk()
            .iter()
            .map(|b| format!("0x{:02x}", b))
            .collect::<Vec<_>>()
            .join(", ");

        println!("[{s}]");

        let len = chunk.len();
        buf.put(chunk);
        body_stream
            .flow_control()
            .release_capacity(len)
            .map_err(map_str!())?;
    }
    Ok(RequestBody::Simple(buf.freeze()))
}

pub fn is_websocket_upgrade(req: &HttpRequest) -> bool {
    req.get_header(UPGRADE).is_some()
        && req.get_header(CONNECTION).is_some()
        && req.get_header(SEC_WEBSOCKET_KEY).is_some()
        && req.get_header(SEC_WEBSOCKET_VERSION).is_some()
}

async fn parse_http_frame<R: AsyncRead + Unpin>(
    r:&mut R,
    buf: &mut BytesMut,
) -> Result<HttpRequest, String> {
    let mut builder = http::Request::builder();
    builder = parse_line_header_frame(r, buf, builder).await?;
    let mut req = HttpRequest::from_req(builder.body(None).map_err(map_str!())?);
    if let Some(len) = req.get_header(CONTENT_LENGTH) {
        let len = len
            .to_str()
            .map_err(map_str!())?
            .parse::<usize>()
            .map_err(map_str!())?;

        let body = parse_body_frame(len, r, buf, req._inner.headers()).await?;
        // let body = buf.split_to(len).freeze();
        *req._inner.body_mut() = Some(body);
    }
    // println!("{:?}", req);
    Ok(req)
}

pub async fn parse_body_frame<R: AsyncRead + Unpin>(
    len: usize,
    r: &mut R,
    buf: &mut BytesMut,
    headers: &HeaderMap<HeaderValue>,
) -> Result<RequestBody, String> {
    use ContentType::*;
    let content_type = ContentType::try_from(headers)?;
    match content_type {
        ApplicationJson => parse_simple_body(r, buf, len).await,
        MultipartFormData(boundary) => MultiPartBodyParser::parse_h1(r, buf, len, boundary).await,
        _ => parse_simple_body(r, buf, len).await,
    }
}

async fn parse_simple_body<R: AsyncRead + Unpin>(
    r: &mut R,
    buf: &mut BytesMut,
    len: usize,
) -> Result<RequestBody, String> {
    loop {
        if buf.len() >= len {
            let body = buf.split_to(len).freeze();
            return Ok(RequestBody::Simple(body));
        }
        if let Ok(len) = r.read_buf(buf).await {
            if len == 0 {
                return Err("other side closed".to_string());
            }
        } else {
            return Err("error!".to_string());
        }
    }
}
async fn parse_line_header_frame<R: AsyncRead + Unpin>(
    r: &mut R,
    buf: &mut BytesMut,
    builder: http::request::Builder,
) -> Result<http::request::Builder, String> {
    loop {
        if let Ok(read_len) = r.read_buf(buf).await {
            if read_len == 0 {
                return Err("other side closed".to_string());
            }
            let (check_header_is_ok, position) = check_header(buf.chunk());
            if check_header_is_ok {
                let b = parse_line_header(&buf[..position], builder)?;
                buf.advance(position);
                return Ok(b);
            }
        } else {
            return Err(
                "reading bytes from socket error while parsing parse_line_header_frame".to_string(),
            );
        }
    }
}
fn parse_line(builder: http::request::Builder, s: &str) -> Result<http::request::Builder, String> {
    let mut head_line = s.split_whitespace();
    let method = head_line.next().ok_or("method parsing error".to_string())?;
    let method: http::Method = method.try_into().map_err(map_str!())?;

    let uri = head_line.next().ok_or("uri parsing error".to_string())?;
    let uri = Uri::from_str(uri).map_err(map_str!())?;

    let version = head_line
        .next()
        .ok_or("version parsing error no headline".to_string())?;
    let v = match version {
        "HTTP/1.1" => Version::HTTP_11,
        "HTTP/1.0" => Version::HTTP_10,
        _ => return Err("version parsing error".to_string()),
    };
    Ok(builder.method(method).uri(uri).version(v))
}

fn parse_line_header(
    raw_header: &[u8],
    builder: http::request::Builder,
) -> Result<http::request::Builder, String> {
    let raw_header = str::from_utf8(raw_header).map_err(map_str!())?;
    let mut raw_header = raw_header.split("\r\n");
    let mut builder = parse_line(
        builder,
        raw_header
            .next()
            .ok_or("parse req line error-> no req line")?,
    )?;
    let header_map = builder.headers_mut().unwrap();

    for h in raw_header {
        if !h.is_empty() {
            if let Some((k,v)) = h.split_once(":") {
                let value = v.trim().parse().map_err(map_str!())?;
                let name = HeaderName::from_str(k.trim()).unwrap();
                header_map.insert(name, value);
            }else {
                Err("header parsing error".to_string())?
            }
        }
    }
    Ok(builder)
}

fn check_header(c: &[u8]) -> (bool, usize) {
    for i in 0..=c.len() - 4 {
        if &c[i..i + 4] == b"\r\n\r\n" {
            return (true, i + 4);
        }
    }
    (false, 0)
}

pub trait ConvertFromRefString<'a, O> {
    fn convert(self) -> Result<O, String>;
}

impl<'a, T> ConvertFromRefString<'a, T> for T {
    fn convert(self) -> Result<T, String> {
        Ok(self)
    }
}

impl<'a> ConvertFromRefString<'a, Option<&'a str>> for &'a String {
    fn convert(self) -> Result<Option<&'a str>, String> {
        Ok(Some(self.as_str()))
    }
}

impl<'a> ConvertFromRefString<'a, &'a str> for &'a String {
    fn convert(self) -> Result<&'a str, String> {
        Ok(self.as_str())
    }
}
impl<'a> ConvertFromRefString<'a, String> for &'a String {
    fn convert(self) -> Result<String, String> {
        Ok(self.to_string())
    }
}
//format!("can not convert String \"{}\" to type {}",value,stringify!($t))
macro_rules! impl_convert_from_ref_string2 {
    ($($t:ty),*) => {
        $(
            impl $crate::request::TryConvertFrom<&String> for  $t {
                fn try_convert_from(value:&String) -> Result<Self,$crate::handler::types::HttpHandlerError> {
                    value.parse::<$t>().map_err(|_| $crate::error::Error::before_handler_invalid_param(format!("can not convert String \"{}\" to type {}",value,stringify!($t))))
                }
            }

        )*
    };
}

macro_rules! impl_convert_from_option_ref_string {
    ($($t:ty),*) => {
        $(
            impl $crate::TryConvertFrom<Option<&String>> for  $t {
                fn try_convert_from(value:Option<&String>) -> Result<Self,$crate::handler::types::HttpHandlerError> {
                    if let Some(value) = value {
                        value.parse::<Self>().map_err(|_| $crate::error::Error::before_handler_invalid_param(format!("can not convert String \"{}\" to type {}",value,stringify!($t))))
                    }else {
                        Err($crate::error::Error::before_handler_invalid_param("value is missing"))
                    }
                }
            }
        )*
    };
}

impl_convert_from_ref_string2!(
    i8, i16, i32, i64, i128, isize, usize, f32, f64, u8, u16, u32, u64, u128, bool
);
// impl_convert_from_ref_string!(
//     i8, i16, i32, i64, i128, isize, usize, f32, f64, u8, u16, u32, u64, u128, bool
// );
impl_convert_from_option_ref_string!(
    i8, i16, i32, i64, i128, isize, usize, f32, f64, u8, u16, u32, u64, u128, bool
);
impl<'a> TryConvertFrom<&'a String> for &'a String {
    fn try_convert_from(value: &'a String) -> Result<Self, HttpHandlerError> {
        Ok(value)
    }
}
impl<'a> TryConvertFrom<Option<&'a String>> for &'a String {
    fn try_convert_from(value: Option<&'a String>) -> Result<Self, HttpHandlerError> {
        if let Some(value) = value {
            Ok(value)
        } else {
            Err(crate::error::Error::before_handler_invalid_param("value is missing!"))
        }
    }
}

impl<'a> TryConvertFrom<&'a String> for &'a str {
    fn try_convert_from(value: &'a String) -> Result<Self, HttpHandlerError> {
        Ok(value.as_str())
    }
}
impl<'a> TryConvertFrom<Option<&'a String>> for &'a str {
    fn try_convert_from(value: Option<&'a String>) -> Result<Self, HttpHandlerError> {
        if let Some(value) = value {
            Ok(value)
        } else {
            Err(crate::error::Error::before_handler_invalid_param("value is missing!"))
        }
    }
}

impl TryConvertFrom<&String> for String {
    fn try_convert_from(value: &String) -> Result<Self, HttpHandlerError> {
        Ok(value.to_string())
    }
}
impl TryConvertFrom<Option<&String>> for String {
    fn try_convert_from(value: Option<&String>) -> Result<Self, HttpHandlerError> {
        if let Some(value) = value {
            Ok(value.to_string())
        } else {
            Err(crate::error::Error::before_handler_invalid_param("value is missing!"))
        }
    }
}

impl<'a, O: TryConvertFrom<&'a String>> TryConvertFrom<&'a String> for Option<O> {
    fn try_convert_from(value: &'a String) -> Result<Self, HttpHandlerError> {
        match O::try_convert_from(value) {
            Ok(r) => Ok(Some(r)),
            Err(_) => Ok(None),
        }
    }
}
impl<'a, O: TryConvertFrom<Option<&'a String>>> TryConvertFrom<Option<&'a String>> for Option<O> {
    fn try_convert_from(value: Option<&'a String>) -> Result<Self, HttpHandlerError> {
        match O::try_convert_from(value) {
            Ok(r) => Ok(Some(r)),
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use http::Request;

    use crate::{TryConvertInto, handler::types::HttpHandlerError, request::{ConvertFromRefString, parse_line_header}};

    #[test]
    fn parse_http1_line() {
        let  b = Request::builder();
        let a = parse_line_header(b"GET /hello HTTP/1.1\r\nauth:abc:caonima\r\nlen:123\r\n\r\n", b).unwrap();
        let r = a.body(()).unwrap();
        let a = r.headers().get("auth").unwrap();
        assert_eq!(a, "abc:caonima");
        let a =r.headers().get("len").unwrap();
        assert_eq!(a, "123");
    }

    #[test]
    fn number_test() {
        let s = &"11".to_string();
        let a: Result<i32, HttpHandlerError> = s.try_convert_into();
        let b: Result<i32, HttpHandlerError> = s.try_convert_into();
        assert_eq!(a.is_ok(), b.is_ok())
    }

    #[test]
    fn bool_test() {
        let s = &"true".to_string();
        let a: Result<bool, HttpHandlerError> = s.try_convert_into();
        let b: Result<bool, HttpHandlerError> = s.try_convert_into();
        assert_eq!(a.is_ok(), b.is_ok())
    }

    #[test]
    fn str_test() {
        let s = &"true".to_string();
        let a: Result<String, HttpHandlerError> = s.try_convert_into();
        let b: Result<String, String> = s.convert();
        assert_eq!(a.is_ok(), b.is_ok())
    }
    #[test]
    fn option_test() {
        let s = &"true".to_string();
        let a: Result<Option<i32>, HttpHandlerError> = s.try_convert_into();
        assert_eq!(a.is_ok(), true);
        fn a2(_: Option<bool>) {}
        a2(s.try_convert_into().map_err(|_| "").unwrap());
    }
}
