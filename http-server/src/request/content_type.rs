use http::{HeaderMap, HeaderValue, header::CONTENT_TYPE};

use crate::{ map_str};

pub(crate) enum ContentType<'a> {
    ApplicationJson,
    MultipartFormData(&'a str),
    Simple,
}



impl<'a> TryFrom<&'a HeaderMap<HeaderValue>> for ContentType<'a> {
    type Error = String;
    fn try_from(headers: &'a HeaderMap<HeaderValue>) -> Result<Self, Self::Error> {
        if let Some(content_type) = headers.get(CONTENT_TYPE) {
            let content_type = content_type.to_str().map_err(map_str!())?;
            if content_type.starts_with("application/json") {
                Ok(Self::ApplicationJson)
            } else if content_type.starts_with("multipart/form-data") {
                if let Some(boundary) = get_multipart_boundary(headers) {
                    Ok(Self::MultipartFormData(boundary))
                } else {
                    Err("no boundary found in your multipart/form-data header".to_string())
                }
            } else {
                Ok(Self::Simple)
            }
        } else {
            Ok(Self::Simple)
        }
    }
}


// impl<'a> TryFrom<&'a HttpHeader> for ContentType<'a> {
//     type Error = String;
//     fn try_from(headers: &'a HttpHeader) -> Result<Self, Self::Error> {
//         if let Some(content_type) = headers.get("content-type") {
//             if content_type.starts_with("application/json") {
//                 Ok(Self::ApplicationJson)
//             } else if content_type.starts_with("multipart/form-data") {
//                 if let Some(boundary) = get_multipart_boundary(headers) {
//                     Ok(Self::MultipartFormData(boundary))
//                 } else {
//                     Err("no boundary found in your multipart/form-data header".to_string())
//                 }
//             } else {
//                 Ok(Self::Simple)
//             }
//         } else {
//             Ok(Self::Simple)
//         }
//     }
// }
fn get_multipart_boundary(headers: &HeaderMap<HeaderValue>) -> Option<&str> {
    if let Some(b) = headers.get(CONTENT_TYPE)
        && let Some((_, b)) = b.to_str().ok()?.split_once(";")
        && let Some((_, b)) = b.split_once("=")
    {
        return Some(b);
    }
    None
}
