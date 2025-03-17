use core::fmt;
use std::error::Error;

use crate::{error::B2Error, util::InvalidValue};

#[derive(Debug)]
pub enum FileUploadError {
    Aborted,
    AlreadyStarted,
    FailedToReadFile(std::io::Error),
    RequestError(B2Error),
    InvalidOptions(InvalidValue),
}

impl Error for FileUploadError {}

impl fmt::Display for FileUploadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "B2 upload failed, ")?;

        match self {
            Self::Aborted => write!(f, "Request was aborted."),
            Self::AlreadyStarted => write!(f, "Already started file upload."),
            Self::FailedToReadFile(err) => write!(f, "Failed to read file to upload: {}", err),
            Self::RequestError(err) => write!(f, "{}", err),
            Self::InvalidOptions(err) => write!(f, "{}", err),
        }
    }
}

impl From<B2Error> for FileUploadError {
    fn from(value: B2Error) -> Self {
        FileUploadError::RequestError(value)
    }
}

impl From<InvalidValue> for FileUploadError {
    fn from(value: InvalidValue) -> Self {
        FileUploadError::InvalidOptions(value)
    }
}

impl From<std::io::Error> for FileUploadError {
    fn from(value: std::io::Error) -> Self {
        FileUploadError::FailedToReadFile(value)
    }
}
