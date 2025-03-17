use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum SizeUnit {
    KibiByte(f64),
    MebiByte(f64),
    GibiByte(f64),
}

impl SizeUnit {
    pub const KIBIBYTE: u64 = 1024;
    pub const MEBIBYTE: u64 = 1024 * SizeUnit::KIBIBYTE;
    pub const GIBIBYTE: u64 = 1024 * SizeUnit::MEBIBYTE;

    /// Returns current represented value as bytes
    pub fn as_bytes(self) -> f64 {
        match self {
            Self::KibiByte(v) => v * SizeUnit::KIBIBYTE as f64,
            Self::MebiByte(v) => v * SizeUnit::MEBIBYTE as f64,
            Self::GibiByte(v) => v * SizeUnit::GIBIBYTE as f64,
        }
    }
}

impl<T: Into<f64>> From<T> for SizeUnit {
    fn from(value: T) -> Self {
        let value = value.into();

        if value > Self::GIBIBYTE as f64 {
            SizeUnit::GibiByte(value / SizeUnit::GIBIBYTE as f64)
        } else if value > Self::MEBIBYTE as f64 {
            SizeUnit::MebiByte(value / SizeUnit::MEBIBYTE as f64)
        } else {
            SizeUnit::KibiByte(value / SizeUnit::KIBIBYTE as f64)
        }
    }
}

impl Display for SizeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (value, type_str) = match *self {
            Self::KibiByte(v) => (v, "KiB"),
            Self::MebiByte(v) => (v, "MiB"),
            Self::GibiByte(v) => (v, "GiB"),
        };

        match f.precision() {
            Some(precision) => f.write_fmt(format_args!(
                "{:.precision$} {}",
                value,
                type_str,
                precision = precision
            )),
            None => f.write_fmt(format_args!("{} {}", value, type_str)),
        }
    }
}
