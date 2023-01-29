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

use std::cmp::{min, Ordering};
use fraction::Bounded;
use crate::assetto_corsa::car::data::setup::gears::GearConfig::{GearSets, PerGear};
use crate::assetto_corsa::car::data::setup::HelpData;
use crate::assetto_corsa::car::lut_utils::LutType;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection, OptionalDataSection};
use crate::assetto_corsa::error::{Error, ErrorKind, Result};
use crate::assetto_corsa::car::structs::LutProperty;
use crate::assetto_corsa::ini_utils::Ini;

#[derive(Debug)]
pub struct GearData {
    pub gear_config: Option<GearConfig>,
    pub final_drive: Option<SingleGear>
}

impl MandatoryDataSection for GearData {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<GearData> where Self: Sized {
        let final_drive = SingleGear::load_from_ini(parent_data,"FINAL_GEAR_RATIO".to_string())?;
        let gear_config = GearConfig::load_from_car_data(parent_data)?;
        Ok((GearData{ gear_config, final_drive }))
    }
}

impl CarDataUpdater for GearData {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        if let Some(gear_config) = &self.gear_config {
            gear_config.update_car_data(car_data)?;
        }
        if let Some(final_drive) = &self.final_drive {
            final_drive.update_car_data(car_data)?;
        }
        Ok(())
    }
}

impl GearData {
    // TODO finish this
    pub fn auto_space_gear_menu_positions(&mut self) {
        let mut x_pos;
        let mut start_y_pos = f64::max_value();
        if let Some(gear_config) = &mut self.gear_config {
            match gear_config {
                PerGear(gears) => {
                    for gear in gears {
                        x_pos = gear.menu_pos_x;
                        if let Some(ordering) = gear.menu_pos_y.partial_cmp(&start_y_pos) {
                            match ordering {
                                Ordering::Less => { start_y_pos = gear.menu_pos_y}
                                _ => {}
                            }
                        }
                    }
                },
                _ => {}
            };
        }
    }
}

#[derive(Debug)]
pub enum GearConfig {
    GearSets(Vec<GearSet>),
    PerGear(Vec<SingleGear>)
}

impl GearConfig {
    pub fn load_from_car_data(parent_data: &dyn CarDataFile) -> Result<Option<GearConfig>> {
        let ini_data = parent_data.ini_data();
        let found_gears = SingleGear::load_all_from_car_data(parent_data)?;
        let found_gear_sets = GearSet::load_all_from_car_data(parent_data)?;
        if !found_gears.is_empty() && !found_gear_sets.is_empty() {
            return match ini_utils::get_value::<i32>(ini_data, "GEARS", "USE_GEARSET") {
                None => { Ok(Some(PerGear(found_gears))) }
                Some(val) => {
                    match &val {
                        1 => {
                            Ok(Some(GearSets(found_gear_sets)))
                        },
                        _ => {
                            Ok(Some(PerGear(found_gears)))
                        }
                    }
                }
            }
        }
        if !found_gears.is_empty() {
            return Ok(Some(PerGear(found_gears)))
        }
        if !found_gear_sets.is_empty() {
            return Ok(Some(GearSets(found_gear_sets)))
        }
        Ok(None)
    }

    pub fn clear_existing_config(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        SingleGear::delete_all_from_car_data(car_data)?;
        let ini_data = car_data.mut_ini_data();
        GearSet::delete_all_from_ini(ini_data);
        ini_data.remove_section("GEARS");
        Ok(())
    }

