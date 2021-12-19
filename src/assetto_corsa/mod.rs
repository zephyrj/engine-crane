use std::collections::HashMap;
use std::{fmt, fs, io};
use std::default::Default;
use std::error;
use std::ffi::OsString;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use configparser::ini::Ini;
use serde_json::Value;

use crate::steam;

pub const STEAM_GAME_NAME: &str = "assettocorsa";
pub const STEAM_GAME_ID: i64 = 244210;

pub fn is_installed() -> bool {
    if let Some(install_path) = steam::get_install_path(STEAM_GAME_NAME) {
        install_path.is_dir()
    } else {
        false
    }
}

pub fn get_installed_cars_path() -> Option<PathBuf> {
    if let Some(mut install_path) = steam::get_install_path(STEAM_GAME_NAME) {
        for path in ["content", "cars"] {
            install_path.push(path)
        }
        Some(install_path)
    } else {
        None
    }
}

#[derive(Debug)]
pub struct Cars {
    unpacked_cars: Vec<Car>,
    packed_car_dirs: Vec<OsString>
}

impl Cars {
    pub fn load() {

    }
}


#[derive(Debug, Clone)]
pub struct InvalidCarError {
    reason: String
}

impl fmt::Display for InvalidCarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid car config: {}", self.reason)
    }
}

impl error::Error for InvalidCarError {}


#[derive(Debug)]
pub enum CarVersion {
    Default,
    CspExtendedPhysics
}

impl Default for CarVersion {
    fn default() -> Self {
        CarVersion::Default
    }
}

impl CarVersion {
    pub const VERSION_1 :&'static str = "1";
    pub const CSP_EXTENDED_2 : &'static str = "extended-2";

    fn from_string(s: &str) -> CarVersion {
        match s {
            CarVersion::VERSION_1 => CarVersion::Default,
            CarVersion::CSP_EXTENDED_2 => CarVersion::CspExtendedPhysics,
            _ => CarVersion::Default
        }
    }

