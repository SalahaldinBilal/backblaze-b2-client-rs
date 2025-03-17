use core::fmt;
use std::{error::Error, num::NonZeroU16};

use serde::{Deserialize, Serialize};

use crate::definitions::shared::B2KeyCapability;

#[derive(Debug)]
pub enum B2Error {
    // NotAuthenticated,
    JsonParseError(serde_json::Error),
    RequestError(B2RequestError),
    RequestSendError(reqwest::Error),
    MissingCapability(B2KeyCapability),
    InvalidHeaders(IntoHeaderMapError),
}

impl Error for B2Error {}

impl fmt::Display for B2Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "B2 request encountered an error, ")?;

        match self {
            Self::JsonParseError(err) => write!(f, "Failed to parse JSON: {}", err),
            Self::RequestError(err) => write!(f, "Request returned an error: {}", err),
            Self::RequestSendError(err) => write!(f, "Failed to send request: {}", err),
            Self::MissingCapability(capability) => {
                write!(f, "Client is missing capability: {}", capability)
            }
            Self::InvalidHeaders(err) => write!(f, "Invalid headers passed: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum IntoHeaderMapError {
    InvalidObject,
    SerializationFailed(serde_json::Error),
    InvalidHeaderName(String),
    InvalidHeaderValue(String),
}

impl Error for IntoHeaderMapError {}

impl fmt::Display for IntoHeaderMapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidObject => write!(
                f,
                "Object that implemented `IntoHeaderMap` does not serialize into an object."
            ),
            Self::SerializationFailed(err) => write!(f, "Failed to serialize object: {}", err),
            Self::InvalidHeaderName(name) => write!(f, "[{}] is not a valid header name.", name),
            Self::InvalidHeaderValue(value) => {
                write!(f, "[{}] is not a valid header value.", value)
            }
        }
    }
}

impl From<IntoHeaderMapError> for B2Error {
    fn from(error: IntoHeaderMapError) -> Self {
        B2Error::InvalidHeaders(error)
    }
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct B2RequestError {
    pub status: NonZeroU16,
    pub code: String,
    pub message: Option<String>,
}

impl fmt::Display for B2RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).expect("Valid format"))
    }
}

impl Error for B2RequestError {}
