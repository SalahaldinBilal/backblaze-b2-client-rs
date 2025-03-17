use std::{collections::HashMap, str::FromStr};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Serialize;

use crate::error::IntoHeaderMapError;

pub trait IntoHeaderMap: Sized + Serialize {
    fn into_header_map(self) -> Result<HeaderMap, IntoHeaderMapError> {
        let serialized_object = serde_json::to_value(self)
            .map_err(|err| IntoHeaderMapError::SerializationFailed(err))?;

        match serialized_object {
            serde_json::Value::Object(object) => object
                .into_iter()
                .filter_map(|(key, value)| {
                    let value = match value {
                        serde_json::Value::Null => return None,
                        serde_json::Value::String(value) => value,
                        val => val.to_string(),
                    };

                    let header_name = match HeaderName::from_str(&key) {
                        Ok(header_name) => header_name,
                        Err(_) => return Some(Err(IntoHeaderMapError::InvalidHeaderName(key))),
                    };

                    let header_value = match HeaderValue::from_str(&value) {
                        Ok(header_value) => header_value,
                        Err(_) => return Some(Err(IntoHeaderMapError::InvalidHeaderValue(value))),
                    };

                    Some(Ok((header_name, header_value)))
                })
                .collect(),
            _ => return Err(IntoHeaderMapError::InvalidObject),
        }
    }
}

impl<A: Sized + Serialize, B: Sized + Serialize> IntoHeaderMap for HashMap<A, B> {}
