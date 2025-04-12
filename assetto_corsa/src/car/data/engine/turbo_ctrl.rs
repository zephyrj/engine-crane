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

use std::fmt::{Display, Formatter};

use std::str::FromStr;


use crate::traits::{CarDataFile, CarDataUpdater, DataInterface};
use crate::error::{Error, ErrorKind, PropertyParseError, Result};
use crate::{Car, ini_utils};

use crate::car::lut_utils::{InlineLut, LutType};
use crate::car::structs::LutProperty;
use crate::ini_utils::Ini;

#[derive(Debug)]
pub struct TurboControllerFile<'a> {
    car: &'a mut Car,
    turbo_index: usize,
    ini_data: Ini
}

impl<'a> CarDataFile for TurboControllerFile<'a> {
    fn ini_data(&self) -> &Ini {
        &self.ini_data
    }
    fn mut_ini_data(&mut self) -> &mut Ini {
        &mut self.ini_data
    }
    fn data_interface(&self) -> &dyn DataInterface {
        self.car.data_interface()
    }
    fn mut_data_interface(&mut self) -> &mut dyn DataInterface {
        self.car.mut_data_interface()
    }
}

impl<'a> TurboControllerFile<'a> {
    pub fn new(car: & mut Car, turbo_index: usize) -> TurboControllerFile {
        TurboControllerFile {
            car,
            turbo_index,
            ini_data: Ini::new(),
        }
    }

    pub fn from_car(car: & mut Car, turbo_index: usize) -> Result<Option<TurboControllerFile>> {
        match car.data_interface.get_original_file_data(&TurboControllerFile::get_controller_ini_filename(turbo_index)) {
            Ok(data_option) => {
                match data_option {
                    None => Ok(None),
                    Some(file_data) => {
                        Ok(Some(TurboControllerFile {
                            car,
                            turbo_index,
                            ini_data: Ini::load_from_string(String::from_utf8_lossy(file_data.as_slice()).to_string())
                        }))
                    }
                }
            }
            Err(e) => {
                Err(Error::new(ErrorKind::InvalidCar,
                               format!("error reading {} data. {}",
                                       &TurboControllerFile::get_controller_ini_filename(turbo_index),
                                       e.to_string())))
            }
        }
    }

    pub fn delete_from_car(car: &mut Car, turbo_index: usize) -> Result<()> {
        if let Some(mut ctrl_file) = TurboControllerFile::from_car(car, turbo_index)? {
            ctrl_file.delete_all_controller_sections()?;
        }
        car.mut_data_interface().remove_file(&TurboControllerFile::get_controller_ini_filename(turbo_index));
        Ok(())
    }

    pub fn write(&mut self) -> Result<()> {
        let filename = self.filename();
        let bytes = self.ini_data.to_bytes();
        let data_interface = self.car.mut_data_interface();
        data_interface.update_file_data(&filename, bytes);
        data_interface.write()?;
        Ok(())
    }

    pub fn filename(&self) -> String {
        TurboControllerFile::get_controller_ini_filename(self.turbo_index)
    }

    pub fn num_controller_sections(&self) -> usize {
        let mut count: usize = 0;
        loop {
            if self.ini_data.contains_section(&TurboController::get_controller_section_name(count)) {
                return count;
            }
            count += 1;
        }
    }

    pub fn delete_all_controller_sections(&mut self) -> Result<()> {
        for section_index in 0..self.num_controller_sections() {
            self.delete_controller_section(section_index)?;
        }
        Ok(())
    }

    pub fn delete_controller_section(&mut self, section_index: usize) -> Result<()> {
        let t = TurboController::load_from_parent(section_index, self)?;
        t.delete(self);
        Ok(())
    }

    pub fn get_controller_ini_filename(index: usize) -> String {
        format!("ctrl_turbo{}.ini", index)
    }
}

pub fn delete_all_turbo_controllers_from_car(car: &mut Car) -> Result<()> {
    let mut idx = 0;
    while car.data_interface().contains_file(&TurboControllerFile::get_controller_ini_filename(idx)) {
        TurboControllerFile::delete_from_car(car, idx)?;
        idx += 1;
    }
    Ok(())
}

#[derive(Debug)]
pub struct TurboController {
    index: usize,
    input: ControllerInput,
    combinator: ControllerCombinator,
    lut: LutProperty<f64, f64>,
    filter: f64,
    up_limit: f64,
    down_limit: f64
}

