use thiserror::Error;

use crate::{error::MultipartError, request::ConvertError};

#[derive(Debug, Error)]
pub enum ParseHandlerParamError {
    #[error("{0}")]
    ConvertError(#[from] ConvertError),
    #[error("you have a missing param")]
    ParamNotExist,
    #[error("{0}")]
    MultipartError(#[from] MultipartError),
    #[error("expect a body in request!")]
    BodyNotExist
}


#[derive(Debug,Error)]
pub enum ParseHttpRequestError {
    #[error("")]
    ParseBodyError
}
