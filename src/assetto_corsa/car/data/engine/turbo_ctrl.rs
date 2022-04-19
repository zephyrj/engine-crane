use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io;
use std::path::Path;
use std::str::FromStr;
use csv::Terminator;

use crate::assetto_corsa::traits::DataInterface;
use crate::assetto_corsa::error::{Error, ErrorKind, PropertyParseError, Result};
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::car::data::engine::Turbo;
use crate::assetto_corsa::car::lut_utils;
use crate::assetto_corsa::ini_utils::{Ini, IniUpdater};


#[derive(Debug)]
pub struct TurboControllers {
    index: isize,
    ini_config: Ini,
    controllers: Vec<TurboController>
}

impl TurboControllers {
    pub fn new(index: isize) -> TurboControllers {
        TurboControllers {
            index,
            ini_config: Ini::new(),
            controllers: Vec::new()
        }
    }

    pub fn load_all_from_data(data_source: &dyn DataInterface, ini_data: &Ini) -> Result<HashMap<isize, TurboControllers>> {
        let turbo_count: isize = Turbo::count_turbo_sections(ini_data);
        if turbo_count == 0 {
            return Ok(HashMap::new());
        }
        let mut out_map = HashMap::new();
        for turbo_idx in 0..turbo_count {
            match TurboControllers::load_controller_index_from_dir(data_source, turbo_idx)? {
                None => { continue }
                Some(turbo_ctrls) => {
                    out_map.insert(turbo_idx, turbo_ctrls); }
            }
        }
        Ok(out_map)
    }

    fn load_controller_index_from_dir(data_source: &dyn DataInterface, index: isize) -> Result<Option<TurboControllers>> {
        match data_source.get_file_data(&TurboControllers::get_controller_ini_filename(index)) {
            Ok(data) => {
                let ini_config = Ini::load_from_string(String::from_utf8_lossy(data.as_slice()).to_string());

                let turbo_controller_count: isize = TurboControllers::count_turbo_controller_sections(&ini_config);
                let mut controller_vec: Vec<TurboController> = Vec::new();
                for idx in 0..turbo_controller_count {
                    controller_vec.push(TurboController::load_from_ini(&ini_config, idx, data_source)?);
                }

                Ok(Some(
                    TurboControllers {
                        index,
                        ini_config,
                        controllers: controller_vec
                    }
                ))
            }
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => {
                        return Ok(None)
                    },
                    _ => { return Err(Error::from(e)) }
                }
            }
        }
    }

    pub fn add_controller(&mut self, controller: TurboController) -> Result<()> {
        controller.update_ini(&mut self.ini_config).map_err(|err_str| {
            Error::new(ErrorKind::InvalidUpdate,
                       format!("Failed to add turbo controller with index {} to {}. {}",
                               controller.index(), self.filename(), err_str ))
        })?;
        self.controllers.push(controller);
        Ok(())
    }

    pub fn filename(&self) -> String {
        TurboControllers::get_controller_ini_filename(self.index)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.ini_config.to_bytes()
    }

    pub fn write_to_dir(&self, dir: &Path) -> Result<()> {
        self.ini_config.write_to_file(&dir.join(Path::new(&self.filename())))?;
        Ok(())
    }

    pub fn get_controller_ini_filename(index: isize) -> String {
        format!("ctrl_turbo{}.ini", index)
    }

    fn count_turbo_controller_sections(ini: &Ini) -> isize {
        let mut count = 0;
        loop {
            if !ini.contains_section(TurboController::get_controller_section_name(count).as_str()) {
                return count;
            }
            count += 1;
        }
    }

    fn update(&mut self) -> std::result::Result<(), String> {
        for controller in &self.controllers {
            controller.update_ini(&mut self.ini_config)?;
        }
        Ok(())
    }
}

impl Display for TurboControllers {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ini_config.to_string())
    }
}


#[derive(Debug)]
pub struct TurboController {
    index: isize,
    input: ControllerInput,
    combinator: ControllerCombinator,
    lut: Vec<(f64, f64)>, // TODO create Enum
    filter: f64,
    up_limit: f64,
    down_limit: f64
}

impl TurboController {
    pub fn load_from_ini(ini: &Ini, idx: isize, data_source: &dyn DataInterface) -> Result<TurboController> {
        let section_name = TurboController::get_controller_section_name(idx);
        let lut = lut_utils::load_lut_from_property_value(
            ini_utils::get_mandatory_property(ini, &section_name, "LUT")?,
            data_source
        ).map_err(
            |err_str| {
                Error::new(ErrorKind::InvalidCar,
                           format!("Failed to load turbo controller with index {}: {}", idx, err_str ))
            })?;


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

    pub fn new(index: isize,
               input: ControllerInput,
               combinator: ControllerCombinator,
               lut: Vec<(f64, f64)>,
               filter: f64,
               up_limit: f64,
               down_limit: f64) -> TurboController {
        TurboController {
            index,
            input,
            combinator,
            lut,
            filter,
            up_limit,
            down_limit
        }
    }

    pub fn index(&self) -> isize {
        self.index
    }

    fn get_controller_section_name(index: isize) -> String {
        format!("CONTROLLER_{}", index)
    }
}

impl IniUpdater for TurboController {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        let section_name = TurboController::get_controller_section_name(self.index);
        ini_utils::set_value(ini_data, &section_name, "INPUT", &self.input);
        ini_utils::set_value(ini_data, &section_name, "COMBINATOR", &self.combinator);
        ini_utils::set_value(ini_data,
                             &section_name,
                             "LUT",
                             lut_utils::write_lut_to_property_value(&self.lut, b'=', Terminator::Any(b'|'))?);
        ini_utils::set_float(ini_data, &section_name, "FILTER", self.filter, 3);
        ini_utils::set_value(ini_data, &section_name, "UP_LIMIT", self.up_limit);
        ini_utils::set_value(ini_data, &section_name, "DOWN_LIMIT", self.down_limit);
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

#[derive(Debug)]
pub struct ControllerCombinatorParseError{
    invalid_value: String
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
