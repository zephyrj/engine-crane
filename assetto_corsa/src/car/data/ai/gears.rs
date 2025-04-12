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

use crate::car::data::ai::Ai;
use crate::ini_utils;
use crate::error::{Result, Error, ErrorKind};
use crate::ini_utils::{Ini, MissingMandatoryProperty};
use crate::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};

#[derive(Debug)]
pub struct Gears {
    pub up: i32,
    pub down: i32,
    pub slip_threshold: f64,
    pub gas_cutoff_time: f64
}

impl Gears {
    fn _load(ini_data: &Ini) -> std::result::Result<Gears, MissingMandatoryProperty> {
        let up = ini_utils::get_mandatory_property(ini_data, "GEARS", "UP")?;
        let down = ini_utils::get_mandatory_property(ini_data, "GEARS", "DOWN")?;
        let slip_threshold = ini_utils::get_mandatory_property(ini_data, "GEARS", "SLIP_THRESHOLD")?;
        let gas_cutoff_time = ini_utils::get_mandatory_property(ini_data, "GEARS", "GAS_CUTOFF_TIME")?;
        Ok(Gears{ up, down, slip_threshold, gas_cutoff_time })
    }
}

impl MandatoryDataSection for Gears {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        return match Gears::_load(parent_data.ini_data()) {
            Ok(g) => { Ok(g) }
            Err(e) => {
                Err(Error::new(
                    ErrorKind::InvalidCar,
                    format!("Missing {}.{} in {}", e.section_name, e.property_name, Ai::INI_FILENAME)
                ))
            }
        }
    }
}

impl CarDataUpdater for Gears {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        ini_utils::set_value(ini_data, "GEARS", "UP", self.up);
        ini_utils::set_value(ini_data, "GEARS", "DOWN", self.down);
        ini_utils::set_float(ini_data, "GEARS", "SLIP_THRESHOLD", self.slip_threshold, 2);
        ini_utils::set_float(ini_data, "GEARS", "GAS_CUTOFF_TIME", self.gas_cutoff_time, 2);
        Ok(())
    }
}