/*
 * Copyright (c):
 * 2025 zephyrj
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

use std::collections::BTreeMap;
use crate::car::lut_utils::LutType;
use crate::car::structs::LutProperty;
use crate::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::error::{Result, Error, ErrorKind};

#[derive(Debug)]
pub struct PowerCurve {
    power_lut: LutProperty<i32, f64>,
}

impl PowerCurve {
    pub const SECTION_NAME: &'static str = "POWER_CURVE";

    pub fn update(&mut self, power_vec: Vec<(i32, f64)>) -> Vec<(i32, f64)> {
        self.power_lut.update(power_vec)
    }

    pub fn get_curve_data(&self) -> BTreeMap<i32, f64> {
        let lut_data = self.power_lut.to_vec();
        lut_data.into_iter().collect()
    }

    pub fn get_lut(&self) -> &LutType<i32, f64> {
        self.power_lut.get_type()
    }
}

impl MandatoryDataSection for PowerCurve {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let power_lut = match LutProperty::<i32, f64>::mandatory_from_ini(
            String::from("HEADER"),
            String::from(Self::SECTION_NAME),
            parent_data.ini_data(),
            parent_data.data_interface()) {
            Ok(lut) => {
                lut
            }
            Err(e) => {
                return Err(Error::new(ErrorKind::InvalidCar,
                                      format!("Failed to load power curve lut from ini. {}", e)));
            }
        };
        Ok(PowerCurve{ power_lut })
    }
}

impl CarDataUpdater for PowerCurve {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        self.power_lut.update_car_data(car_data)?;
        Ok(())
    }
}
