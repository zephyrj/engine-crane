/*
 * Copyright (c):
 * 2022 zephyrj
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

use crate::assetto_corsa::car::data::drivetrain::get_mandatory_field;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::assetto_corsa::error::Result;


#[derive(Debug)]
pub struct DownshiftProtection {
    pub active: i32,
    pub debug: i32,
    pub overrev: i32,
    pub lock_n: i32
}

impl MandatoryDataSection for DownshiftProtection {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        Ok(DownshiftProtection{
            active: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "ACTIVE")?,
            debug: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "DEBUG")?,
            overrev: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "OVERREV")?,
            lock_n: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "LOCK_N")?,
        })
    }
}

impl CarDataUpdater for DownshiftProtection {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "ACTIVE", self.active);
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "DEBUG", self.debug);
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "OVERREV", self.overrev);
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "LOCK_N", self.lock_n);
        Ok(())
    }
}