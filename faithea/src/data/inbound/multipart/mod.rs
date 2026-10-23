pub mod parser;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use thiserror::Error;

use crate::request::{HttpRequest, RequestBody, TryFromRequest, error::ParseHandlerParamError};

pub type MultipartDataMap = HashMap<String, Vec<Part>>;

/// Errors that occur during multipart parsing.
#[derive(Debug, Error)]
pub enum MultipartError {
    #[error("field not exist")]
    FieldNotExist,
    #[error("can not parse from part: {0}")]
    CanNotParseFromPart(String),
    #[error("incompatible type: {0}")]
    IncompatibleType(String),
}

/// macro generate!
pub trait TryFromMultipartDataMap: Sized {
    fn try_from_multipart_data_map(data: &mut MultipartDataMap) -> Result<Self, MultipartError>;
}

#[derive(Debug)]
pub enum Part {
    Lit(String),
    File(MultipartFile),
}
pub(crate) trait TryFromPart: Sized {
    fn try_from_part(part: Part) -> Result<Self, MultipartError>;
}
pub trait TryFromParts: Sized {
    fn try_from_parts(parts: Option<Vec<Part>>) -> Result<Self, MultipartError>;
}
macro_rules! impl_try_from_part_for_parse_from_str {
    ($($t:ty),*) => {
        $(
            impl TryFromParts for $t {
                fn try_from_parts(value: Option<Vec<Part>>) -> Result<Self, crate::data::inbound::multipart::MultipartError>{
                    use crate::data::inbound::multipart::MultipartError;
                    if let Some(mut v) = value && let Some(p) = v.pop() {
                        TryFromPart::try_from_part(p)
                    }else {
                         let e = MultipartError::FieldNotExist;
                        Err(e)
                    }
                }
            }

            impl TryFromPart for $t {
                fn try_from_part(value: Part) -> Result<Self,crate::data::inbound::multipart::MultipartError>{
                    use crate::data::inbound::multipart::MultipartError;
                    if let Part::Lit(l) = value {
                        Ok(l.parse::<Self>().map_err(|e|
                                MultipartError::CanNotParseFromPart(e.to_string())
                        )?)
                    }else {
                         let e = MultipartError::FieldNotExist;
                        Err(e)
                    }
                }
            }
        )*
    };
}

impl_try_from_part_for_parse_from_str!(
    i8, i16, i32, i64, i128, isize, usize, f32, f64, u8, u16, u32, u64, u128, bool, String
);

impl<T: TryFromParts + TryFromPart> TryFromParts for Option<T> {
    fn try_from_parts(parts: Option<Vec<Part>>) -> Result<Self, MultipartError> {
        match parts {
            Some(mut p) => {
                if let Some(p) = p.pop() {
                    Ok(Some(TryFromPart::try_from_part(p)?))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}

impl<T: TryFromParts> TryFromParts for Vec<T> {
    fn try_from_parts(parts: Option<Vec<Part>>) -> Result<Self, MultipartError> {
        if let Some(parts) = parts {
            Ok(parts
                .into_iter()
                .map(|x| TryFromParts::try_from_parts(Some(vec![x])))
                .collect::<Result<Vec<_>, _>>()?)
        } else {
            Ok(vec![])
        }
    }
}
/// An uploaded file, available at `temp_path` until this value is dropped.
#[derive(Debug)]
pub struct MultipartFile {
    pub file_name: Option<String>,
    pub temp_path: String,
    pub mime_type: Option<String>,
}

impl Drop for MultipartFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.temp_path.as_str());
    }
}

impl TryFromPart for MultipartFile {
    fn try_from_part(value: Part) -> Result<Self, MultipartError> {
        if let Part::File(f) = value {
            Ok(f)
        } else {
            Err(MultipartError::IncompatibleType("Expect a File!".into()))
        }
    }
}
impl TryFromParts for MultipartFile {
    fn try_from_parts(parts: Option<Vec<Part>>) -> Result<Self, MultipartError> {
        if let Some(mut parts) = parts
            && let Some(Part::File(f)) = parts.pop()
        {
            Ok(f)
        } else {
            Err(MultipartError::IncompatibleType("Expect a File!".into()))
        }
    }
}

#[derive(Debug)]
pub struct Multipart<T: TryFromMultipartDataMap>(T);

impl<T: TryFromMultipartDataMap> Multipart<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: TryFromMultipartDataMap> Deref for Multipart<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: TryFromMultipartDataMap> DerefMut for Multipart<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<'a, T: TryFromMultipartDataMap> TryFromRequest<'a> for Multipart<T> {
    fn try_from_request(req: &'a mut HttpRequest) -> Result<Self, ParseHandlerParamError> {
        match req.inner.body_mut() {
            Some(RequestBody::Multipart(body)) => {
                Ok(Multipart(T::try_from_multipart_data_map(body)?))
            }
            _ => Err(ParseHandlerParamError::BodyNotExist),
        }
    }
}
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn convert_to_base_type() {
        let p = Some(vec![Part::Lit("hello".into())]);
        let a: String = TryFromParts::try_from_parts(p).unwrap();
        assert_eq!(a, "hello")
    }
    #[test]
    fn convert_to_vec() {
        let p = Some(vec![Part::Lit("hello".into())]);

        let a: Vec<String> = TryFromParts::try_from_parts(p).unwrap();
        assert_eq!(a, vec!["hello"])
    }
    #[test]
    fn convert_to_option() {
        let p = Some(vec![Part::Lit("hello".into())]);
        let a: Option<String> = TryFromParts::try_from_parts(p).unwrap();
        assert_eq!(a, Some("hello".to_string()))
    }
}
