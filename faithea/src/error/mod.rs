use std::fmt::Display;

use bytes::Bytes;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, InvalidHeaderValue};

use crate::response::{HttpResponseModifier, ResponseBody};

#[derive(Debug)]
pub enum Error {
    Unknown,
    AfterHandler(ModifierError),
    BeforeHandler(BeforeHandlerError),
    InvalidJsonStr(serde_json::Error),
}
impl Error {
    pub fn before_handler_incompatible_request_body_type() -> Self {
        Self::BeforeHandler(BeforeHandlerError::IncompatibleBodyType)
    }
    pub fn before_handler_invalid_param<C: AsRef<str>>(cause: C) -> Self {
        Self::BeforeHandler(BeforeHandlerError::InvalidParam(cause.as_ref().to_string()))
    }
    pub fn before_handler_empty_request_body() -> Self {
        Self::BeforeHandler(BeforeHandlerError::EmpeyRequestBody)
    }
    pub fn before_handler_multipart_field_not_exist() -> Self {
        Self::BeforeHandler(BeforeHandlerError::MultipartError(
            MultipartError::FieldNotExist,
        ))
    }
    pub fn before_handler_multipart_incompatible_type<C: AsRef<str>>(cause: C) -> Self {
        Self::BeforeHandler(BeforeHandlerError::MultipartError(
            MultipartError::IncompatibleType(cause.as_ref().to_string()),
        ))
    }
    pub fn before_handler_multipart_can_not_parse_from_part<C: AsRef<str>>(cause: C) -> Self {
        Self::BeforeHandler(BeforeHandlerError::MultipartError(
            MultipartError::CanNotParseFromPart(cause.as_ref().to_string()),
        ))
    }
    pub fn after_handler_incompatible_body_type() -> Self {
        Self::AfterHandler(ModifierError::IncompatibleBodyType)
    }

    pub fn after_handler_file_not_exists(file_path: String) -> Self {
        Self::AfterHandler(ModifierError::FileNotExists(file_path))
    }
}
#[derive(Debug)]
pub enum BeforeHandlerError {
    InvalidParam(String),
    EmpeyRequestBody,
    IncompatibleBodyType,
    MultipartError(MultipartError),
}
#[derive(Debug)]
pub enum MultipartError {
    FieldNotExist,
    CanNotParseFromPart(String),
    IncompatibleType(String),
}
#[derive(Debug)]
pub enum ModifierError {
    InvalidHeaderValue,
    IncompatibleBodyType,
    IoError(std::io::Error),
    FileNotExists(String),
}
impl From<InvalidHeaderValue> for Error {
    fn from(_: InvalidHeaderValue) -> Self {
        Self::AfterHandler(ModifierError::InvalidHeaderValue)
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidJsonStr(value)
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::AfterHandler(ModifierError::IoError(value))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
impl std::error::Error for Error {}
impl HttpResponseModifier for Error {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut crate::response::HttpResponse,
    ) -> std::pin::Pin<
        Box<
            dyn Future<Output = Result<(), crate::handler::types::HttpHandlerError>>
                + 'a
                + Send
                + Sync,
        >,
    > {
        Box::pin(async move {
            res.add_header(CONTENT_TYPE, "text/plain".parse()?);
            let b = format!("{:?}", self).as_bytes().to_vec();
            let b = Bytes::from(b);
            res.add_header(CONTENT_LENGTH, b.len().to_string().parse()?);
            res.set_body(ResponseBody::Simple(b));
            Ok(())
        })
    }
}
