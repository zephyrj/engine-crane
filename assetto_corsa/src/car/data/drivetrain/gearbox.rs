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
use crate::error::{Error, ErrorKind, Result};


#[derive(Debug)]
pub struct Gearbox {
    gear_count: i32,
    pub reverse_gear_ratio: f64,
    pub final_gear_ratio: f64,
    gear_ratios: Vec<f64>,
    pub change_up_time: i32,
    pub change_dn_time: i32,
    pub auto_cutoff_time: i32,
    pub supports_shifter: i32,
    pub valid_shift_rpm_window: i32,
    pub controls_window_gain: f64,
    pub inertia: f64
}

impl Gearbox {
    pub fn update_gears(&mut self, gear_ratios: Vec<f64>) {
        self.gear_ratios = gear_ratios;
        self.gear_count = self.gear_ratios.len() as i32;
    }

    pub fn num_gears(&self) -> usize {
        self.gear_ratios.len()
    }

    fn create_gear_key(gear_num: i32) -> String {
        format!("GEAR_{}", gear_num)
    }

    pub fn gear_ratios(&self) -> &Vec<f64> {
        &self.gear_ratios
    }

    pub fn final_drive(&self) -> f64 {
        self.final_gear_ratio
    }

    pub fn update_final_drive(&mut self, new: f64) {
        self.final_gear_ratio = new
    }
}

impl MandatoryDataSection for Gearbox {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let gear_count = get_mandatory_field(ini_data, "GEARS", "COUNT")?;
        let mut gear_ratios = Vec::new();
        for gear_num in 1..gear_count+1 {
            let gear_key = format!("GEAR_{}", gear_num);
            gear_ratios.push(get_mandatory_field(ini_data, "GEARS", gear_key.as_str())?);
        }
        let reverse_gear_ratio = get_mandatory_field(ini_data, "GEARS", "GEAR_R")?;
        let final_gear_ratio = get_mandatory_field(ini_data, "GEARS", "FINAL")?;
        let change_up_time = get_mandatory_field(ini_data, "GEARBOX", "CHANGE_UP_TIME")?;
        let change_dn_time = get_mandatory_field(ini_data, "GEARBOX", "CHANGE_DN_TIME")?;
        let auto_cutoff_time = get_mandatory_field(ini_data, "GEARBOX", "AUTO_CUTOFF_TIME")?;
        let supports_shifter = get_mandatory_field(ini_data, "GEARBOX", "SUPPORTS_SHIFTER")?;
        let valid_shift_rpm_window = get_mandatory_field(ini_data, "GEARBOX", "VALID_SHIFT_RPM_WINDOW")?;
        let controls_window_gain = get_mandatory_field(ini_data, "GEARBOX", "CONTROLS_WINDOW_GAIN")?;
        let inertia = get_mandatory_field(ini_data, "GEARBOX", "INERTIA")?;
        Ok(Gearbox {
            gear_count,
            reverse_gear_ratio,
            final_gear_ratio,
            gear_ratios,
            change_up_time,
            change_dn_time,
            auto_cutoff_time,
            supports_shifter,
            valid_shift_rpm_window,
            controls_window_gain,
            inertia
        })
    }
}

impl CarDataUpdater for Gearbox {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        let current_count_opt: Option<i32> = ini_utils::get_value(ini_data, "GEARS", "COUNT");
        if let Some(current_count) = current_count_opt {
            if current_count != self.gear_count {
                for gear_num in 1..current_count+1 {
                    ini_data.remove_value("GEARS", Gearbox::create_gear_key(gear_num).as_str());
                }
            }
        }
        ini_data.set_value("GEARS", "COUNT", self.gear_count.to_string());
        for gear_num in 1..self.gear_count+1 {
            if let Some(gear_ratio) = self.gear_ratios.get((gear_num-1) as usize) {
                ini_utils::set_float(ini_data,
                                     "GEARS",
                                     Gearbox::create_gear_key(gear_num).as_str(),
                                     *gear_ratio,
                                     3);
            } else {
                return Err(Error::new(ErrorKind::InvalidUpdate,
                                      "gear count doesn't match stored ratios".to_owned()));
            }
        }
        ini_utils::set_float(ini_data, "GEARS", "GEAR_R", self.reverse_gear_ratio, 3);
        ini_utils::set_float(ini_data, "GEARS", "FINAL", self.final_gear_ratio, 3);
        ini_utils::set_value(ini_data, "GEARBOX", "CHANGE_UP_TIME", self.change_up_time);
        ini_utils::set_value(ini_data, "GEARBOX", "CHANGE_DN_TIME", self.change_dn_time);
        ini_utils::set_value(ini_data, "GEARBOX", "AUTO_CUTOFF_TIME", self.auto_cutoff_time);
        ini_utils::set_value(ini_data, "GEARBOX", "SUPPORTS_SHIFTER", self.supports_shifter);
        ini_utils::set_value(ini_data, "GEARBOX", "VALID_SHIFT_RPM_WINDOW", self.valid_shift_rpm_window);
        ini_utils::set_float(ini_data, "GEARBOX", "CONTROLS_WINDOW_GAIN", self.controls_window_gain, 2);
        ini_utils::set_float(ini_data, "GEARBOX", "INERTIA", self.inertia, 3);
        Ok(())
    }
}
