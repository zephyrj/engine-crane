use std::fmt;
use std::default::Default;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use configparser::ini::Ini;


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

#[derive(Debug, Clone)]
pub struct InvalidCarError {
    reason: String
}

impl fmt::Display for InvalidCarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid car config: {}", self.reason)
    }
}

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

#[derive(Debug)]
#[derive(Default)]
pub struct Car {
    root_path: OsString,
    pub(crate) ini_config: Ini,
    version: CarVersion,
    screen_name: String,
    total_mass: u32,
    fuel_consumption: Option<f64>,
    default_fuel: u32,
    max_fuel: u32
}

impl Car {
    pub fn load_from_path(car_folder_path: &Path) -> Result<Car, InvalidCarError> {
        let mut car = Car {
            root_path: OsString::from(car_folder_path),
            ini_config: Ini::new(),
            ..Default::default()
        };
        let mut ini_path = PathBuf::from(car_folder_path);
        ini_path.push("data");
        ini_path.push("car.ini");
        match car.ini_config.load(ini_path.as_path()) {
            Err(err_str) =>  {
                return Err(InvalidCarError {
                    reason: String::from(format!("Failed to decode {}: {}",
                                                 ini_path.display(),
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

