pub mod content_type;
pub mod cookie;
pub mod method;
pub mod path_param;
pub mod search_param;
use bytes::{Buf, Bytes, BytesMut};
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};

use crate::{
    HttpHeader, TryConvertFrom,
    data::{
        inbound::multipart::{MultiPartBodyParser, MultipartDataMap},
        outbound::StaticFile,
    },
    map_str,
    request::{
        content_type::ContentType, cookie::Cookie, method::Method, path_param::PathParam,
        search_param::SearchParam,
    },
    res_modifiers,
    response::HttpResponseModifier,
    route::{Route, RouteComponent},
};

#[derive(Debug)]
pub enum RequestBody {
    Simple(Bytes),
    MultiPart(MultipartDataMap),
}

#[derive(Debug)]
pub struct HttpRequest {
    pub(crate) req_line: HttpReqLine,
    pub(crate) headers: HttpHeader,
    pub(crate) body: Option<RequestBody>, // way too big!!!!!!!!!!!!
    pub(crate) path_param: Option<PathParam>,
    pub(crate) search_param: Option<SearchParam>,
    pub(crate) multi_seg_param: Option<String>,
    // pub(crate) cookie:Option<Cookie<'a>>
}

pub async fn static_map<P: AsRef<str>>(
    _req: &HttpRequest,
    path: P,
) -> Vec<Box<dyn HttpResponseModifier + Send + Sync>> {
    if let Some(multi_seg_param) = _req.multi_seg_param.as_ref() {
        let a: StaticFile<String> = StaticFile(format!("{}/{}", path.as_ref(), multi_seg_param));
        res_modifiers!(a)
    } else {
        res_modifiers!("no multi_seg_param found try to use /abc/** route!")
    }
}

impl HttpRequest {
    pub fn new(req_line: HttpReqLine, headers: HttpHeader, body: Option<RequestBody>) -> Self {
        Self {
            req_line,
            headers,
            body,
            path_param: None,
            multi_seg_param: None,
            search_param: None,
            // cookie:None
        }
    }
    //#[allow(unused)]
    pub fn cookies<'a>(&'a self) -> Option<Cookie<'a>> {
        self.headers
            .get("cookie")
            .map(|cookie| Cookie::from_cookie_header(cookie))
    }
    pub fn fake() -> Self {
        let req_line = HttpReqLine::parse("POST /api/data HTTP/1.1").unwrap();
        let headers = HttpHeader::new();
        let body = Some(RequestBody::Simple(Bytes::from("request payload")));
        HttpRequest::new(req_line, headers, body)
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
    pub(crate) fn process_routes(&mut self, handler_route: &Route, incoming_route: &Route) {
        self.assamble_path_param(handler_route, incoming_route);
        self.assamble_multi_seg_param(handler_route, incoming_route);
    }

    pub(crate) fn process_search_param(&mut self, url: &str) {
        self.search_param = Some(SearchParam::from_url(url));
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
}

#[derive(Debug)]
pub struct HttpReqLine {
    pub method: Method,
    pub url: String,
    pub version: String,
}

impl HttpReqLine {
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut head_line = s.split_whitespace();
        let method = head_line
            .next()
            .ok_or("method parsing error".to_string())?
            .to_string();
        let url = head_line
            .next()
            .ok_or("url parsing error".to_string())?
            .to_string();
        let version = head_line
            .next()
            .ok_or("version parsing error".to_string())?
            .to_string();
        Ok(Self {
            method: method.try_into()?,
            url,
            version,
        })
    }
}

pub async fn parse_http_frame(
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
) -> Result<HttpRequest, String> {
    let (l, h) = parse_line_header_frame(r, buf).await?;
    let mut req = HttpRequest::new(l, h, None);
    if let Some(len) = req.headers.get("content-length") {
        let len = len.parse::<usize>().map_err(map_str!())?;
        let body = parse_body_frame2(len, r, buf, &req.headers).await?;
        // let body = buf.split_to(len).freeze();
        req.body = Some(body);
    }
    Ok(req)
}

pub async fn parse_body_frame2(
    len: usize,
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
    headers: &HttpHeader,
) -> Result<RequestBody, String> {
    use ContentType::*;
    let content_type = ContentType::try_from(headers)?;
    match content_type {
        ApplicationJson => parse_simple_body(r, buf, len).await,
        MultipartFormData(boundary) => MultiPartBodyParser::parse(r, buf, len, boundary).await,
        _ => parse_simple_body(r, buf, len).await,
    }
}