    pub fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        self.clear_existing_config(car_data)?;
        match &self {
            GearSets(gearset_vec) => {
                let ini_data = car_data.mut_ini_data();
                ini_data.set_value("GEARS", "USE_GEARSET", "1".to_string());
                for (idx, gearset) in gearset_vec.iter().enumerate() {
                    gearset.update_ini(ini_data, idx)?;
                }
            }
            PerGear(gears) => {
                for gear in gears {
                    gear.update_car_data(car_data)?;
                }
                let ini_data = car_data.mut_ini_data();
                ini_data.set_value("GEARS", "USE_GEARSET", "0".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SingleGear {
    pub gear_id: String,
    pub ratios_lut: LutProperty<String, f64>,
    pub name: String,
    pub menu_pos_x: f64,
    pub menu_pos_y: f64,
    pub help_data: HelpData,
}

impl SingleGear {
    pub fn load_all_from_car_data(parent_data: &dyn CarDataFile) -> Result<Vec<SingleGear>> {
        let mut gear_vec : Vec<SingleGear> = Vec::new();
        let mut current_index = 1;
        loop {
            match SingleGear::load_from_ini(parent_data,
                                            SingleGear::create_gear_key(current_index)) {
                Ok(option) => {
                    match option
                    {
                        None => break,
                        Some(gear_config) => {
                            gear_vec.push(gear_config);
                        }
                    }
                }
                Err(e) => {
                    return Err(Error::new(ErrorKind::InvalidCar,
                                          format!("Failed to load gear info from setup.ini. {}", e)));
                }
            }
            current_index += 1;
        }
        Ok(gear_vec)
    }

    pub fn load_from_ini(parent_data: &dyn CarDataFile, section_name: String) -> Result<Option<SingleGear>> {
        let ini_data = parent_data.ini_data();
        return match ini_data.contains_section(&section_name) {
            true => {
                let ratios_lut =
                    match LutProperty::<String, f64>::mandatory_from_ini(
                        section_name.clone(),
                        String::from("RATIOS"),
                        ini_data,
                        parent_data.data_interface()) {
                        Ok(lut) => {
                            lut
                        }
                        Err(e) => {
                            return Err(Error::new(ErrorKind::InvalidCar,
                                                  format!("Failed to load ratios file for {}. {}", section_name, e)));
                        }
                };
                let name = ini_utils::get_mandatory_property::<String>(ini_data, &section_name, "NAME")?;
                let menu_pos_x = ini_utils::get_mandatory_property::<f64>(ini_data,&section_name, "POS_X")?;
                let menu_pos_y = ini_utils::get_mandatory_property::<f64>(ini_data,&section_name, "POS_Y")?;
                let help_data = ini_utils::get_mandatory_property::<HelpData>(ini_data,&section_name, "HELP")?;
                Ok(Some(SingleGear {
                    gear_id: section_name, ratios_lut, name, menu_pos_x, menu_pos_y, help_data
                }))
            }
            false => { Ok(None) }
        }
    }

    pub fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        self.ratios_lut.update_car_data(car_data)?;
        let ini_data = car_data.mut_ini_data();
        ini_utils::set_value(ini_data, &self.gear_id, "NAME", self.name.clone());
        ini_utils::set_value(ini_data,&self.gear_id, "POS_X", self.menu_pos_x);
        ini_utils::set_value(ini_data,&self.gear_id, "POS_Y", self.menu_pos_y);
        ini_utils::set_value(ini_data,&self.gear_id, "HELP", &self.help_data);
        Ok(())
    }

    pub fn load_section_names_from_ini(ini_data: &Ini) -> Vec<&str> {
        let mut sections = ini_data.get_section_names_starting_with("GEAR_");
        sections = sections.iter().filter_map(|name| -> Option<&str> {
            if !name.contains("SET") {
                return Some(name);
            }
            None
        }).collect();
        sort_by_numeric_index(sections)
    }

    pub fn delete_all_from_car_data(car_data: &mut dyn CarDataFile) -> Result<()> {
        let existing_gear_names : Vec<String>;
        {
            let ini_data = car_data.mut_ini_data();
            {
                let tmp = SingleGear::load_section_names_from_ini(ini_data);
                existing_gear_names = tmp.iter().map(|name| -> String { name.to_string() }).collect();
            }
        }

        for name in existing_gear_names {
            if let Some(gear) = SingleGear::load_from_ini(car_data, name.clone())? {
                gear.ratios_lut.delete_from_car_data(car_data)
            }
            car_data.mut_ini_data().remove_section(&name);
        }
        Ok(())
    }

    pub fn create_gear_key(gear_index: i32) -> String {
        format!("GEAR_{}", gear_index)
    }

    pub fn create_gear_name(gear_index: u32) -> String {
        match gear_index {
            1 => "First".to_string(),
            2 => "Second".to_string(),
            3 => "Third".to_string(),
            4 => "Fourth".to_string(),
            5 => "Fifth".to_string(),
            6 => "Sixth".to_string(),
            7 => "Seventh".to_string(),
            8 => "Eighth".to_string(),
            9 => "Ninth".to_string(),
            10 => "Tenth".to_string(),
            _ => gear_index.to_string()
        }
    }

    pub fn deduce_gear_ratio_filename(&self) -> String {
        if self.gear_id.starts_with("FINAL") {
            return "final".to_string();
        }
        return match self.gear_id.chars().last() {
            None => "Unknown".to_string(),
            Some(c) => {
                match c.to_digit(10) {
                    None => { "Unknown".to_string() }
                    Some(digit) => {
                        SingleGear::create_gear_name(digit)
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct GearSet {
    name: String,
    ratios: Vec<f64>
}

impl GearSet {
    const SECTION_PREFIX: &'static str = "GEAR_SET_";

    pub fn load_all_from_car_data(parent_data: &dyn CarDataFile) -> Result<Vec<GearSet>> {
        let mut gearset_vet = Vec::new();
        let ini_data = parent_data.ini_data();
        for section_name in GearSet::load_section_names_from_ini(ini_data) {
            let name = ini_utils::get_mandatory_property(ini_data, section_name, "NAME")?;
            let mut ratios: Vec<f64> = Vec::new();
            let mut gear_idx = 1;
            loop {
                if let Some(ratio) = ini_utils::get_value(ini_data, section_name, &format!("GEAR_{}", gear_idx)) {
                    ratios.push(ratio);
                    gear_idx+=1;
                } else {
                    break;
                }
            }
            gearset_vet.push(GearSet { name, ratios} )
        }
        Ok(gearset_vet)
    }

    pub fn load_section_names_from_ini(ini_data: &Ini) -> Vec<&str> {
        sort_by_numeric_index(ini_data.get_section_names_starting_with(GearSet::SECTION_PREFIX))
    }

    pub fn delete_all_from_ini(ini_data: &mut Ini) {
        let existing_gearset_names: Vec<String>;
        {
            let tmp = GearSet::load_section_names_from_ini(ini_data);
            existing_gearset_names = tmp.iter().map(|name| -> String { name.to_string() }).collect();
        }
        for name in existing_gearset_names {
            ini_data.remove_section(&name);
        }
    }

    pub fn update_ini(&self, ini_data: &mut Ini, as_index: usize) -> Result<()> {
        ini_data.set_value(&format!("{}{}", GearSet::SECTION_PREFIX, as_index), "NAME", self.name.clone());
        for (idx, ratio) in (1..=self.ratios.len()).zip(&self.ratios) {
            ini_data.set_value(&format!("{}{}", GearSet::SECTION_PREFIX, as_index),
                               &format!("GEAR_{}", idx),
                               ratio.to_string());
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn num_gears(&self) -> usize {
        self.ratios.len()
    }
}

fn sort_by_numeric_index(mut var: Vec<&str>) -> Vec<&str> {
    var.sort_by_key(|name|{
        let mut tmp = String::new();
        for c in name.chars() {
            if c.is_numeric() {
                tmp.push(c)
            }
        }
        tmp.parse::<u32>().unwrap_or(0)
    });
    var
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use rand::thread_rng;
    use rand::seq::SliceRandom;
    use tracing_subscriber::fmt::format;
    use crate::assetto_corsa::{Car, car};
    use crate::assetto_corsa::car::data::setup::gears::{GearConfig, GearData, GearSet, SingleGear, sort_by_numeric_index};
    use crate::assetto_corsa::car::data::setup::Setup;
    use crate::assetto_corsa::traits::{CarDataUpdater, MandatoryDataSection};

    const TEST_DATA_PATH: &'static str = "test_data";
    const TEMP_TEST_CAR_NAME: &'static str = "tmp_car";

    fn get_test_car_path(car_name: &str) -> PathBuf {
        let mut test_folder_path = PathBuf::from(Path::new(file!()).parent().unwrap());
        test_folder_path.push(format!("{}/{}", TEST_DATA_PATH, car_name));
        test_folder_path
    }

    fn load_test_car(test_car_name: &str) -> Car {
        Car::load_from_path(&get_test_car_path(test_car_name)).unwrap()
    }

    fn get_tmp_car() -> Car {
        load_test_car(TEMP_TEST_CAR_NAME)
    }

    fn delete_tmp_car() {
        let tmp_car_path = get_test_car_path(TEMP_TEST_CAR_NAME);
        if tmp_car_path.exists() {
            std::fs::remove_dir_all(tmp_car_path).unwrap();
        }
    }

    fn create_tmp_car() -> Car {
        delete_tmp_car();
        Car::new(get_test_car_path(TEMP_TEST_CAR_NAME)).unwrap()
    }

    fn setup_tmp_car_as(test_car_name: &str) -> Car {
        create_tmp_car();
        let mut copy_options = fs_extra::dir::CopyOptions::new();
        copy_options.content_only = true;
        fs_extra::dir::copy(get_test_car_path(test_car_name),
                            get_test_car_path(TEMP_TEST_CAR_NAME),
                            &copy_options).unwrap();
        Car::load_from_path(&get_test_car_path(TEMP_TEST_CAR_NAME)).unwrap()
    }

    fn create_vec_for_range_from(range: Vec<usize>, elements: &Vec<String>) -> Vec<&str> {
        let mut out_vec = Vec::new();
        for n in range {
            out_vec.push(elements[n].as_str());
        }
        out_vec
    }

    #[test]
    fn order_by_index() {
        let create_sorted_ved = |num_elements: usize, element_prefix: &str| -> Vec<String> {
            let mut sorted : Vec<String> = Vec::new();
            for n in 1..num_elements+1 {
                sorted.push(format!("{}_{}", element_prefix, n));
            }
            sorted
        };

        let num_gears = 9;
        let sorted_gears = create_sorted_ved(num_gears, "GEARS");
        let sorted_vec = vec!(&sorted_gears[0], &sorted_gears[1], &sorted_gears[2],
                                          &sorted_gears[3], &sorted_gears[4], &sorted_gears[5],
                                          &sorted_gears[6], &sorted_gears[7], &sorted_gears[8]);

        let t = create_vec_for_range_from((0..num_gears).collect(), &sorted_gears);
        assert_eq!(sort_by_numeric_index(t), sorted_vec);

        let t2 = create_vec_for_range_from((0..num_gears).rev().collect(), &sorted_gears);
        assert_eq!(sort_by_numeric_index(t2), sorted_vec);

        let test_runs = 100;
        for n in 0..test_runs {
            let mut vec : Vec<usize> = (0..num_gears).collect();
            vec.shuffle(&mut thread_rng());
            let t = create_vec_for_range_from(vec, &sorted_gears);
            assert_eq!(sort_by_numeric_index(t), create_vec_for_range_from((0..num_gears).collect(), &sorted_gears));
        }
    }

    #[test]
    fn load_gearset() {
        let mut car = load_test_car("gearset");
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        let gear_sets = GearSet::load_all_from_car_data(&car_setup_data).unwrap();
        assert_eq!(gear_sets.len(), 3)
    }

    #[test]
    fn load_gears_single_ratio_file() {
        let mut car = load_test_car("gears-single-ratio-file");
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        let gears = SingleGear::load_all_from_car_data(&car_setup_data).unwrap();
        assert_eq!(gears.len(), 6)
    }

    #[test]
    fn load_gears_per_gear_ratio_file() {
        let mut car = load_test_car("gears-per-gear-ratio-file");
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        let gears = SingleGear::load_all_from_car_data(&car_setup_data).unwrap();
        assert_eq!(gears.len(), 7);
        let expected_ratios : Vec<usize> = vec![15, 11, 11, 21, 21, 14, 14];
        for (num_ratios, gear) in expected_ratios.iter().zip(gears) {
            assert_eq!(*num_ratios, gear.ratios_lut.num_entries())
        }
    }

    #[test]
    fn load_no_gears() {
        let mut car = load_test_car("no-customizable-gears");
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        let gear_data =  GearData::load_from_parent(&car_setup_data).unwrap();
        assert!(gear_data.gear_config.is_none());
        assert!(gear_data.final_drive.is_none());
    }

    #[test]
    fn load_only_final() {
        let mut car = load_test_car("only-final-drive");
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        let gear_data =  GearData::load_from_parent(&car_setup_data).unwrap();
        assert!(gear_data.gear_config.is_none());
        let final_drive = gear_data.final_drive.unwrap();
        assert_eq!(final_drive.name, "Final Gear Ratio");
        assert_eq!(final_drive.ratios_lut.num_entries(), 5)
    }

    #[test]
    fn read_write_read_gearset() {
        {
            let mut car = setup_tmp_car_as("gearset");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            assert_eq!(GearSet::load_all_from_car_data(&car_setup_data).unwrap().len(), 3);
            let data = GearData::load_from_parent(&car_setup_data).unwrap();
            data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }

        let mut car = load_test_car(TEMP_TEST_CAR_NAME);
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        assert_eq!(GearSet::load_all_from_car_data(&car_setup_data).unwrap().len(), 3);
    }

    #[test]
    fn read_write_read_gears() {
        let validate_gears = |gears : &Vec<SingleGear>| {
            assert_eq!(gears.len(), 7);
            let expected_ratios : Vec<usize> = vec![15, 11, 11, 21, 21, 14, 14];
            for (num_ratios, gear) in expected_ratios.iter().zip(gears) {
                assert_eq!(*num_ratios, gear.ratios_lut.num_entries())
            }
        };

        {
            let mut car = setup_tmp_car_as("gears-per-gear-ratio-file");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            let data = GearData::load_from_parent(&car_setup_data).unwrap();
            assert!(data.gear_config.is_some());
            match &data.gear_config.as_ref().unwrap() {
                GearConfig::GearSets(_) => {}
                GearConfig::PerGear(gears) => {
                    validate_gears(gears);
                }
            }
            data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }

        let mut car = load_test_car(TEMP_TEST_CAR_NAME);
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        let data = GearData::load_from_parent(&car_setup_data).unwrap();
        assert!(data.gear_config.is_some());
        match &data.gear_config.as_ref().unwrap() {
            GearConfig::GearSets(_) => {}
            GearConfig::PerGear(gears) => {
                validate_gears(gears);
            }
        }
    }

    #[test]
    fn read_write_read_final_drive() {
        let validate_gears = |gear_data : &GearData| {
            assert!(gear_data.gear_config.is_none());
            let final_drive = gear_data.final_drive.as_ref().unwrap();
            assert_eq!(final_drive.name, "Final Gear Ratio");
            assert_eq!(final_drive.ratios_lut.num_entries(), 5)
        };

        {
            let mut car = setup_tmp_car_as("only-final-drive");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
            validate_gears(&gear_data);
            gear_data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }

        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        validate_gears(&gear_data);
    }

    #[test]
    fn read_write_read_no_gears() {
        let validate_gears = |gear_data : &GearData| {
            assert!(gear_data.gear_config.is_none());
            assert!(gear_data.final_drive.is_none());
        };

        {
            let mut car = setup_tmp_car_as("no-customizable-gears");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            assert!(!car_setup_data.ini_data.contains_section("GEARS"));
            let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
            validate_gears(&gear_data);
            gear_data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }

        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        assert!(!car_setup_data.ini_data.contains_section("GEARS"));
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        validate_gears(&gear_data);
    }
}