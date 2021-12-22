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
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Operation failed; {} - {}", self.kind.as_str(), self.details)
    }
}

impl error::Error for Error {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    InvalidCar,
    NotInstalled,
    Uncategorized
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::InvalidCar => "invalid car",
            ErrorKind::NotInstalled => "not installed",
            ErrorKind::Uncategorized => "uncategorized error"
        }
    }
}