async fn parse_simple_body(
    r: &mut OwnedReadHalf,
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
async fn parse_line_header_frame(
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
) -> Result<(HttpReqLine, HttpHeader), String> {
    loop {
        if let Ok(read_len) = r.read_buf(buf).await {
            if read_len == 0 {
                return Err("other side closed".to_string());
            }
            let (check_header_is_ok, position) = check_header(buf.chunk());
            if check_header_is_ok {
                let (l, h) = parse_line_header(&buf[..position])?;
                buf.advance(position);
                return Ok((l, h));
            }
        } else {
            return Err(
                "reading bytes from socket error while parsing parse_line_header_frame".to_string(),
            );
        }
    }
}

fn parse_line_header(raw_header: &[u8]) -> Result<(HttpReqLine, HttpHeader), String> {
    let raw_header = str::from_utf8(raw_header).map_err(map_str!())?;
    let mut raw_header = raw_header.split("\r\n");
    let req_line = HttpReqLine::parse(
        raw_header
            .next()
            .ok_or("parse req line error".to_string())?,
    )?;
    let mut http_header = HttpHeader::new();
    for h in raw_header {
        if !h.is_empty() {
            http_header.parse_new_header(h)?;
        }
    }
    Ok((req_line, http_header))
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

macro_rules! impl_convert_from_ref_string2 {
    ($($t:ty),*) => {
        $(
            impl $crate::request::TryConvertFrom<&String> for  $t {
                fn try_convert_from(value:&String) -> Result<Self,String> {
                    value.parse::<$t>().map_err(|_|format!("can not convert String \"{}\" to type {}",value,stringify!($t)))
                }
            }

        )*
    };
}

macro_rules! impl_convert_from_option_ref_string {
    ($($t:ty),*) => {
        $(
            impl $crate::TryConvertFrom<Option<&String>> for  $t {
                fn try_convert_from(value:Option<&String>) -> Result<Self,String> {
                    if let Some(value) = value {
                        value.parse::<Self>().map_err(|_|format!("can not convert String \"{}\" to type {}",value,stringify!($t)))
                    }else {
                        Err("value is missing".to_string())
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
    fn try_convert_from(value: &'a String) -> Result<Self, String> {
        Ok(value)
    }
}
impl<'a> TryConvertFrom<Option<&'a String>> for &'a String {
    fn try_convert_from(value: Option<&'a String>) -> Result<Self, String> {
        if let Some(value) = value {
            Ok(value)
        } else {
            Err("missing value".into())
        }
    }
}

impl<'a> TryConvertFrom<&'a String> for &'a str {
    fn try_convert_from(value: &'a String) -> Result<Self, String> {
        Ok(value.as_str())
    }
}
impl<'a> TryConvertFrom<Option<&'a String>> for &'a str {
    fn try_convert_from(value: Option<&'a String>) -> Result<Self, String> {
        if let Some(value) = value {
            Ok(value)
        } else {
            Err("missing value".into())
        }
    }
}

impl TryConvertFrom<&String> for String {
    fn try_convert_from(value: &String) -> Result<Self, String> {
        Ok(value.to_string())
    }
}
impl TryConvertFrom<Option<&String>> for String {
    fn try_convert_from(value: Option<&String>) -> Result<Self, String> {
        if let Some(value) = value {
            Ok(value.to_string())
        } else {
            Err("missing value".into())
        }
    }
}

impl<'a, O: TryConvertFrom<&'a String>> TryConvertFrom<&'a String> for Option<O> {
    fn try_convert_from(value: &'a String) -> Result<Self, String> {
        match O::try_convert_from(value) {
            Ok(r) => Ok(Some(r)),
            Err(_) => Ok(None),
        }
    }
}
impl<'a, O: TryConvertFrom<Option<&'a String>>> TryConvertFrom<Option<&'a String>> for Option<O> {
    fn try_convert_from(value: Option<&'a String>) -> Result<Self, String> {
        match O::try_convert_from(value) {
            Ok(r) => Ok(Some(r)),
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{TryConvertInto, request::ConvertFromRefString};
    #[test]
    fn number_test() {
        let s = &"11".to_string();
        let a: Result<i32, String> = s.try_convert_into();
        let b: Result<i32, String> = s.try_convert_into();
        assert_eq!(a, b)
    }

    #[test]
    fn bool_test() {
        let s = &"true".to_string();
        let a: Result<bool, String> = s.try_convert_into();
        let b: Result<bool, String> = s.try_convert_into();
        assert_eq!(a, b)
    }

    #[test]
    fn str_test() {
        let s = &"true".to_string();
        let a: Result<String, String> = s.try_convert_into();
        let b: Result<String, String> = s.convert();
        assert_eq!(a, b)
    }
    #[test]
    fn option_test() {
        let s = &"true".to_string();
        let a: Option<i32> = s.try_convert_into().unwrap();
        assert_eq!(a, None);
        fn a2(_: Option<bool>) {}
        a2(s.try_convert_into().unwrap());
    }

    // #[test]
    // fn url_encoded_special_chars() {
    //     // key 里带 = & 空格，value 里带 & =
    //     let url = "https://example.com/?key%3D%26=v%26a%3Dl%20ue";
    //     let s = SearchParam::from_url(url);
    //     assert_eq!(s._inner, map! { "key=&" => "v&al ue" });
    // }

    // #[test]
    // fn fragment_should_be_ignored() {
    //     let url = "https://example.com/?a=1&b=2#fragment";
    //     let s = SearchParam::from_url(url);
    //     assert_eq!(s._inner, map! { "a" => "1", "b" => "2" });
    // }

    // #[test]
    // fn plus_sign() {
    //     // 加号在 query-string 中被解释为空格（application/x-www-form-urlencoded）
    //     let url = "https://example.com/?msg=hello+world";
    //     let s = SearchParam::from_url(url);
    //     assert_eq!(s._inner, map! { "msg" => "hello world" });
    // }

    // #[test]
    // fn non_utf8_percent() {
    //     // 非法 UTF-8 百分号序列，应不 panic，能跳过或给出空值
    //     let url = "https://example.com/?key=%FF%FE";
    //     let s = SearchParam::from_url(url);
    //     // 这里只是断言不 panic，具体行为取决于你用的 urldecode 库
    //     // 如果解码失败返回 ""，则：
    //     assert_eq!(s._inner.get("key").map(|v| v.as_str()), Some(""));
    // }

    // #[test]
    // fn key_without_equal() {
    //     // 没有等号时，value 视为空字符串
    //     let url = "https://example.com/?flag";
    //     let s = SearchParam::from_url(url);
    //     assert_eq!(s._inner, map! { "flag" => "" });
    // }

    // #[test]
    // fn multibyte_unicode() {
    //     let url = "https://example.com/?name=测试&emoji=%F0%9F%98%82";
    //     let s = SearchParam::from_url(url);
    //     assert_eq!(s._inner, map! { "name" => "测试", "emoji" => "😂" });
    // }
}
