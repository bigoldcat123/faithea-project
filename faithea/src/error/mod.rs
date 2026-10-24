use bytes::Bytes;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, InvalidHeaderValue};
use thiserror::Error;

use crate::{
    request::error::{ParseHandlerParamError, ParseHttpRequestError},
    response::{HttpResponseModifier, HttpResponseModifierFuture, ResponseBody},
};

/// Errors that occur during low-level body/multipart parsing.
#[derive(Debug, Error)]
pub enum BodyParseError {
    #[error("{resource} exceeds limit of {limit}")]
    LimitExceeded {
        resource: &'static str,
        limit: usize,
    },

    #[error("request body read timed out")]
    Timeout,

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

impl From<String> for BodyParseError {
    fn from(s: String) -> Self {
        BodyParseError::Other(s)
    }
}

impl From<std::string::FromUtf8Error> for BodyParseError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        BodyParseError::Other(e.to_string())
    }
}

/// Top-level error type for the HTTP framework.
///
/// Distinguishes between errors that occur before the handler is invoked
/// (request parsing, parameter extraction) and after the handler returns
/// (response modification).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("unknown error")]
    Unknown,

    #[error("after handler error: {0}")]
    AfterHandler(#[from] ModifierError),

    #[error("before handler error: {0}")]
    BeforeHandler(#[from] BeforeHandlerError),
}

impl Error {
    pub fn after_handler_incompatible_body_type() -> Self {
        Self::AfterHandler(ModifierError::IncompatibleBodyType)
    }

    pub fn after_handler_file_not_exists(file_path: String) -> Self {
        Self::AfterHandler(ModifierError::FileNotExists(file_path))
    }
}

/// Errors that occur before the request handler is invoked.
#[derive(Debug, Error)]
pub enum BeforeHandlerError {
    #[error("parse param error: {0}")]
    ParseHandlerParamError(#[from] ParseHandlerParamError),

    #[error("parse http request error: {0}")]
    ParseHttpRequestError(#[from] ParseHttpRequestError),
}

/// Errors that occur after the handler returns, during response modification.
#[derive(Debug, Error)]
pub enum ModifierError {
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error("incompatible body type")]
    IncompatibleBodyType,

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("file not exist: {0}")]
    FileNotExists(String),
}

// Note: The following `From` implementations must be kept as manual impls
// (rather than using thiserror's `#[from]`) because Rust's `?` operator only
// performs a single `From` conversion — it cannot chain through multiple layers.
// These provide direct conversion paths for types that appear in multiple
// error variants (e.g., `std::io::Error` exists in both `BodyParseError::Io`
// and `ModifierError::IoError`).

impl From<InvalidHeaderValue> for Error {
    fn from(e: InvalidHeaderValue) -> Self {
        Self::AfterHandler(ModifierError::InvalidHeaderValue(e))
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::AfterHandler(ModifierError::IoError(e))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::AfterHandler(ModifierError::JsonError(e))
    }
}

// Direct conversions for leaf error types to simplify `.map_err(Into::into)` chains.
// These bypass the intermediate BeforeHandlerError wrapper for convenience.
impl From<ParseHandlerParamError> for Error {
    fn from(e: ParseHandlerParamError) -> Self {
        Self::BeforeHandler(BeforeHandlerError::ParseHandlerParamError(e))
    }
}

impl From<ParseHttpRequestError> for Error {
    fn from(e: ParseHttpRequestError) -> Self {
        Self::BeforeHandler(BeforeHandlerError::ParseHttpRequestError(e))
    }
}

impl HttpResponseModifier for Error {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut crate::response::HttpResponse,
    ) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            log::error!("HTTP error: {}", self);

            res.add_header(CONTENT_TYPE, "text/plain".parse()?);
            let b = format!("{}", self).as_bytes().to_vec();
            let b = Bytes::from(b);
            res.add_header(CONTENT_LENGTH, b.len().to_string().parse()?);
            res.set_body(ResponseBody::Simple(Some(b)));
            Ok(())
        })
    }
}
