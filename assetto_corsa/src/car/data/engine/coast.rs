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
use crate::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::error::{PropertyParseError, Result};
use crate::ini_utils;


#[derive(Debug)]
pub struct CoastCurve {
    curve_data_source: CoastSource,
    reference_rpm: i32,
    torque: i32,
    non_linearity: f64
}

impl CoastCurve {
    pub fn new_from_coast_ref(reference_rpm: i32, torque: i32, non_linearity: f64) -> CoastCurve {
        CoastCurve {
            curve_data_source: CoastSource::FromCoastRef,
            reference_rpm,
            torque,
            non_linearity
        }
    }
}

impl MandatoryDataSection for CoastCurve {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let curve_data_source: CoastSource = ini_utils::get_mandatory_property(ini_data, "HEADER", "COAST_CURVE")?;

        let section_name = curve_data_source.get_section_name();
        Ok(CoastCurve{
            curve_data_source,
            reference_rpm: ini_utils::get_mandatory_property(ini_data, section_name, "RPM")?,
            torque: ini_utils::get_mandatory_property(ini_data, section_name, "TORQUE")?,
            non_linearity: ini_utils::get_mandatory_property(ini_data, section_name, "NON_LINEARITY")?,
        })
    }
}

impl CarDataUpdater for CoastCurve {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        return match self.curve_data_source {
            CoastSource::FromCoastRef => {
                let section_name = self.curve_data_source.get_section_name();
                ini_utils::set_value(ini_data, "HEADER", "COAST_CURVE", &self.curve_data_source);
                ini_utils::set_value(ini_data, section_name, "RPM", self.reference_rpm);
                ini_utils::set_value(ini_data, section_name, "TORQUE", self.torque);
                ini_utils::set_float(ini_data, section_name, "NON_LINEARITY", self.non_linearity, 2);
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CoastSource {
    FromCoastRef
}

impl CoastSource {
    pub const FROM_COAST_REF_VALUE: &'static str = "FROM_COAST_REF";
    pub const COAST_REF_SECTION_NAME: &'static str = "COAST_REF";

    pub fn as_str(&self) -> &'static str {
        match self { CoastSource::FromCoastRef => CoastSource::FROM_COAST_REF_VALUE }
    }

    pub fn get_section_name(&self) -> &'static str {
        match self { CoastSource::FromCoastRef => CoastSource::COAST_REF_SECTION_NAME }
    }
}

impl FromStr for CoastSource {
    type Err = PropertyParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            CoastSource::FROM_COAST_REF_VALUE => Ok(CoastSource::FromCoastRef),
            _ => Err(PropertyParseError::new(s))
        }
    }
}

impl Display for CoastSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
