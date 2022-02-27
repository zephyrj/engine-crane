use std::{error, fmt, result};
use std::fmt::{Display, Formatter};
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error{
    kind: ErrorKind,
    details: String
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, details: String) -> Error {
        Error{ kind, details }
    }
    pub(crate) fn from_io_error(io_err: std::io::Error, failed_operation: &str) -> Error {
        Error{ kind: ErrorKind::IOError,
               details: format!("{}. {}", failed_operation, io_err.to_string()) }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Operation failed; {} - {}", self.kind.as_str(), self.details)
    }
}

impl error::Error for Error {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    NoSuchCar,
    CarAlreadyExists,
    InvalidCar,
    InvalidUpdate,
    NotInstalled,
    InvalidEngineMetadata,
    InvalidEngineTurboController,
    IOError,
    Uncategorized
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::NoSuchCar => "car doesn't exist",
            ErrorKind::CarAlreadyExists => "car already exists",
            ErrorKind::InvalidCar => "invalid car",
            ErrorKind::InvalidUpdate => "requested update is invalid",
            ErrorKind::NotInstalled => "not installed",
            ErrorKind::InvalidEngineMetadata => "engine metadata is invalid",
            ErrorKind::InvalidEngineTurboController => "engine turbo controller is invalid",
            ErrorKind::IOError => "io error",
            ErrorKind::Uncategorized => "uncategorized error"
        }
    }
}

#[derive(Debug)]
pub struct FieldParseError {
    invalid_value: String
}

impl FieldParseError {
    pub fn new(invalid_value: &str) -> FieldParseError {
        FieldParseError {
            invalid_value: String::from(invalid_value)
        }
    }
}

impl Display for FieldParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown value '{}'", &self.invalid_value)
    }
}

impl error::Error for FieldParseError {}

