pub mod parser;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{
    TryConvertFrom,
    error::{BeforeHandlerError, MultipartError},
    handler::types::HttpHandlerError,
    request::{HttpRequest, RequestBody, TryFromRequest},
};

pub type MultipartDataMap = HashMap<String, Vec<Part>>;

/// macro generate!
pub trait TryFromMultipartDataMap: Sized {
    fn try_from_multipart_data_map(data: &mut MultipartDataMap) -> Result<Self, HttpHandlerError>;
}

#[derive(Debug)]
pub enum Part {
    Lit(String),
    File(MultiPartFile),
}
pub trait TryFromPart: Sized {
    fn try_from_part(part: Part) -> Result<Self, HttpHandlerError>;
}
macro_rules! impl_try_from_part_for_parse_from_str {
    ($($t:ty),*) => {
        $(
            impl TryFromPart for $t {
                fn try_from_part(value: Part) -> Result<Self, $crate::handler::types::HttpHandlerError>{
                    if let Part::Lit(l) = value {
                        Ok(l.parse::<Self>().map_err(|x| $crate::handler::types::HttpHandlerError::before_handler_multipart_can_not_parse_from_part(x.to_string()))?)
                    }else {
                        let e = $crate::handler::types::HttpHandlerError::before_handler_multipart_incompatible_type(format!("{} not compatiable to transform part to MultiPartFile",stringify!($t)));
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

impl<T: TryFromPart> TryConvertFrom<Option<Vec<Part>>> for T {
    fn try_convert_from(value: Option<Vec<Part>>) -> Result<Self, HttpHandlerError> {
        if let Some(mut value) = value {
            if let Some(value) = value.pop() {
                T::try_from_part(value)
            } else {
                Err(HttpHandlerError::before_handler_multipart_incompatible_type(""))
            }
        } else {
            Err(HttpHandlerError::before_handler_multipart_field_not_exist())
        }
    }
}

impl<T: TryFromPart> TryConvertFrom<Option<Vec<Part>>> for Option<T> {
    fn try_convert_from(value: Option<Vec<Part>>) -> Result<Self, HttpHandlerError> {
        match T::try_convert_from(value) {
            Ok(r) => Ok(Some(r)),
            Err(e) => match e {
                HttpHandlerError::BeforeHandler(BeforeHandlerError::MultipartError(
                    MultipartError::FieldNotExist,
                )) => Ok(None),
                _ => Err(e),
            },
        }
    }
}

impl<T: TryFromPart> TryConvertFrom<Option<Vec<Part>>> for Vec<T> {
    fn try_convert_from(value: Option<Vec<Part>>) -> Result<Self, HttpHandlerError> {
        if let Some(value) = value {
            Ok(value
                .into_iter()
                .filter_map(|x| T::try_from_part(x).ok())
                .collect())
        } else {
            Ok(vec![])
        }
    }
}

#[derive(Debug)]
pub struct MultiPartFile {
    pub file_name: Option<String>,
    pub temp_path: String,
    pub mime_type: Option<String>,
}

impl Drop for MultiPartFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.temp_path.as_str());
    }
}

impl TryFromPart for MultiPartFile {
    fn try_from_part(value: Part) -> Result<Self, HttpHandlerError> {
        if let Part::File(f) = value {
            Ok(f)
        } else {
            Err(HttpHandlerError::before_handler_multipart_incompatible_type("this is not a file"))
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
    fn try_from_request(req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
        match req._inner.body_mut() {
            Some(RequestBody::MultiPart(body)) => {
                Ok(Multipart(T::try_from_multipart_data_map(body)?))
            }
            _ => Err(HttpHandlerError::before_handler_incompatible_request_body_type()),
        }
    }
}
