use std::fmt::{Display, Formatter};
use std::str::FromStr;
use crate::assetto_corsa::drivetrain::get_mandatory_field;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::{Ini, IniUpdater};
use crate::assetto_corsa::traits::{CarDataFile, MandatoryDataSection};
use crate::assetto_corsa::error::{PropertyParseError, Result};


#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriveType {
    RWD,
    FWD,
    AWD
}

impl DriveType {
    pub const RWD_VALUE: &'static str = "RWD";
    pub const FWD_VALUE: &'static str = "FWD";
    pub const AWD_VALUE: &'static str = "AWD";

    pub fn as_str(&self) -> &'static str {
        match self {
            DriveType::RWD => { DriveType::RWD_VALUE }
            DriveType::FWD => { DriveType::FWD_VALUE }
            DriveType::AWD => { DriveType::AWD_VALUE }
        }
    }

    pub fn mechanical_efficiency(&self) -> f64 {
        match self {
            DriveType::RWD => { 0.85 }
            DriveType::FWD => { 0.9 }
            DriveType::AWD => { 0.75 }
        }
    }
}

impl FromStr for DriveType {
    type Err = PropertyParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            DriveType::RWD_VALUE => Ok(DriveType::RWD),
            DriveType::FWD_VALUE => Ok(DriveType::FWD),
            DriveType::AWD_VALUE => Ok(DriveType::AWD),
            _ => Err(PropertyParseError::new(s))
        }
    }
}

impl Display for DriveType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
pub struct Traction {
    pub drive_type: DriveType
}

impl MandatoryDataSection for Traction {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        Ok(Traction{
            drive_type: get_mandatory_field(parent_data.ini_data(), "TRACTION", "TYPE")?
        })
    }
}

impl IniUpdater for Traction {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_value(ini_data, "TRACTION", "TYPE", &self.drive_type);
        Ok(())
    }
}
