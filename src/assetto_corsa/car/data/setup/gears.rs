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
use std::collections::HashMap;
use fraction::Bounded;
use itertools::Itertools;
use crate::assetto_corsa::car::data::setup::gears::GearConfig::{GearSets, PerGear};
use crate::assetto_corsa::car::data::setup::HelpData;
use crate::assetto_corsa::car::lut_utils::{LutFile, LutType};
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
        Ok(GearData{ gear_config, final_drive })
    }
}

impl CarDataUpdater for GearData {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        self.clear_existing_config(car_data)?;
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

    pub fn set_gear_config(&mut self, new_gear_config: Option<GearConfig>) -> Option<GearConfig> {
        std::mem::replace(&mut self.gear_config, new_gear_config)
    }

    pub fn clear_gear_config(&mut self) -> Option<GearConfig> {
        self.set_gear_config(None)
    }

    pub fn set_final_drive(&mut self, new_final_drive: Option<SingleGear>) -> Option<SingleGear> {
        std::mem::replace(&mut self.final_drive, new_final_drive)
    }

    pub fn clear_final_drive(&mut self) -> Option<SingleGear> {
        self.set_final_drive(None)
    }

    fn clear_existing_config(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        SingleGear::delete_all_gears_from_car_data(car_data)?;
        SingleGear::delete_final_drive_from_car_data(car_data)?;
        let ini_data = car_data.mut_ini_data();
        GearSet::delete_all_from_ini(ini_data);
        ini_data.remove_section("GEARS");
        Ok(())
    }
}

#[derive(Debug)]
pub enum GearConfig {
    GearSets(Vec<GearSet>),
    PerGear(Vec<SingleGear>)
}

impl GearConfig {
    pub fn new_gearset_config(gearset_ratios: &HashMap<String, Vec<f64>>) -> GearConfig {
        let mut gearsets: Vec<GearSet> = Vec::new();
        let mut num_gears = 0;
        for (name, gearset_vec) in gearset_ratios {
            if num_gears == 0 {
                num_gears = gearset_vec.len();
            }
            if gearset_vec.len() >= num_gears {
                gearsets.push(GearSet::new(name.clone(), gearset_vec[0..num_gears].to_vec()));
            } else {
                let mut fixed_ratios = Vec::with_capacity(num_gears);
                fixed_ratios.fill(1.0);
                for (idx, g) in gearset_vec.iter().enumerate() {
                    fixed_ratios[idx] = *g;
                }
                gearsets.push(GearSet::new(name.clone(), fixed_ratios));
            }
        }
        return GearSets(gearsets)
    }

    pub fn new_gears_config(gear_config: Vec<Vec<(String, f64)>>) -> GearConfig {
        let mut gears: Vec<SingleGear> = Vec::new();
        for (idx, ratios_vec) in gear_config.into_iter().enumerate() {
            gears.push(SingleGear::new_gearbox_gear(idx+1, ratios_vec))
        }
        PerGear(gears)
    }

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

    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
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

#[derive(Debug, Clone)]
pub struct SingleGear {
    pub gear_id: String,
    pub ratios_lut: LutProperty<String, f64>,
    pub name: String,
    pub menu_pos_x: f64,
    pub menu_pos_y: f64,
    pub help_data: HelpData,
}

impl SingleGear {
    pub fn new_gearbox_gear(gear_num: usize, ratios: Vec<(String, f64)>) -> SingleGear {
        let section_name = SingleGear::create_gear_key(gear_num);
        let gear_name = SingleGear::create_gear_name(gear_num);
        let ratios_lut = LutProperty::new(
            LutType::File(LutFile::new(format!("{}.rto", gear_name.to_lowercase()), ratios)),
            section_name.clone(),
            "RATIOS".to_owned()
        );
        let help_data = HelpData::Id("HELP_REAR_GEAR".to_owned());
        SingleGear{
            gear_id: section_name,
            ratios_lut,
            name: format!("{} Gear", gear_name),
            menu_pos_x: ((gear_num-1) as f64) * 0.5,
            menu_pos_y: 0.0,
            help_data
        }
    }

