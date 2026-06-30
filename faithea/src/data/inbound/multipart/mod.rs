pub mod parser;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{
    error::MultipartError,
    request::{HttpRequest, RequestBody, TryFromRequest, error::ParseHandlerParamError},
};

pub type MultipartDataMap = HashMap<String, Vec<Part>>;

/// macro generate!
pub trait TryFromMultipartDataMap: Sized {
    fn try_from_multipart_data_map(data: &mut MultipartDataMap) -> Result<Self, MultipartError>;
}

#[derive(Debug)]
pub enum Part {
    Lit(String),
    File(MultiPartFile),
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
                fn try_from_parts(value: Option<Vec<Part>>) -> Result<Self,$crate::error::MultipartError>{
                    use $crate::error::MultipartError;
                    if let Some(mut v) = value && let Some(p) = v.pop() {
                        TryFromPart::try_from_part(p)
                        // Ok(l.parse::<Self>().map_err(|x| $crate::handler::types::HttpHandlerError::before_handler_multipart_can_not_parse_from_part(x.to_string()))?)
                    }else {
                        // let e = $crate::handler::types::HttpHandlerError::before_handler_multipart_incompatible_type(format!("{} not compatiable to transform part to MultiPartFile",stringify!($t)));
                         let e = MultipartError::FieldNotExist;
                        Err(e)
                    }
                }
            }

            impl TryFromPart for $t {
                fn try_from_part(value: Part) -> Result<Self,$crate::error::MultipartError>{
                    use $crate::error::MultipartError;
                    if let Part::Lit(l) = value {
                        Ok(l.parse::<Self>().map_err(|e|
                                MultipartError::CanNotParseFromPart(e.to_string())
                        )?)
                        // Ok(l.parse::<Self>().map_err(|x| $crate::handler::types::HttpHandlerError::before_handler_multipart_can_not_parse_from_part(x.to_string()))?)
                    }else {
                        // let e = $crate::handler::types::HttpHandlerError::before_handler_multipart_incompatible_type(format!("{} not compatiable to transform part to MultiPartFile",stringify!($t)));
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

// impl<T: TryFromParts> TryConvertFrom<Option<Vec<Part>>> for T {
//     type Error = HttpHandlerError;
//     fn try_convert_from(value: Option<Vec<Part>>) -> Result<Self, HttpHandlerError> {
//         T::try_from_parts(value)
//             .map_err(Into::<ParseHandlerParamError>::into)
//             .map_err(Into::<BeforeHandlerError>::into)
//             .map_err(Into::into)
//         // if let Some(value) = value.pop() {

//         // } else {
//         //     let before = Into::<BeforeHandlerError>::into(
//         //         ParseHandlerParamError::MultipartError(MultipartError::FieldNotExist),
//         //     );
//         //     let err = Into::<Self::Error>::into(before);
//         //     Err(err)
//         // }
//     }
// }

// impl<T: TryFromParts> TryConvertFrom<Option<Vec<Part>>> for Option<T> {
//     type Error = HttpHandlerError;
//     fn try_convert_from(value: Option<Vec<Part>>) -> Result<Self, HttpHandlerError> {
//         match T::try_convert_from(value) {
//             Ok(r) => Ok(Some(r)),
//             Err(e) => match e {
//                 HttpHandlerError::BeforeHandler(BeforeHandlerError::ParseHandlerParamError(
//                     ParseHandlerParamError::ParamNotExist,
//                 )) => Ok(None),
//                 _ => Err(e),
//             },
//         }
//     }
// }

impl<T: TryFromParts + TryFromPart> TryFromParts for Option<T> {
    fn try_from_parts(parts: Option<Vec<Part>>) -> Result<Self, MultipartError> {
        match parts {
            Some(mut p) => Ok(Some(TryFromPart::try_from_part(p.pop().unwrap())?)),
            None => Ok(None),
        }
    }
}

// impl<T: TryFromParts> TryConvertFrom<Option<Vec<Part>>> for Vec<T> {
//     type Error = HttpHandlerError;
//     fn try_convert_from(value: Option<Vec<Part>>) -> Result<Self, HttpHandlerError> {
//         if let Some(value) = value {
//             Ok(value
//                 .into_iter()
//                 .filter_map(|x| T::try_from_parts(Some(vec![x])).ok())
//                 .collect())
//         } else {
//             Ok(vec![])
//         }
//     }
// }
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
    fn try_from_part(value: Part) -> Result<Self, MultipartError> {
        if let Part::File(f) = value {
            Ok(f)
        } else {
            Err(MultipartError::IncompatibleType("Expect a File!".into()))
        }
    }
}
impl TryFromParts for MultiPartFile {
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
        match req._inner.body_mut() {
            Some(RequestBody::MultiPart(body)) => {
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
    fn convert_to_optiong() {
        let p = Some(vec![Part::Lit("hello".into())]);
        let a: Option<String> = TryFromParts::try_from_parts(p).unwrap();
        assert_eq!(a, Some("hello".to_string()))
    }
}
