use std::cell::RefCell;
use std::ffi::OsString;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
use configparser::ini::Ini;
use crate::assetto_corsa::error::{Result, Error, ErrorKind, FieldParseError};
use crate::assetto_corsa::file_utils::load_ini_file;
use crate::assetto_corsa::ini_utils;


pub enum DriveType {
    RWD,
    FWD,
    AWD
}

impl DriveType {
    pub const RWD_VALUE: &'static str = "RWD";
    pub const FWD_VALUE: &'static str = "FWD";
    pub const AWD_VALUE: &'static str = "AWD";

    pub fn as_str(&self) -> &'static str {
        match self {
            DriveType::RWD => { DriveType::RWD_VALUE }
            DriveType::FWD => { DriveType::FWD_VALUE }
            DriveType::AWD => { DriveType::AWD_VALUE }
        }
    }

    pub fn mechanical_efficiency(&self) -> f64 {
        match self {
            DriveType::RWD => { 0.85 }
            DriveType::FWD => { 0.9 }
            DriveType::AWD => { 0.75 }
        }
    }
}

impl FromStr for DriveType {
    type Err = FieldParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            DriveType::RWD_VALUE => Ok(DriveType::RWD),
            DriveType::FWD_VALUE => Ok(DriveType::FWD),
            DriveType::AWD_VALUE => Ok(DriveType::AWD),
            _ => Err(FieldParseError::new(s))
        }
    }
}

impl ToString for DriveType {
    fn to_string(&self) -> String {
        String::from(self.as_str())
    }
}


#[derive(Debug)]
pub struct Drivetrain {
    data_dir: OsString,
    ini_data: Ini
}

pub struct Gearbox {
    gear_count: i32,
    reverse_gear_ratio: f64,
    final_gear_ratio: f64,
    gear_ratios: Vec<f64>,
    change_up_time: i32,
    change_dn_time: i32,
    auto_cutoff_time: i32,
    supports_shifter: i32,
    valid_shift_rpm_window: i32,
    controls_window_gain: f64,
    inertia: f64
}

fn mandatory_field_error(section: &str, key: &str) -> Error {
    return Error::new(
        ErrorKind::InvalidCar,
        format!("Missing {}.{} in {}", section, key, Drivetrain::INI_FILENAME)
    )
}

impl Gearbox {
    pub fn load_from_ini(ini_data: &Ini) -> Result<Gearbox> {
        let gear_count = match ini_utils::get_value(ini_data, "GEARS", "COUNT") {
            Some(val) => val,
            None => { return Err(mandatory_field_error("GEARS", "COUNT")); }
        };
        let mut gear_ratios = Vec::new();
        for gear_num in 1..gear_count+1 {
            let gear_key = format!("GEAR_{}", gear_num);
            gear_ratios.push(match ini_utils::get_value(ini_data, "GEARS", gear_key.as_str()) {
                Some(val) => val,
                None => { return Err(mandatory_field_error("GEARS", gear_key.as_str())); }
            });
        }
        let reverse_gear_ratio = match ini_utils::get_value(ini_data, "GEARS", "GEAR_R") {
            Some(val) => val,
            None => { return Err(mandatory_field_error("GEARS", "GEAR_R")); }
        };
        let final_gear_ratio = match ini_utils::get_value(ini_data, "GEARS", "FINAL") {
            Some(val) => val,
            None => { return Err(mandatory_field_error("GEARS", "FINAL")); }
        };
        let change_up_time = match ini_utils::get_value(ini_data, "GEARBOX", "CHANGE_UP_TIME") {
            Some(val) => val,
            None => { return Err(mandatory_field_error("GEARBOX", "CHANGE_UP_TIME")); }
        };
        let change_dn_time = match ini_utils::get_value(ini_data, "GEARBOX", "CHANGE_DN_TIME") {
            Some(val) => val,
            None => { return Err(mandatory_field_error("GEARBOX", "CHANGE_DN_TIME")); }
        };
        let auto_cutoff_time = match ini_utils::get_value(ini_data, "GEARBOX", "AUTO_CUTOFF_TIME") {
            Some(val) => val,
            None => { return Err(mandatory_field_error("GEARBOX", "AUTO_CUTOFF_TIME")); }
        };
        let supports_shifter = match ini_utils::get_value(ini_data, "GEARBOX", "SUPPORTS_SHIFTER") {
            Some(val) => val,
            None => { return Err(mandatory_field_error("GEARBOX", "SUPPORTS_SHIFTER")); }
        };
        let valid_shift_rpm_window = match ini_utils::get_value(ini_data, "GEARBOX", "VALID_SHIFT_RPM_WINDOW") {
            Some(val) => val,
            None => { return Err(mandatory_field_error("GEARBOX", "VALID_SHIFT_RPM_WINDOW")); }
        };
        let controls_window_gain = match ini_utils::get_value(ini_data, "GEARBOX", "CONTROLS_WINDOW_GAIN") {
            Some(val) => val,
            None => { return Err(mandatory_field_error("GEARBOX", "CONTROLS_WINDOW_GAIN")); }
        };
        let inertia = match ini_utils::get_value(ini_data, "GEARBOX", "INERTIA") {
            Some(val) => val,
            None => { 0.02 }
        };
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

impl Drivetrain {
    const INI_FILENAME: &'static str = "drivetrain.ini";

    pub fn load_from_path(data_dir: &Path) -> Result<Drivetrain> {
        let ini_data = match load_ini_file(data_dir.join(Drivetrain::INI_FILENAME).as_path()) {
            Ok(ini_object) => { ini_object }
            Err(err_str) => {
                return Err(Error::new(ErrorKind::InvalidCar, err_str ));
            }
        };
        Ok(Drivetrain {
            data_dir: OsString::from(data_dir),
            ini_data
        })
    }

    pub fn drive_type(&self) -> Option<DriveType> {
        ini_utils::get_value(&self.ini_data, "TRACTION", "TYPE")
    }

    pub fn gearbox(&self) -> Result<Gearbox> {
        Gearbox::load_from_ini(&self.ini_data)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use crate::assetto_corsa::drivetrain::Drivetrain;
    use crate::assetto_corsa::engine::Engine;

    #[test]
    fn load_drivetrain() -> Result<(), String> {
        let path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/a1_science_car/data");
        match Drivetrain::load_from_path(&path) {
            Ok(drivetrain) => {
                let gearbox = drivetrain.gearbox().unwrap();
                Ok(())
            }
            Err(e) => { Err(e.to_string()) }
        }
    }
}