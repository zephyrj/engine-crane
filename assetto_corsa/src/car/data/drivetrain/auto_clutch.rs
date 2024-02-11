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

use crate::car::data::drivetrain::{get_mandatory_field, mandatory_field_error};
use crate::ini_utils;
use crate::ini_utils::Ini;
use crate::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::error::Result;


#[allow(dead_code)]
#[derive(Debug)]
pub struct ShiftProfile {
    name: String,
    points: Vec<i32>
}

impl ShiftProfile {
    pub fn load_from_ini(ini_data: &Ini, name: &str) -> Result<ShiftProfile> {
        let name = String::from(name);
        let mut points = Vec::new();
        for idx in 0..3 {
            points.push(get_mandatory_field(ini_data, &name, &format!("POINT_{}", idx))?);
        }
        Ok(ShiftProfile { name, points })
    }
}

#[derive(Debug)]
pub struct AutoClutch {
    #[allow(dead_code)]
    upshift_profile: Option<ShiftProfile>,
    #[allow(dead_code)]
    downshift_profile: Option<ShiftProfile>,
    pub use_on_changes: i32,
    pub min_rpm: i32,
    pub max_rpm: i32,
    pub forced_on: i32
}

impl AutoClutch {
    fn load_shift_profile(ini_data: &Ini, key_name: &str) -> Result<Option<ShiftProfile>> {
        if let Some(profile_name) = ini_utils::get_value(ini_data, "AUTOCLUTCH", key_name) {
            let section_name: String = profile_name;
            if section_name.to_lowercase() != "none" {
                return match ShiftProfile::load_from_ini(ini_data, &section_name) {
                    Ok(prof) => { Ok(Some(prof)) },
                    Err(_) => { return Err(mandatory_field_error(key_name, &section_name)); }
                }
            }
        }
        Ok(None)
    }
}

impl MandatoryDataSection for AutoClutch {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let upshift_profile = AutoClutch::load_shift_profile(ini_data, "UPSHIFT_PROFILE")?;
        let downshift_profile = AutoClutch::load_shift_profile(ini_data, "DOWNSHIFT_PROFILE")?;
        let use_on_changes = get_mandatory_field(ini_data, "AUTOCLUTCH", "USE_ON_CHANGES")?;
        let min_rpm = get_mandatory_field(ini_data, "AUTOCLUTCH", "MIN_RPM")?;
        let max_rpm = get_mandatory_field(ini_data, "AUTOCLUTCH", "MAX_RPM")?;
        let forced_on = get_mandatory_field(ini_data, "AUTOCLUTCH", "FORCED_ON")?;

        Ok(AutoClutch {
            upshift_profile,
            downshift_profile,
            use_on_changes,
            min_rpm,
            max_rpm,
            forced_on
        })
    }
}

impl CarDataUpdater for AutoClutch {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "USE_ON_CHANGES", self.use_on_changes);
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "MIN_RPM", self.min_rpm);
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "MAX_RPM", self.max_rpm);
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "FORCED_ON", self.forced_on);
        Ok(())
    }
}
