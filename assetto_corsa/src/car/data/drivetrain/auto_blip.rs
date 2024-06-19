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

use crate::car::data::drivetrain::get_mandatory_field;
use crate::ini_utils;
use crate::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::error::Result;


#[derive(Debug)]
pub struct AutoBlip {
    pub electronic: i32,
    pub points: Vec<i32>,
    pub level: f64
}

impl AutoBlip {
    const SECTION_NAME: &'static str = "AUTOBLIP";
    fn get_point_key<T: std::fmt::Display>(idx: T) -> String {
        format!("POINT_{}", idx)
    }
}

impl MandatoryDataSection for AutoBlip {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let electronic = get_mandatory_field(ini_data, AutoBlip::SECTION_NAME, "ELECTRONIC")?;
        let level = get_mandatory_field(ini_data, AutoBlip::SECTION_NAME, "LEVEL")?;
        let mut points = Vec::new();
        for idx in 0..3 {
            points.push(get_mandatory_field(ini_data,
                                            AutoBlip::SECTION_NAME,
                                            AutoBlip::get_point_key(idx).as_str())?);
        }
        Ok(AutoBlip{ electronic, points, level })
    }
}

impl CarDataUpdater for AutoBlip {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        ini_utils::set_value(ini_data, AutoBlip::SECTION_NAME, "ELECTRONIC", self.electronic);
        ini_utils::set_float(ini_data, AutoBlip::SECTION_NAME, "LEVEL", self.level, 2);
        for (idx, point) in self.points.iter().enumerate() {
            ini_utils::set_value(ini_data,
                                 AutoBlip::SECTION_NAME,
                                 AutoBlip::get_point_key(idx).as_str(),
                                 point);
        }
        Ok(())
    }
}
