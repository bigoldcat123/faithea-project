use crate::HttpHeader;

pub(crate) enum ContentType<'a> {
    ApplicationJson,
    MultipartFormData(&'a str),
    Simple,
}
impl<'a> TryFrom<&'a HttpHeader> for ContentType<'a> {
    type Error = String;
    fn try_from(headers: &'a HttpHeader) -> Result<Self, Self::Error> {
        if let Some(content_type) = headers.get("content-type") {
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
fn get_multipart_boundary(headers: &HttpHeader) -> Option<&str> {
    if let Some(b) = headers.get("content-type")
        && let Some((_, b)) = b.split_once(";")
        && let Some((_, b)) = b.split_once("=")
    {
        return Some(b);
    }
    None
}
