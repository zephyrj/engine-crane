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

use crate::car::data::drivetrain::get_mandatory_field;
use crate::ini_utils;
use crate::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::error::Result;


#[derive(Debug)]
pub struct Clutch {
    pub max_torque: i32
}

impl MandatoryDataSection for Clutch {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> {
        Ok(Clutch{
            max_torque: get_mandatory_field(parent_data.ini_data(), "CLUTCH", "MAX_TORQUE")?
        })
    }
}

impl CarDataUpdater for Clutch {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        ini_utils::set_value(car_data.mut_ini_data(), "CLUTCH", "MAX_TORQUE", self.max_torque);
        Ok(())
    }
}