    pub fn new_final_drive(ratios: Vec<(String, f64)>) -> SingleGear {
        let section_name = "FINAL_GEAR_RATIO".to_owned();
        let ratios_lut = LutProperty::new(
            LutType::File(LutFile::new("final.rto".to_owned(), ratios)),
            section_name.clone(),
            "RATIOS".to_owned()
        );
        let help_data = HelpData::Id("HELP_REAR_GEAR".to_owned());
        SingleGear{
            gear_id: section_name,
            ratios_lut,
            name: "Final Gear Ratio".to_owned(),
            menu_pos_x: 0.0,
            menu_pos_y: 1.0,
            help_data
        }
    }

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

    pub fn delete_all_gears_from_car_data(car_data: &mut dyn CarDataFile) -> Result<()> {
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

    pub fn delete_final_drive_from_car_data(car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        ini_data.remove_section("FINAL_GEAR_RATIO");
        Ok(())
    }

    pub fn get_index(&self) -> std::result::Result<usize, std::num::ParseIntError> {
        let split = self.gear_id.split_terminator('_').collect_vec();
        split[1].parse::<usize>()
    }

    pub fn create_gear_key(gear_index: usize) -> String {
        format!("GEAR_{}", gear_index)
    }

    pub fn create_gear_name(gear_index: usize) -> String {
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
                        SingleGear::create_gear_name(digit as usize)
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

    pub fn new(name: String, ratios: Vec<f64>) -> GearSet {
        return GearSet { name, ratios }
    }

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

    pub fn ratios(&self) -> &Vec<f64> {
        &self.ratios
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
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use rand::thread_rng;
    use rand::seq::SliceRandom;
    use tracing_subscriber::fmt::format;
    use crate::assetto_corsa::{Car, car, ini_utils};
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

    #[test]
    fn update_no_gears_to_gearset() {
        let orig_ratios = vec![2.615, 1.938, 1.526, 1.286, 1.136, 1.043];
        let updated_ratios = vec![2.615, 1.800, 1.450, 1.200, 1.136, 0.950];
        {
            let mut car = setup_tmp_car_as("no-customizable-gears");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            assert!(!car_setup_data.ini_data.contains_section("GEARS"));
            let mut gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
            let mut gearset_map = HashMap::new();
            gearset_map.insert("original".to_string(), orig_ratios.clone());
            gearset_map.insert("updated".to_string(), updated_ratios.clone());
            gear_data.set_gear_config(Some(GearConfig::new_gearset_config(&gearset_map)));
            gear_data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }

        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        assert!(car_setup_data.ini_data.contains_section("GEARS"));
        assert_eq!(ini_utils::get_value::<i32>(&car_setup_data.ini_data, "GEARS", "USE_GEARSET").expect("Couldn't get gearset in use parameter"), 1);
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        match gear_data.gear_config.as_ref().expect("GearConfig was None") {
            GearConfig::GearSets(sets) => {
                assert_eq!(sets.len(), 2);
                assert_eq!(sets[0].name, "original");
                assert_eq!(sets[0].ratios, orig_ratios);
                assert_eq!(sets[1].name, "updated");
                assert_eq!(sets[1].ratios, updated_ratios);
            }
            GearConfig::PerGear(_) => { assert!(false) }
        }
    }

    #[test]
    fn update_no_gears_to_multiple_gear_ratios() {
        let first_ratios = vec![("2.615".to_owned(), 2.615), ("2.595".to_owned(), 2.595), ("2.500".to_owned(), 2.500)];
        let second_ratios = vec![("1.800".to_owned(), 1.800), ("1.750".to_owned(), 1.750), ("1.700".to_owned(), 1.700)];
        let third_ratios = vec![("1.526".to_owned(), 1.526), ("1.450".to_owned(), 1.450), ("1.400".to_owned(), 1.400)];
        let fourth_ratios = vec![("1.286".to_owned(), 1.286), ("1.250".to_owned(), 1.250), ("1.200".to_owned(), 1.200)];
        let fifth_ratios = vec![("1.136".to_owned(), 1.136), ("1.136".to_owned(), 1.136), ("1.100".to_owned(), 1.100)];
        let sixth_ratios = vec![("1.043".to_owned(), 1.043), ("0.950".to_owned(), 0.950), ("0.900".to_owned(), 0.900)];
        {
            let mut car = setup_tmp_car_as("no-customizable-gears");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            assert!(!car_setup_data.ini_data.contains_section("GEARS"));
            let mut gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
            let config = GearConfig::new_gears_config(vec![first_ratios.clone(), second_ratios.clone(), third_ratios.clone(), fourth_ratios.clone(), fifth_ratios.clone(), sixth_ratios.clone()]);
            gear_data.set_gear_config(Some(config));
            gear_data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }
        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        assert!(car_setup_data.ini_data.contains_section("GEARS"));
        assert_eq!(ini_utils::get_value::<i32>(&car_setup_data.ini_data, "GEARS", "USE_GEARSET").expect("Couldn't get gearset in use parameter"), 0);
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        match gear_data.gear_config.as_ref().expect("GearConfig was None") {
            GearConfig::GearSets(sets) => { assert!(false) },
            GearConfig::PerGear(gears) => {
                assert_eq!(gears.len(), 6);
                let mut next_gear_num = 1;
                for (gear, expected_ratios) in gears.iter().zip(vec![&first_ratios, &second_ratios, &third_ratios, &fourth_ratios, &fifth_ratios, &sixth_ratios]) {
                    assert_eq!(gear.gear_id, format!("GEAR_{}", next_gear_num));
                    for (actual, expected_pair) in gear.ratios_lut.clone_values().iter().zip(expected_ratios) {
                        assert_eq!(*actual, expected_pair.1);
                    }
                    next_gear_num+=1;
                }
            }
        }
    }

    #[test]
    fn update_gearset_to_multiple_gear_ratios() {
        let first_ratios = vec![("2.615".to_owned(), 2.615), ("2.595".to_owned(), 2.595), ("2.500".to_owned(), 2.500)];
        let second_ratios = vec![("1.800".to_owned(), 1.800), ("1.750".to_owned(), 1.750), ("1.700".to_owned(), 1.700)];
        let third_ratios = vec![("1.526".to_owned(), 1.526), ("1.450".to_owned(), 1.450), ("1.400".to_owned(), 1.400)];
        let fourth_ratios = vec![("1.286".to_owned(), 1.286), ("1.250".to_owned(), 1.250), ("1.200".to_owned(), 1.200)];
        let fifth_ratios = vec![("1.136".to_owned(), 1.136), ("1.136".to_owned(), 1.136), ("1.100".to_owned(), 1.100)];
        let sixth_ratios = vec![("1.043".to_owned(), 1.043), ("0.950".to_owned(), 0.950), ("0.900".to_owned(), 0.900)];
        {
            let mut car = setup_tmp_car_as("gearset");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            assert_eq!(GearSet::load_all_from_car_data(&car_setup_data).unwrap().len(), 3);
            let mut gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
            let config = GearConfig::new_gears_config(vec![first_ratios.clone(), second_ratios.clone(), third_ratios.clone(), fourth_ratios.clone(), fifth_ratios.clone(), sixth_ratios.clone()]);
            gear_data.set_gear_config(Some(config));
            gear_data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }
        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        assert!(car_setup_data.ini_data.contains_section("GEARS"));
        assert_eq!(ini_utils::get_value::<i32>(&car_setup_data.ini_data, "GEARS", "USE_GEARSET").expect("Couldn't get gearset in use parameter"), 0);
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        match gear_data.gear_config.as_ref().expect("GearConfig was None") {
            GearConfig::GearSets(sets) => { assert!(false) },
            GearConfig::PerGear(gears) => {
                assert_eq!(gears.len(), 6);
                let mut next_gear_num = 1;
                for (gear, expected_ratios) in gears.iter().zip(vec![&first_ratios, &second_ratios, &third_ratios, &fourth_ratios, &fifth_ratios, &sixth_ratios]) {
                    assert_eq!(gear.gear_id, format!("GEAR_{}", next_gear_num));
                    for (actual, expected_pair) in gear.ratios_lut.clone_values().iter().zip(expected_ratios) {
                        assert_eq!(*actual, expected_pair.1);
                    }
                    next_gear_num+=1;
                }
            }
        }
    }

    #[test]
    fn update_gears_to_gearset() {
        let orig_ratios = vec![2.615, 1.938, 1.526, 1.286, 1.136, 1.043];
        let updated_ratios = vec![2.615, 1.800, 1.450, 1.200, 1.136, 0.950];
        {
            let mut car = setup_tmp_car_as("gears-per-gear-ratio-file");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            let mut gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
            let mut gearset_map = HashMap::new();
            gearset_map.insert("original".to_string(), orig_ratios.clone());
            gearset_map.insert("updated".to_string(), updated_ratios.clone());
            gear_data.set_gear_config(Some(GearConfig::new_gearset_config(&gearset_map)));
            gear_data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }

        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        assert!(car_setup_data.ini_data.contains_section("GEARS"));
        assert_eq!(ini_utils::get_value::<i32>(&car_setup_data.ini_data, "GEARS", "USE_GEARSET").expect("Couldn't get gearset in use parameter"), 1);
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        match gear_data.gear_config.as_ref().expect("GearConfig was None") {
            GearConfig::GearSets(sets) => {
                assert_eq!(sets.len(), 2);
                assert_eq!(sets[0].name, "original");
                assert_eq!(sets[0].ratios, orig_ratios);
                assert_eq!(sets[1].name, "updated");
                assert_eq!(sets[1].ratios, updated_ratios);
            }
            GearConfig::PerGear(_) => { assert!(false) }
        }
    }

    #[test]
    fn clear_customizable_setup() {
        {
            let mut car = setup_tmp_car_as("gears-per-gear-ratio-file");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            let mut data = GearData::load_from_parent(&car_setup_data).unwrap();
            assert!(data.gear_config.is_some());
            data.clear_gear_config();
            data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }

        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        assert!(!car_setup_data.ini_data.contains_section("GEARS"));
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        assert!(gear_data.gear_config.is_none());
        assert!(gear_data.final_drive.is_some());
    }

    #[test]
    fn update_final_drive() {
        let final_ratios = vec![("4.500".to_owned(), 4.500), ("4.300".to_owned(), 4.300), ("4.000".to_owned(), 4.000)];
        {
            let mut car = setup_tmp_car_as("only-final-drive");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            let mut gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
            let final_gear = SingleGear::new_final_drive(final_ratios.clone());
            gear_data.set_final_drive(Some(final_gear));
            gear_data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }
        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        assert!(gear_data.gear_config.is_none());
        let final_drive = gear_data.final_drive.as_ref().unwrap();
        assert_eq!(final_drive.ratios_lut.num_entries(), 3);
        for (actual, expected) in final_drive.ratios_lut.to_vec().iter().zip(final_ratios) {
            assert_eq!(actual.0, expected.0);
            assert_eq!(actual.1, expected.1);
        }
    }

    #[test]
    fn clear_final_drive() {
        {
            let mut car = setup_tmp_car_as("gears-per-gear-ratio-file");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            let mut data = GearData::load_from_parent(&car_setup_data).unwrap();
            assert!(data.final_drive.is_some());
            data.clear_final_drive();
            data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }

        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        assert!(gear_data.gear_config.is_some());
        assert!(gear_data.final_drive.is_none());
    }

    #[test]
    fn clear_only_final_drive() {
        {
            let mut car = setup_tmp_car_as("only-final-drive");
            let mut car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
            let mut data = GearData::load_from_parent(&car_setup_data).unwrap();
            assert!(data.final_drive.is_some());
            data.clear_final_drive();
            data.update_car_data(&mut car_setup_data).unwrap();
            car_setup_data.write().unwrap();
        }

        let mut car = get_tmp_car();
        let car_setup_data = Setup::from_car(&mut car).unwrap().unwrap();
        assert!(!car_setup_data.ini_data.contains_section("GEARS"));
        let gear_data = GearData::load_from_parent(&car_setup_data).unwrap();
        assert!(gear_data.gear_config.is_none());
        assert!(gear_data.final_drive.is_none());
    }
}
