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

use crate::traits::{CarDataFile, CarDataUpdater, OptionalDataSection};
use crate::error::Result;
use crate::ini_utils;
use crate::ini_utils::Ini;


#[derive(Debug)]
pub struct Turbo {
    bov_pressure_threshold: Option<f64>,
    sections: Vec<TurboSection>
}

impl OptionalDataSection for Turbo {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Option<Self>> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let turbo_count = Turbo::count_turbo_sections(ini_data);
        if turbo_count == 0 {
            return Ok(None);
        }

        let pressure_threshold = ini_utils::get_value(ini_data, "BOV", "PRESSURE_THRESHOLD");
        let mut section_vec: Vec<TurboSection> = Vec::new();
        for idx in 0..turbo_count {
            section_vec.push(TurboSection::load_from_parent(idx, parent_data)?);
        }

        Ok(Some(Turbo{
            bov_pressure_threshold: pressure_threshold,
            sections: section_vec
        }))
    }
}

impl Turbo {
    pub fn new() -> Turbo {
        Turbo {
            bov_pressure_threshold: None,
            sections: Vec::new()
        }
    }

    pub fn set_bov_threshold(&mut self, threshold: f64) {
        self.bov_pressure_threshold = Some(threshold)
    }

    pub fn clear_bov_threshold(&mut self) {
        self.bov_pressure_threshold = None
    }

    pub fn add_section(&mut self, section: TurboSection) {
        self.sections.push(section)
    }

    pub fn clear_sections(&mut self) {
        self.sections.clear()
    }

    pub fn delete_from_car_data(&mut self, car_data: &mut dyn CarDataFile) -> Result<()> {
        for section in &mut self.sections {
            section.delete_from_car_data(car_data)?;
        }
        self.sections.clear();
        Ok(())
    }

    pub fn count_turbo_sections(ini: &Ini) -> usize {
        let mut count = 0;
        loop {
            if !ini.contains_section(TurboSection::get_ini_section_name(count).as_str()) {
                return count;
            }
            count += 1;
        }
    }
}

impl CarDataUpdater for Turbo {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        for idx in 0..Turbo::count_turbo_sections(car_data.ini_data()) {
            TurboSection::load_from_parent(idx, car_data)?.delete_from_car_data(car_data)?;
        }
        if let Some(bov_pressure_threshold) = self.bov_pressure_threshold {
            ini_utils::set_float(car_data.mut_ini_data(), "BOV", "PRESSURE_THRESHOLD", bov_pressure_threshold, 2);
        } else {
            car_data.mut_ini_data().remove_section("BOV");
        }
        for section in &self.sections {
            section.update_car_data(car_data)?;
        }
        Ok(())
    }
}


#[derive(Debug)]
pub struct TurboSection {
    index: usize,
    lag_dn: f64,
    lag_up: f64,
    max_boost: f64,
    wastegate: f64,
    display_max_boost: f64,
    reference_rpm: i32,
    gamma: f64,
    cockpit_adjustable: i32,
}

impl TurboSection {
    pub fn from_defaults(index: usize) -> TurboSection {
        TurboSection {
            index,
            lag_dn: 0.99,
            lag_up: 0.965,
            max_boost: 1.0,
            wastegate: 1.0,
            display_max_boost: 1.0,
            reference_rpm: 3000,
            gamma: 1.0,
            cockpit_adjustable: 0,
        }
    }

    pub fn new(index: usize,
               lag_dn: f64,
               lag_up: f64,
               max_boost: f64,
               wastegate: f64,
               display_max_boost: f64,
               reference_rpm: i32,
               gamma: f64,
               cockpit_adjustable: i32) -> TurboSection
    {
        TurboSection {
            index,
            lag_dn,
            lag_up,
            max_boost,
            wastegate,
            display_max_boost,
            reference_rpm,
            gamma,
            cockpit_adjustable,
        }
    }

    pub fn load_from_parent(idx: usize, parent_data: &dyn CarDataFile) -> Result<TurboSection> {
        let section_name = TurboSection::get_ini_section_name(idx);
        let ini_data = parent_data.ini_data();
        Ok(TurboSection {
            index: idx,
            lag_dn: ini_utils::get_mandatory_property(ini_data, &section_name, "LAG_DN")?,
            lag_up: ini_utils::get_mandatory_property(ini_data, &section_name, "LAG_UP")?,
            max_boost: ini_utils::get_mandatory_property(ini_data, &section_name, "MAX_BOOST")?,
            wastegate: ini_utils::get_mandatory_property(ini_data, &section_name, "WASTEGATE")?,
            display_max_boost: ini_utils::get_mandatory_property(ini_data, &section_name, "DISPLAY_MAX_BOOST")?,
            reference_rpm: ini_utils::get_mandatory_property(ini_data, &section_name, "REFERENCE_RPM")?,
            gamma: ini_utils::get_mandatory_property(ini_data, &section_name, "GAMMA")?,
            cockpit_adjustable: ini_utils::get_mandatory_property(ini_data, &section_name, "COCKPIT_ADJUSTABLE")?,
        })
    }

    pub fn section_name(&self) -> String {
        TurboSection::get_ini_section_name(self.index)
    }

    pub fn delete_from_car_data(&mut self, car_data: &mut dyn CarDataFile) -> Result<()> {
        car_data.mut_ini_data().remove_section(&self.section_name());
        Ok(())
    }

    pub fn get_ini_section_name(idx: usize) -> String {
        format!("TURBO_{}", idx)
    }
}

impl CarDataUpdater for TurboSection {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        let section_name = TurboSection::get_ini_section_name(self.index);
        ini_utils::set_float(ini_data, &section_name, "LAG_DN", self.lag_dn, 3);
        ini_utils::set_float(ini_data, &section_name, "LAG_UP", self.lag_up, 3);
        ini_utils::set_float(ini_data, &section_name, "MAX_BOOST", self.max_boost, 2);
        ini_utils::set_float(ini_data, &section_name, "WASTEGATE", self.wastegate, 2);
        ini_utils::set_float(ini_data, &section_name, "DISPLAY_MAX_BOOST", self.display_max_boost, 2);
        ini_utils::set_value(ini_data, &section_name, "REFERENCE_RPM", self.reference_rpm);
        ini_utils::set_float(ini_data, &section_name, "GAMMA", self.gamma, 2);
        ini_utils::set_value(ini_data, &section_name, "COCKPIT_ADJUSTABLE", self.cockpit_adjustable);
        Ok(())
    }
}
