

use std::{collections::HashMap};

use bytes::{Buf, Bytes, BytesMut};
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};

use crate::{
    HttpHeader,
    data::outbound::StaticFile,
    impl_convert_from_ref_string, map_str, res_modifiers,
    response::HttpResponseModifier,
    route::{Route, RouteComponent},
};

#[derive(Debug, Default)]
pub(crate) struct SearchParam {
    _inner: HashMap<String, String>,
}

impl SearchParam {
    fn from_url(url: &str) -> Self {
        let mut map = HashMap::new();
        if let Some((_, search_params)) = url.split_once("?") {
            for (k, v) in search_params.split("&").filter_map(|x| x.split_once("=")) {
                if let Ok(ok) = urlencoding::decode(v) {
                    map.insert(k.into(), ok.to_string());
                }
            }
        }
        Self { _inner: map }
    }
}

#[derive(Debug, Default)]
pub struct PathParam {
    _inner: HashMap<String, String>,
}
impl PathParam {
    pub fn get<S: AsRef<str>>(&self, key: S) -> Option<&String> {
        self._inner.get(key.as_ref())
    }
    pub fn try_from_route(handler_route: &Route, incoming_route: &Route) -> Result<Self, String> {
        use crate::route::RouteComponent::*;
        let mut _inner = HashMap::new();
        if handler_route.r.len() != incoming_route.r.len() {
            return Err("route len not match!".to_string());
        }
        for cmp in handler_route.r.iter().zip(incoming_route.r.iter()) {
            match cmp {
                (PathParam(p), Exact(v)) => {
                    _inner.insert(p.to_string(), v.to_string());
                }
                _ => {}
            }
        }
        if _inner.is_empty() {
            Err("no path params".to_string())
        } else {
            Ok(Self { _inner })
        }
    }
}

#[derive(Debug)]
pub struct HttpRequest {
    pub(crate) req_line: HttpReqLine,
    pub(crate) headers: HttpHeader,
    pub(crate) body: Option<Bytes>,
    pub(crate) path_param: Option<PathParam>,
    pub(crate) search_param: Option<SearchParam>,
    pub(crate) multi_seg_param: Option<String>,
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

    pub fn new(req_line: HttpReqLine, headers: HttpHeader, body: Option<Bytes>) -> Self {
        Self {
            req_line,
            headers,
            body,
            path_param: None,
            multi_seg_param: None,
            search_param: None,
        }
    }
    pub fn fake() -> Self {
        let req_line = HttpReqLine::parse("POST /api/data HTTP/1.1").unwrap();
        let headers = HttpHeader::new();
        let body = Some(Bytes::from("request payload"));
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
                if i >= handler_route.r.len() - 1 {
                    if let RouteComponent::Exact(ref p) = incoming_route.r[i] {
                        s.push(p.as_str());
                    }
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
        }else {
            None
        }
    }
}

#[derive(Debug)]
pub struct HttpReqLine {
    pub method: String,
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
            method,
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
        parse_body_frame(len, r, buf).await?;
        let body = buf.split_to(len).freeze();
        req.body = Some(body);
    }
    Ok(req)
}

async fn parse_body_frame(
    len: usize,
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
) -> Result<(), String> {
    loop {
        if buf.len() >= len {
            break;
        }
        if let Ok(len) = r.read_buf(buf).await {
            if len == 0 {
                return Err("other side closed".to_string());
            }
        } else {
            return Err("error!".to_string());
        }
    }
    Ok(())
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
            return Err("error!".to_string());
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
impl_convert_from_ref_string!(
    i8, i16, i32, i64, i128, isize, usize, f32, f64, u8, u16, u32, u64, u128, bool
);

impl<'a, T> ConvertFromRefString<'a, T> for T {
    fn convert(self) -> Result<T, String> {
        Ok(self)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_single_param_parsing_test() {
        let handler_route = Route::from("/hello/{name}");
        let incoming_route = Route::from("/hello/chenzhonghai");
        let p = PathParam::try_from_route(&handler_route, &incoming_route).unwrap();
        let a = p.get("name").unwrap();
        assert_eq!(a, "chenzhonghai")
    }
    #[test]
    fn path_multi_param_parsing_test() {
        let handler_route = Route::from("/hello/{name}/{age}/dadigua");
        let incoming_route = Route::from("/hello/chenzhonghai/22/dadigua");
        let p = PathParam::try_from_route(&handler_route, &incoming_route).unwrap();
        let a = p.get("name").unwrap();
        let age = p.get("age").unwrap();
        assert_eq!(a, "chenzhonghai");
        assert_eq!(age, "22");
    }

    #[test]
    fn search_param_test() {
        let url = "";
        let s = SearchParam::from_url(url);
        assert_eq!(s._inner.is_empty(), true)
    }
    #[test]
    fn search_param_test2() {
        let url = "https://www.bilibili.com/?a=a=10&c=200";
        let s = SearchParam::from_url(url);
        assert_eq!(s._inner, HashMap::from([
            ("a".into(), "a=10".into()),
            ("c".into(), "200".into())
        ]))
    }
    /// 快速构造 HashMap 的宏，减少视觉噪音
    macro_rules! map {
        ($( $k:expr => $v:expr ),* $(,)?) => {
            HashMap::from([
                $( ($k.into(), $v.into()) ),*
            ])
        };
    }

    #[test]
    fn basic_kv() {
        let url = "https://example.com/?name=kimi&age=18";
        let s = SearchParam::from_url(url);
        assert_eq!(s._inner, map! { "name" => "kimi", "age" => "18" });
    }

    #[test]
    fn empty_value() {
        let url = "https://example.com/?key=";
        let s = SearchParam::from_url(url);
        assert_eq!(s._inner, map! { "key" => "" });
    }

    #[test]
    fn empty_key() {
        let url = "https://example.com/?=value";
        let s = SearchParam::from_url(url);
        // 空字符串当 key 也是合法实现，这里按「空 key」处理
        assert_eq!(s._inner, map! { "" => "value" });
    }

    #[test]
    fn duplicate_keys_keep_last() {
        let url = "https://example.com/?a=1&b=2&a=3";
        let s = SearchParam::from_url(url);
        assert_eq!(s._inner, map! { "a" => "3", "b" => "2" });
    }

    #[test]
    fn no_query_string() {
        let url = "https://example.com/path";
        let s = SearchParam::from_url(url);
        assert_eq!(s._inner, map! {});
    }

    #[test]
    fn question_mark_only() {
        let url = "https://example.com/?";
        let s = SearchParam::from_url(url);
        assert_eq!(s._inner, map! {});
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

    #[test]
    fn spaces_around_query() {
        let url = " https://example.com/?  x=1  & y = 2  ";
        let s = SearchParam::from_url(url);
        // 空格保留在 value 里，取决于你的 trim 策略，这里假设不自动 trim
        assert_eq!(s._inner, map! { "  x" => "1  ", " y " => " 2  " });
    }

    #[test]
    fn given_weird_case() {
        // 题目自带的用例
        let url = "https://www.bilibili.com/?a=a=10&c=200 ";
        let s = SearchParam::from_url(url);
        assert_eq!(s._inner, map! { "a" => "a=10", "c" => "200 " });
    }

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