    fn to_string(&self) -> &str {
        match self {
            CarVersion::Default => CarVersion::VERSION_1,
            CarVersion::CspExtendedPhysics => CarVersion::CSP_EXTENDED_2
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum SpecValue<'a> {
    Bhp(&'a str),
    Torque(&'a str),
    Weight(&'a str),
    TopSpeed(&'a str),
    Acceleration(&'a str),
    PWRatio(&'a str),
    Range(i32)
}

impl<'a> SpecValue<'a> {
    fn parse(key: &str, value: &'a Value) -> Option<SpecValue<'a>> {
        match key {
            "bhp" => if let Some(val) = value.as_str() { return Some(SpecValue::Bhp(val)); },
            "torque" => if let Some(val) = value.as_str() { return Some(SpecValue::Torque(val)); },
            "weight" => if let Some(val) = value.as_str() { return Some(SpecValue::Weight(val)); },
            "topspeed" => if let Some(val) = value.as_str() { return Some(SpecValue::TopSpeed(val)); },
            "acceleration" => if let Some(val) = value.as_str() { return Some(SpecValue::Acceleration(val)); },
            "pwratio" => if let Some(val) = value.as_str() { return Some(SpecValue::PWRatio(val)); },
            "range" => if let Some(val) = value.as_i64() { return Some(SpecValue::Range(val as i32)); },
            _ => {}
        }
        None
    }
}

#[derive(Debug)]
#[derive(Default)]
pub struct UiInfo {
    ui_info_path: OsString,
    json_config: serde_json::Value,
    car_class: String
}

impl UiInfo {
    fn load(ui_json_path: &Path) -> Result<UiInfo, Box<dyn error::Error>> {
        let ui_info_string = fs::read_to_string(ui_json_path)?;
        let ui_info = UiInfo { ui_info_path: OsString::from(ui_json_path),
            json_config: serde_json::from_str(ui_info_string.replace("\r\n", " ").replace("\t", "  ").as_str())?,
            ..Default::default() };
        Ok(ui_info)
    }

    pub fn name(&self) -> Option<&str> {
        self.get_json_string("name")
    }

    pub fn brand(&self) -> Option<&str> {
        self.get_json_string("brand")
    }

    pub fn description(&self) -> Option<&str> {
        self.get_json_string("description")
    }

    pub fn class(&self) -> Option<&str> {
        self.get_json_string("class")
    }

    pub fn tags(&self) -> Option<Vec<&str>> {
        let mut return_vec: Vec<&str> = Vec::new();
        if let Some(value) = self.json_config.get("tags") {
            if let Some(list) = value.as_array() {
                for val in list {
                    if let Some(v) = val.as_str() {
                        return_vec.push(v);
                    }
                }
                Some(return_vec)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn specs(&self) -> Option<HashMap<&str, SpecValue>> {
        let mut return_map: HashMap<&str, SpecValue> = HashMap::new();
        if let Some(value) = self.json_config.get("specs") {
            if let Some(map) = value.as_object() {
                map.iter().for_each(|(k, v)| {
                    if let Some(val) = SpecValue::parse(k.as_str(), v) {
                        return_map.insert(k.as_str(), val);
                    }
                });
                Some(return_map)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn torque_curve(&self) -> Option<Vec<Vec<&str>>> {
        self.load_curve_data("torqueCurve")
    }

    pub fn power_curve(&self) -> Option<Vec<Vec<&str>>> {
        self.load_curve_data("powerCurve")
    }

    fn get_json_string(&self, key: &str) -> Option<&str> {
        if let Some(value) = self.json_config.get(key) {
            value.as_str()
        } else {
            None
        }
    }

    fn load_curve_data(&self, key: &str) -> Option<Vec<Vec<&str>>> {
        let mut outer_vec: Vec<Vec<&str>> = Vec::new();
        if let Some(value) = self.json_config.get(key) {
            if let Some(out_vec) = value.as_array() {
                out_vec.iter().for_each(|x: &Value| {
                    let mut inner_vec: Vec<&str> = Vec::new();
                    if let Some(v2) = x.as_array() {
                        v2.iter().for_each(|y: &Value| {
                            if let Some(val) = y.as_str() {
                                inner_vec.push(val);
                            }
                        });
                        outer_vec.push(inner_vec);
                    }
                });
                Some(outer_vec)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
#[derive(Default)]
pub struct Car {
    root_path: OsString,
    ini_config: Ini,
    pub ui_info: UiInfo,
    version: CarVersion,
    screen_name: String,
    total_mass: u32,
    fuel_consumption: Option<f64>,
    default_fuel: u32,
    max_fuel: u32
}

impl Car {
    pub fn load_from_path(car_folder_path: &Path) -> Result<Car, InvalidCarError> {
        let ui_info_path = car_folder_path.join(["ui", "ui_car.json"].iter().collect::<PathBuf>());
        let ui_info = match UiInfo::load(ui_info_path.as_path()) {
            Ok(result) => result,
            Err(e) => { return Err(InvalidCarError { reason: format!("Failed to parse {}: {}", ui_info_path.display(), e.to_string()) }) }
        };
        let mut car = Car {
            root_path: OsString::from(car_folder_path),
            ini_config: Ini::new(),
            ui_info,
            ..Default::default()
        };
        let car_ini_path = car_folder_path.join(["data", "car.ini"].iter().collect::<PathBuf>());
        match car.ini_config.load(car_ini_path.as_path()) {
            Err(err_str) =>  {
                return Err(InvalidCarError {
                    reason: String::from(format!("Failed to decode {}: {}",
                                                 car_ini_path.display(),
                                                 err_str)) })
            },
            Ok(_) => {}
        }
        car.version = CarVersion::from_string(&car.get_ini_string("header", "version")?);
        car.screen_name = car.get_ini_string("info", "screen_name")?;
        car.total_mass = car.get_ini_int("basic", "totalmass")? as u32;
        car.default_fuel = car.get_ini_int("fuel", "fuel")? as u32;
        car.max_fuel = car.get_ini_int("fuel", "max_fuel")? as u32;
        match car.get_ini_float("fuel", "consumption") {
            Ok(fuel_consumption) => car.fuel_consumption = Some(fuel_consumption),
            Err(_) => {}
        }
        Ok(car)
    }

    fn get_ini_string(&self, section: &str, key: &str) -> Result<String, InvalidCarError> {
        if let Some(var) = self.ini_config.get(section, key) {
            Ok(var)
        } else {
            Err(InvalidCarError { reason: String::from(format!("Missing field {} -> {}",
                                                               section, key)) })
        }
    }

    fn get_ini_int(&self, section: &str, key: &str) -> Result<i64, InvalidCarError> {
        Car::handle_number_result(self.ini_config.getint(section, key), section, key)
    }

    fn get_ini_float(&self, section: &str, key: &str) -> Result<f64, InvalidCarError> {
        Car::handle_number_result(self.ini_config.getfloat(section, key), section, key)
    }

    fn handle_number_result<T>(result: Result<Option<T>, String>, section: &str, key: &str) -> Result<T, InvalidCarError> {
        match result {
            Ok(var) => {
                if let Some(ret_var) = var {
                    Ok(ret_var)
                } else {
                    Err(InvalidCarError {
                        reason: String::from(format!("Missing field {} -> {}",
                                                     section, key)) })
                }
            },
            Err(str) => Err(InvalidCarError {
                reason: String::from(format!("Failed to parse field {} -> {}: {}",
                                             section, key, str)) })
        }
    }
}
