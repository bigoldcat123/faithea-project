use bytes::Bytes;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, InvalidHeaderValue};
use thiserror::Error;

use crate::{
    request::error::{ParseHandlerParamError, ParseHttpRequestError},
    response::{HttpResponseModifier, HttpResponseModifierFuture, ResponseBody},
};

/// Errors that occur during low-level body/multipart parsing.
#[derive(Debug, Error)]
// #[allow(dead_code)]
pub enum BodyParseError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid multipart boundary")]
    InvalidBoundary,
    #[error("unexpected EOF while reading body")]
    UnexpectedEof,
    #[error("invalid UTF-8 in body: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("{0}")]
    Other(String),
}

impl From<BodyParseError> for Error {
    fn from(e: BodyParseError) -> Self {
        Self::BeforeHandler(BeforeHandlerError::ParseHttpRequestError(
            ParseHttpRequestError::ParseBodyError(e),
        ))
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("this error is not supported 😼")]
    Unknown,
    #[error("after handler: {0}")]
    AfterHandler(#[from] ModifierError),
    #[error("BeforeHandlerError: {0}")]
    BeforeHandler(#[from] BeforeHandlerError),
    #[error("{0}")]
    InvalidJsonStr(#[from] serde_json::Error),
}
impl Error {
    // pub fn before_handler_incompatible_request_body_type() -> Self {
    //     log::error!("body type is incompatible");
    //     Self::BeforeHandler(BeforeHandlerError::IncompatibleBodyType)
    // }
    // pub fn before_handler_param_not_exist() -> Self {
    //     log::error!("param_not_exist");
    //     Self::BeforeHandler(BeforeHandlerError::ParamNotExist)
    // }
    // pub fn before_handler_empty_request_body() -> Self {
    //     log::error!("empty request body");
    //     Self::BeforeHandler(BeforeHandlerError::EmpeyRequestBody)
    // }
    // pub fn before_handler_multipart_field_not_exist() -> Self {
    //     log::error!("multipart field not exist");
    //     Self::BeforeHandler(BeforeHandlerError::MultipartError(
    //         MultipartError::FieldNotExist,
    //     ))
    // }
    // pub fn before_handler_multipart_incompatible_type<C: AsRef<str>>(cause: C) -> Self {
    //     log::error!("multipart incompatible type: {}", cause.as_ref());
    //     Self::BeforeHandler(BeforeHandlerError::MultipartError(
    //         MultipartError::IncompatibleType(cause.as_ref().to_string()),
    //     ))
    // }
    // pub fn before_handler_multipart_can_not_parse_from_part<C: AsRef<str>>(cause: C) -> Self {
    //     log::error!(
    //         "multipart cat not parse from part cause -> {}",
    //         cause.as_ref()
    //     );
    //     Self::BeforeHandler(BeforeHandlerError::MultipartError(
    //         MultipartError::CanNotParseFromPart(cause.as_ref().to_string()),
    //     ))
    // }
    pub fn after_handler_incompatible_body_type() -> Self {
        log::error!("after handler incompatible body type");
        Self::AfterHandler(ModifierError::IncompatibleBodyType)
    }

    pub fn after_handler_file_not_exists(file_path: String) -> Self {
        log::error!("{} file not exists", file_path);
        Self::AfterHandler(ModifierError::FileNotExists(file_path))
    }
}

// TODO add update errors that may happen before handler
#[derive(Debug, Error)]
pub enum BeforeHandlerError {
    #[error("ParseParamError: {0}")]
    ParseHandlerParamError(#[from] ParseHandlerParamError),

    #[error("ParseHttpRequestError: {0}")]
    ParseHttpRequestError(#[from] ParseHttpRequestError)
    // #[error("")]
    // ParamNotExist,
    // #[error("")]
    // EmpeyRequestBody,
    // #[error("")]
    // IncompatibleBodyType,
    // #[error("")]
    // MultipartError(MultipartError),
}
#[derive(Debug, Error)]
pub enum MultipartError {
    #[error("field not exit")]
    FieldNotExist,
    #[error("CanNotParseFromPart {0}")]
    CanNotParseFromPart(String),
    #[error("IncompatibleType {0}")]
    IncompatibleType(String),
}
#[derive(Debug, Error)]
pub enum ModifierError {
    #[error("InvalidHeaderValue")]
    InvalidHeaderValue,
    #[error("IncompatibleBodyType")]
    IncompatibleBodyType,
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("file not exist: {0}")]
    FileNotExists(String),
}
impl From<InvalidHeaderValue> for Error {
    fn from(_: InvalidHeaderValue) -> Self {
        Self::AfterHandler(ModifierError::InvalidHeaderValue)
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        log::error!("std::io::Error -> {}", value);
        Self::AfterHandler(ModifierError::IoError(value))
    }
}

// impl Display for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "")
//     }
// }
// impl std::error::Error for Error {}
impl HttpResponseModifier for Error {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut crate::response::HttpResponse,
    ) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            res.add_header(CONTENT_TYPE, "text/plain".parse()?);
            let b = format!("{}", self).as_bytes().to_vec();
            let b = Bytes::from(b);
            res.add_header(CONTENT_LENGTH, b.len().to_string().parse()?);
            res.set_body(ResponseBody::Simple(Some(b)));
            Ok(())
        })
    }
}
