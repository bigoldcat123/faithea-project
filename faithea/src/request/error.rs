use thiserror::Error;

use crate::{
    data::inbound::multipart::MultipartError, error::BodyParseError, request::ConvertError,
};

#[derive(Debug, Error)]
pub enum ParseHandlerParamError {
    #[error("{0}")]
    ConvertError(#[from] ConvertError),

    #[error("param not exist")]
    ParamNotExist,

    #[error("{0}")]
    MultipartError(#[from] MultipartError),

    #[error("expect a body in request")]
    BodyNotExist,
}

#[derive(Debug, Error)]
pub enum ParseHttpRequestError {
    #[error("parse body error: {0}")]
    ParseBodyError(#[from] BodyParseError),
}
