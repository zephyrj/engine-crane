/*
 * Copyright (c):
 * 2023 zephyrj
 * zephyrj@protonmail.com
 *
 * This file is part of engine-crane.
 *
 * engine-crane is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * engine-crane is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
 */

use std::{error, fmt, io, result};
use std::fmt::{Display, Formatter};
use crate::traits::DataInterfaceError;

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
        write!(f, "{} - {}", self.kind.as_str(), self.details)
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::new(ErrorKind::IOError, format!("{}. {}", e.to_string(), e.kind().to_string()))
    }
}

impl From<fs_extra::error::Error> for Error {
    fn from(e: fs_extra::error::Error) -> Self {
        return match e.kind {
            fs_extra::error::ErrorKind::Io(io_error) => { Error::new(ErrorKind::IOError, format!("{}. {}", io_error.to_string(), io_error.kind().to_string())) }
            _ => { Error::new(ErrorKind::IOError, e.to_string()) }
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::new(ErrorKind::JsonDecodeError, e.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::new(ErrorKind::TomlDecodeError, e.to_string())
    }
}

impl From<crate::car::acd_utils::AcdError> for Error {
    fn from(e: crate::car::acd_utils::AcdError) -> Self {
        Error::new(ErrorKind::AcdError, e.to_string())
    }
}

impl From<DataInterfaceError> for Error {
    fn from(e: DataInterfaceError) -> Self {
        Error::new(ErrorKind::IOError, e.to_string())
    }
}


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    NoSuchCar,
    CarAlreadyExists,
    InvalidCar,
    InvalidUpdate,
    NotInstalled,
    IOError,
    JsonDecodeError,
    TomlDecodeError,
    AcdError,
    UpdateError,
    ArgumentError,
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
            ErrorKind::IOError => "io error",
            ErrorKind::JsonDecodeError => "json decode error",
            ErrorKind::TomlDecodeError => "toml decode error",
            ErrorKind::AcdError => "acd decode error",
            ErrorKind::ArgumentError => "argument error",
            ErrorKind::Uncategorized => "uncategorized error",
            ErrorKind::UpdateError => "update error"
        }
    }
}

#[derive(Debug)]
pub struct PropertyParseError {
    invalid_value: String
}

impl PropertyParseError {
    pub fn new(invalid_value: &str) -> PropertyParseError {
        PropertyParseError {
            invalid_value: String::from(invalid_value)
        }
    }
}

impl Display for PropertyParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown value '{}'", &self.invalid_value)
    }
}

impl error::Error for PropertyParseError {}
