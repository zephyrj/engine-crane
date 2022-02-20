use std::collections::HashMap;
use std::fs;
use std::default::Default;
use std::ffi::OsString;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use serde_json::Value;
use crate::assetto_corsa::drivetrain::Drivetrain;
use crate::assetto_corsa::error::{Result, Error, ErrorKind, FieldParseError};
use crate::assetto_corsa::engine::{Engine};
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::Ini;

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

    fn as_str(&self) -> &'static str {
        match self {
            CarVersion::Default => CarVersion::VERSION_1,
            CarVersion::CspExtendedPhysics => CarVersion::CSP_EXTENDED_2
        }
    }
}

impl FromStr for CarVersion {
    type Err = FieldParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            CarVersion::VERSION_1 => Ok(CarVersion::Default),
            CarVersion::CSP_EXTENDED_2 => Ok(CarVersion::CspExtendedPhysics),
            _ => Err(FieldParseError::new(s))
        }
    }
}

impl Display for CarVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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
    fn load(ui_json_path: &Path) -> Result<UiInfo> {
        let ui_info_string = match fs::read_to_string(ui_json_path) {
            Ok(str) => { str }
            Err(e) => {
                return Err( Error::new(ErrorKind::InvalidCar,
                                       String::from(format!("Failed to read {}: {}",
                                                            ui_json_path.display(),
                                                            e.to_string()))) )
            }
        };
        let json_config = match serde_json::from_str(ui_info_string.replace("\r\n", " ").replace("\t", "  ").as_str()) {
            Ok(decoded_json) => { decoded_json },
            Err(e) => {
                return Err( Error::new(ErrorKind::InvalidCar,
                                       String::from(format!("Failed to decode {}: {}",
                                                            ui_json_path.display(),
                                                            e.to_string()))) )
            }
        };
        let ui_info = UiInfo { ui_info_path: OsString::from(ui_json_path),
            json_config,
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
pub struct Car {
    root_path: OsString,
    ini_config: Ini,
    pub ui_info: UiInfo,
    engine: Engine,
    drivetrain: Drivetrain
}

impl Car {
    pub fn version(&self) -> Option<CarVersion> {
        ini_utils::get_value(&self.ini_config, "header", "version")
    }

    pub fn screen_name(&self) -> Option<String> {
        ini_utils::get_value(&self.ini_config, "info","screen_name")
    }

    pub fn total_mass(&self) -> Option<u32> {
        ini_utils::get_value(&self.ini_config, "basic","totalmass")
    }

    pub fn default_fuel(&self) -> Option<u32> {
        ini_utils::get_value(&self.ini_config, "fuel","fuel")
    }

    pub fn max_fuel(&self) -> Option<u32> {
        ini_utils::get_value(&self.ini_config, "fuel","max_fuel")
    }

    pub fn fuel_consumption(&self) -> Option<f64> {
        ini_utils::get_value(&self.ini_config, "fuel","consumption")
    }

    pub fn load_from_path(car_folder_path: &Path) -> Result<Car> {
        let ui_info_path = car_folder_path.join(["ui", "ui_car.json"].iter().collect::<PathBuf>());
        let ui_info = match UiInfo::load(ui_info_path.as_path()) {
            Ok(result) => result,
            Err(e) => { return Err(Error::new(ErrorKind::InvalidCar,
                                              format!("Failed to parse {}: {}",
                                                      ui_info_path.display(),
                                                      e.to_string()))) }
        };
        let car_ini_path = car_folder_path.join(["data", "car.ini"].iter().collect::<PathBuf>());
        let mut car = Car {
            root_path: OsString::from(car_folder_path),
            ini_config: Ini::load_from_file(car_ini_path.as_path()).map_err(|err| {
                Error::new(ErrorKind::InvalidCar,
                           format!("Failed to decode {}: {}",
                                   car_ini_path.display(),
                                   err.to_string()))
            })?,
            ui_info,
            engine: Engine::load_from_dir(car_folder_path.join("data").as_path())?,
            drivetrain: Drivetrain::load_from_path(car_folder_path.join("data").as_path())?
        };
        Ok(car)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::assetto_corsa::car::Car;

    #[test]
    fn load_car() -> Result<(), String> {
        let path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/zephyr_za401/");
        match Car::load_from_path(&path) {
            Ok(_) => {
                Ok(())
            }
            Err(e) => { Err(e.to_string()) }
        }
    }
}
