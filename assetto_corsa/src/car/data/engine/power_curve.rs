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

use crate::car::structs::LutProperty;
use crate::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::error::{Result, Error, ErrorKind};

#[derive(Debug)]
pub struct PowerCurve {
    power_lut: LutProperty<i32, f64>,
}

impl PowerCurve {
    pub fn update(&mut self, power_vec: Vec<(i32, f64)>) -> Result<Vec<(i32, f64)>> {
        Ok(self.power_lut.update(power_vec))
    }
}

impl MandatoryDataSection for PowerCurve {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let power_lut = match LutProperty::<i32, f64>::mandatory_from_ini(
            String::from("HEADER"),
            String::from("POWER_CURVE"),
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
