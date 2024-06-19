/*
 * Copyright (c):
 * 2024 zephyrj
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

use std::fmt::{Display, Formatter};
use std::str::FromStr;
use crate::car::data::drivetrain::get_mandatory_field;
use crate::ini_utils;
use crate::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::error::{PropertyParseError, Result};


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DriveType {
    RWD,
    FWD,
    AWD,
    AWD2
}

impl DriveType {
    pub const RWD_VALUE: &'static str = "RWD";
    pub const FWD_VALUE: &'static str = "FWD";
    pub const AWD_VALUE: &'static str = "AWD";
    pub const AWD2_VALUE: &'static str = "AWD2";

    pub fn as_str(&self) -> &'static str {
        match self {
            DriveType::RWD => { DriveType::RWD_VALUE }
            DriveType::FWD => { DriveType::FWD_VALUE }
            DriveType::AWD => { DriveType::AWD_VALUE }
            DriveType::AWD2 => { DriveType::AWD2_VALUE }
        }
    }

    pub fn mechanical_efficiency(&self) -> f64 {
        match self {
            DriveType::RWD => { 0.85 }
            DriveType::FWD => { 0.9 }
            DriveType::AWD => { 0.75 }
            DriveType::AWD2 => { 0.75 }
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
            DriveType::AWD2_VALUE => Ok(DriveType::AWD2),
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

impl CarDataUpdater for Traction {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        ini_utils::set_value(car_data.mut_ini_data(), "TRACTION", "TYPE", &self.drive_type);
        Ok(())
    }
}