impl TurboController {
    pub fn load_from_parent(idx: usize, parent_data: &dyn CarDataFile) -> Result<TurboController> {
        let ini = parent_data.ini_data();
        let section_name = TurboController::get_controller_section_name(idx);
        let lut = LutProperty::mandatory_from_ini(
            section_name.clone(),
            "LUT".to_owned(),
            ini,
            parent_data.data_interface()).map_err(
                |err_str| {
                    Error::new(ErrorKind::InvalidCar,
                               format!("Failed to load turbo controller with index {}: {}", idx, err_str ))
                }
            )?;
        Ok(TurboController {
            index: idx,
            input: ini_utils::get_mandatory_property(ini, &section_name, "INPUT")?,
            combinator: ini_utils::get_mandatory_property(ini, &section_name, "COMBINATOR")?,
            lut,
            filter: ini_utils::get_mandatory_property(ini, &section_name, "FILTER")?,
            up_limit: ini_utils::get_mandatory_property(ini, &section_name, "UP_LIMIT")?,
            down_limit: ini_utils::get_mandatory_property(ini, &section_name, "DOWN_LIMIT")?
        })
    }

    pub fn new(index: usize,
               input: ControllerInput,
               combinator: ControllerCombinator,
               lut: Vec<(f64, f64)>,
               filter: f64,
               up_limit: f64,
               down_limit: f64) -> TurboController {
        let lut_property= LutProperty::new(
            LutType::Inline(InlineLut::from_vec(lut)),
            TurboController::get_controller_section_name(index),
            "LUT".to_owned());
        TurboController {
            index,
            input,
            combinator,
            lut: lut_property,
            filter,
            up_limit,
            down_limit
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn delete(&self, controller_file: &mut TurboControllerFile) {
        self.lut.delete_from_car_data(controller_file);
        controller_file.mut_ini_data().remove_section(&self.section_name())
    }

    pub fn section_name(&self) -> String {
        TurboController::get_controller_section_name(self.index)
    }

    pub fn get_lut(&self) -> &LutProperty<f64, f64> {
        &self.lut
    }

    fn get_controller_section_name(index: usize) -> String {
        format!("CONTROLLER_{}", index)
    }
}

impl CarDataUpdater for TurboController {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        {
            let ini_data = car_data.mut_ini_data();
            let section_name = self.section_name();
            ini_utils::set_value(ini_data, &section_name, "INPUT", &self.input);
            ini_utils::set_value(ini_data, &section_name, "COMBINATOR", &self.combinator);
            ini_utils::set_float(ini_data, &section_name, "FILTER", self.filter, 3);
            ini_utils::set_value(ini_data, &section_name, "UP_LIMIT", self.up_limit);
            ini_utils::set_value(ini_data, &section_name, "DOWN_LIMIT", self.down_limit);
        }
        self.lut.update_car_data(car_data)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ControllerInput {
    Rpms,
    Gas,
    Gear
}

impl Default for ControllerInput {
    fn default() -> Self { ControllerInput::Rpms }
}

impl ControllerInput {
    pub const RPMS_VALUE: &'static str = "RPMS";
    pub const GAS_VALUE: &'static str = "GAS";
    pub const GEAR_VALUE: &'static str= "GEAR";

    pub fn as_str(&self) -> &'static str {
        match self {
            ControllerInput::Rpms => ControllerInput::RPMS_VALUE,
            ControllerInput::Gas => ControllerInput::GAS_VALUE,
            ControllerInput::Gear => ControllerInput::GEAR_VALUE
        }
    }
}

impl FromStr for ControllerInput {
    type Err = PropertyParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            ControllerInput::RPMS_VALUE => Ok(ControllerInput::Rpms),
            ControllerInput::GAS_VALUE => Ok(ControllerInput::Gas),
            ControllerInput::GEAR_VALUE => Ok(ControllerInput::Gear),
            _ => Err(PropertyParseError::new(s))
        }
    }
}

impl Display for ControllerInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
pub enum ControllerCombinator {
    Add,
    Mult
}

impl ControllerCombinator {
    pub const ADD_VALUE :&'static str = "ADD";
    pub const MULT_VALUE :&'static str = "MULT";

    pub fn as_str(&self) -> &'static str {
        match self {
            ControllerCombinator::Add => ControllerCombinator::ADD_VALUE,
            ControllerCombinator::Mult => ControllerCombinator::MULT_VALUE
        }
    }
}

impl FromStr for ControllerCombinator {
    type Err = PropertyParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            ControllerCombinator::ADD_VALUE => Ok(ControllerCombinator::Add),
            ControllerCombinator::MULT_VALUE => Ok(ControllerCombinator::Mult),
            _ => Err(PropertyParseError::new(s))
        }
    }
}

impl Display for ControllerCombinator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
