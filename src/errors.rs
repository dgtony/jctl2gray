//! Custom errors module

use std::fmt;
use std::result::Result as StdResult;

use serde_json::Error as SerdeJSONErr;
use std::error::Error as StdErr;
use std::io::Error as IOErr;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    IOError(String),
    SerdeParsing(String),
    InsufficientLogLevel,
    NoMessage,
    InternalError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IOError(ref reason) => write!(f, "[IO] {}", reason),
            Error::SerdeParsing(ref reason) => write!(f, "[JSON parsing] {}", reason),
            Error::InternalError(ref reason) => write!(f, "[Internal] {}", reason),
            ref e @ Error::InsufficientLogLevel => write!(f, "{}", e.to_string()),
            ref e @ Error::NoMessage => write!(f, "{}", e.to_string()),
        }
    }
}

impl StdErr for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IOError(ref reason) => reason.as_str(),
            Error::SerdeParsing(ref reason) => reason.as_str(),
            Error::InternalError(ref reason) => reason.as_str(),
            Error::InsufficientLogLevel => "insufficient log level",
            Error::NoMessage => "no message found",
        }
    }
}

impl From<IOErr> for Error {
    fn from(e: IOErr) -> Self {
        Error::IOError(e.to_string())
    }
}

impl From<SerdeJSONErr> for Error {
    fn from(e: SerdeJSONErr) -> Error {
        Error::SerdeParsing(e.to_string())
    }
}
