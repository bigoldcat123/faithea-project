pub mod parser;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};


use crate::{
    TryConvertFrom,
    handler::types::HttpHandlerError,
    request::{HttpRequest, RequestBody},
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
macro_rules! impl_try_from_part_for_parse_from_str {
    ($($t:ty),*) => {
        $(
            impl TryFrom<Part> for $t {
                type Error = $crate::handler::types::HttpHandlerError;
                fn try_from(value: Part) -> Result<Self, Self::Error> {
                    if let Part::Lit(l) = value {
                        Ok(l.parse::<Self>().map_err(|x| Self::Error::before_handler_multipart_can_not_parse_from_part(x.to_string()))?)
                    }else {
                        let e = Self::Error::before_handler_multipart_incompatible_type(format!("{} not compatiable to transform part to MultiPartFile",stringify!($t)));
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

impl<T: TryFrom<Part, Error = HttpHandlerError>> TryConvertFrom<Vec<Part>> for T {
    fn try_convert_from(mut value: Vec<Part>) -> Result<Self, HttpHandlerError> {
        if let Some(value) = value.pop() {
            value.try_into()
        } else {
            Err(HttpHandlerError::before_handler_multipart_field_not_exist())
        }
    }
}

impl<T: TryFrom<Part>> TryConvertFrom<Vec<Part>> for Vec<T> {
    fn try_convert_from(value: Vec<Part>) -> Result<Self, HttpHandlerError> {
        Ok(value
            .into_iter()
            .filter_map(|x| T::try_from(x).ok())
            .collect())
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

impl TryFrom<Part> for MultiPartFile {
    type Error = HttpHandlerError;
    fn try_from(value: Part) -> Result<Self, Self::Error> {
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

impl<T: TryFromMultipartDataMap> TryFrom<&mut HttpRequest> for Multipart<T> {
    type Error = HttpHandlerError;
    fn try_from(req: &mut HttpRequest) -> Result<Self, Self::Error> {
        match req._inner.body_mut() {
            Some(RequestBody::MultiPart(body)) => {
                Ok(Multipart(T::try_from_multipart_data_map(body)?))
            }
            _ => Err(HttpHandlerError::before_handler_incompatible_request_body_type()),
        }
    }
}
