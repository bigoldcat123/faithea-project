pub mod content_type;
pub mod cookie;
pub mod error;
// pub mod method;
pub mod path_param;
pub mod search_param;
use std::{fmt::Debug, path::PathBuf};

use bytes::{Buf, Bytes, BytesMut};
use http::{
    HeaderMap, HeaderValue, Request, Uri,
    header::{AsHeaderName, COOKIE},
};
use thiserror::Error;

use crate::{
    TryConvertFrom,
    data::inbound::multipart::{MultipartDataMap, parser::h1::MultiPartBodyParser},
    error::BeforeHandlerError,
    handler::types::HttpHandlerError,
    map_str,
    request::{
        content_type::ContentType, cookie::Cookie, error::ParseHandlerParamError,
        path_param::PathParam, search_param::SearchParam,
    },
    route::{Route, RouteComponent},
    server::BytesSource,
};

pub enum ParseRequestBodyError {}

pub enum RequestBody {
    Simple(Bytes),
    MultiPart(MultipartDataMap),
    Stream(PathBuf), // the path to a file saved on the disk
}
impl Debug for RequestBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Simple(_) => write!(f, "Simple(Bytes)")?,
            Self::MultiPart(_) => write!(f, "MultiPart(MultipartDataMap)")?,
            Self::Stream(_) => write!(f, "Stream(PathBuf)")?,
        }
        Ok(())
    }
}
#[derive(Debug, Error)]
#[error("ConvertError: can not conver Value:\"{from}\" to Type: {to}")]
pub struct ConvertError {
    /// Value
    pub from: String,
    /// Type
    pub to: String,
}
pub trait TryFromParam<'a>: Sized {
    fn try_from_param(value: &'a str) -> Result<Self, ParseHandlerParamError>;
}
// impl<T: TryFromParam> TryConvertFrom<&String> for T {
//     fn try_convert_from(value: &String) -> Result<Self, HttpHandlerError> {
//         Self::try_from_param(value)
//     }
// }
pub trait TryFromRequest<'a>: Sized {
    fn try_from_request(req: &'a mut HttpRequest) -> Result<Self, ParseHandlerParamError>;
}
impl<'a, T: TryFromRequest<'a>> TryConvertFrom<&'a mut HttpRequest> for T {
    type Error = HttpHandlerError;
    fn try_convert_from(value: &'a mut HttpRequest) -> Result<Self, Self::Error> {
        Self::try_from_request(value)
            .map_err(Into::<BeforeHandlerError>::into)
            .map_err(Into::into)
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
    pub fn body(&mut self) -> &mut Option<RequestBody> {
        self._inner.body_mut()
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
        if let Some(cookie) = self._inner.headers().get(COOKIE) {
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

    pub fn uri(&self) -> &Uri {
        self._inner.uri()
    }

    pub fn get_search_param<S: AsRef<str>>(&self, _key: S) -> Option<&String> {
        if let Some(s) = self.search_param.as_ref() {
            s._inner.get(_key.as_ref())
        } else {
            None
        }
    }
}

pub async fn parse_body_frame<SOURCE: BytesSource>(
    bs: SOURCE,
    buf: &mut BytesMut,
    headers: &HeaderMap<HeaderValue>,
) -> Result<RequestBody, String> {
    use ContentType::*;
    let content_type = ContentType::try_from(headers)?;
    match content_type {
        ApplicationJson => parse_simple_body(bs, buf).await,
        MultipartFormData(boundary) => MultiPartBodyParser::parse(bs, buf, boundary).await,
        _ => parse_simple_body(bs, buf).await,
    }
}

async fn parse_simple_body<R: BytesSource>(
    mut r: R,
    buf: &mut BytesMut,
) -> Result<RequestBody, String> {
    loop {
        if r.is_end() {
            let body = buf.split_to(buf.remaining()).freeze();
            return Ok(RequestBody::Simple(body));
        }
        let _len = r.read_buf2(buf).await.map_err(map_str!())?;
    }
}
macro_rules! impl_convert_from_param {
    ($($t:ty),*) => {

        $(
            impl <'a> $crate::request::TryFromParam<'a> for  $t {
                fn try_from_param(value:&'a str) -> Result<Self,$crate::request::error::ParseHandlerParamError> {
                    use $crate::request::ConvertError;
                    value.parse::<$t>().map_err(|_| ConvertError {from:value.to_string(),to:stringify!($t).to_string()}).map_err(Into::into)
                    // value.parse::<$t>().map_err(|_| $crate::error::Error::before_handler_invalid_param(format!("can not convert String \"{}\" to type {}",value,stringify!($t))))
                }
            }
        )*
    };
}

impl_convert_from_param!(
    i8, i16, i32, i64, i128, isize, usize, f32, f64, u8, u16, u32, u64, u128, bool
);
impl<'a> TryFromParam<'a> for &'a str {
    fn try_from_param(value: &'a str) -> Result<Self, ParseHandlerParamError> {
        Ok(value)
    }
}
// impl <'a> TryFromParam<'a> for &'a String {
//     fn try_from_param(value: &'a str) -> Result<Self, HttpHandlerError> {
//         Ok(value)
//     }
// }
impl<'a> TryFromParam<'a> for String {
    fn try_from_param(value: &'a str) -> Result<Self, ParseHandlerParamError> {
        Ok(value.to_string())
    }
}

impl<'a, T: TryFromParam<'a>> TryConvertFrom<Option<&'a String>> for T {
    type Error = HttpHandlerError;
    fn try_convert_from(value: Option<&'a String>) -> Result<Self, Self::Error> {
        if let Some(value) = value {
            T::try_from_param(value)
                .map_err(Into::<ParseHandlerParamError>::into)
                .map_err(Into::<BeforeHandlerError>::into)
                .map_err(Into::<Self::Error>::into)
        } else {
            Err(Into::<Self::Error>::into(Into::<BeforeHandlerError>::into(
                ParseHandlerParamError::ParamNotExist,
            )))
        }
    }
}
impl<'a, T: TryFromParam<'a>> TryConvertFrom<Option<&'a String>> for Option<T> {
    type Error = HttpHandlerError;
    fn try_convert_from(value: Option<&'a String>) -> Result<Self, Self::Error> {
        if let Some(value) = value {
            T::try_from_param(value)
                .map(|x| Some(x))
                .map_err(Into::<ParseHandlerParamError>::into)
                .map_err(Into::<BeforeHandlerError>::into)
                .map_err(Into::<Self::Error>::into)
        } else {
            Ok(None)
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::{TryConvertInto, handler::types::HttpHandlerError};

    #[test]
    fn number_test() {
        let s = Some(&"11".to_string());
        let a: Result<i32, HttpHandlerError> = s.try_convert_into();
        let b: Result<i32, HttpHandlerError> = s.try_convert_into();
        assert_eq!(a.is_ok(), b.is_ok())
    }

    #[test]
    fn bool_test() {
        let s = Some(&"true".to_string());
        let a: Result<bool, HttpHandlerError> = s.try_convert_into();
        let b: Result<bool, HttpHandlerError> = s.try_convert_into();
        assert_eq!(a.is_ok(), b.is_ok())
    }

    #[test]
    fn str_test() {
        let s = Some(&"true".to_string());
        let a: Result<String, HttpHandlerError> = s.try_convert_into();
        let b: Result<String, HttpHandlerError> = s.try_convert_into();
        assert_eq!(a.is_ok(), b.is_ok())
    }
    #[test]
    fn option_test() {
        let s = Some(&"true".to_string());
        let a: Result<bool, HttpHandlerError> = s.try_convert_into();
        assert_eq!(a.is_ok(), true);
        fn a2(_: bool) {}
        a2(s.try_convert_into().map_err(|_| "").unwrap());
    }
}
