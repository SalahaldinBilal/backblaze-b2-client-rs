use std::{error::Error, fmt::Display};

pub trait IsValid {
    /// Whether the current object is valid or not
    fn is_valid(&self) -> Result<(), InvalidValue>;
}

#[derive(Debug)]
pub struct InvalidValue {
    pub object_name: String,
    pub value_name: String,
    pub value_as_string: String,
    pub expected: String,
}

impl Display for InvalidValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Passed {} value is valid, for field {} got [{}] expected [{}].",
            self.object_name, self.value_name, self.value_as_string, self.expected
        )
    }
}

impl Error for InvalidValue {}
