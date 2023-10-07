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

use std::fmt::{Display, Formatter};
use std::str::FromStr;
use crate::car::Car;
use crate::ini_utils::Ini;
use crate::error::{Error, ErrorKind, PropertyParseError, Result};
use crate::ini_utils;


#[derive(Debug)]
pub struct CarIniData<'a> {
    car: &'a mut Car,
    ini_config: Ini,
}

impl<'a> CarIniData<'a> {
    const FILENAME: &'static str = "car.ini";

    pub fn from_car(car: &'a mut Car) -> Result<CarIniData<'a>> {
        let car_ini_data = match car.data_interface.get_original_file_data(CarIniData::FILENAME) {
            Ok(data_option) => match data_option {
                None => Err(Error::new(ErrorKind::InvalidCar,
                                       format!("missing {} data", CarIniData::FILENAME))),
                Some(data) => Ok(data)
            }
            Err(e) => Err(e)?
        }?;
        Ok(CarIniData {
            car,
            ini_config: Ini::load_from_string(String::from_utf8_lossy(car_ini_data.as_slice()).into_owned())
        })
    }

    pub fn version(&self) -> Option<CarVersion> {
        ini_utils::get_value(&self.ini_config, "HEADER", "VERSION")
    }

    pub fn set_version(&mut self, version: CarVersion) {
        ini_utils::set_value(&mut self.ini_config, "HEADER", "VERSION", version);
    }

    pub fn screen_name(&self) -> Option<String> {
        ini_utils::get_value(&self.ini_config, "INFO","SCREEN_NAME")
    }

    pub fn set_screen_name(&mut self, name: &str) {
        ini_utils::set_value(&mut self.ini_config, "INFO","SCREEN_NAME", name);
    }

    pub fn total_mass(&self) -> Option<u32> {
        ini_utils::get_value(&self.ini_config, "BASIC","TOTALMASS")
    }

    pub fn set_total_mass(&mut self, new_mass: u32) {
        ini_utils::set_value(&mut self.ini_config, "BASIC","TOTALMASS", new_mass);
    }

    pub fn default_fuel(&self) -> Option<u32> {
        ini_utils::get_value(&self.ini_config, "FUEL","FUEL")
    }

    pub fn max_fuel(&self) -> Option<u32> {
        ini_utils::get_value(&self.ini_config, "FUEL","MAX_FUEL")
    }

    pub fn fuel_consumption(&self) -> Option<f64> {
        ini_utils::get_value(&self.ini_config, "FUEL","CONSUMPTION")
    }

    pub fn set_fuel_consumption(&mut self, consumption: f64) {
        ini_utils::set_float(&mut self.ini_config, "FUEL","CONSUMPTION", consumption, 4);
    }

    pub fn clear_fuel_consumption(&mut self) {
        self.ini_config.remove_value("FUEL", "CONSUMPTION");
    }

    pub fn write(&'a mut self) -> Result<()> {
        let data_interface = self.car.mut_data_interface();
        data_interface.update_file_data("car.ini", self.ini_config.to_bytes());
        data_interface.write()?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum CarVersion {
    One,
    Two,
    CspExtendedPhysics
}

impl Default for CarVersion {
    fn default() -> Self {
        CarVersion::One
    }
}

impl CarVersion {
    pub const VERSION_1 :&'static str = "1";
    pub const VERSION_2 :&'static str = "2";
    pub const CSP_EXTENDED_2 : &'static str = "extended-2";

    fn as_str(&self) -> &'static str {
        match self {
            CarVersion::One => CarVersion::VERSION_1,
            CarVersion::Two => CarVersion::VERSION_2,
            CarVersion::CspExtendedPhysics => CarVersion::CSP_EXTENDED_2
        }
    }
}

impl FromStr for CarVersion {
    type Err = PropertyParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            CarVersion::VERSION_1 => Ok(CarVersion::One),
            CarVersion::VERSION_2 => Ok(CarVersion::Two),
            CarVersion::CSP_EXTENDED_2 => Ok(CarVersion::CspExtendedPhysics),
            _ => Err(PropertyParseError::new(s))
        }
    }
}

impl Display for CarVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
