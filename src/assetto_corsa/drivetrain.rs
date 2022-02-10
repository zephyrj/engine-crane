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

impl Gearbox {
    pub fn load_from_ini(ini_data: &Ini) -> Result<Gearbox> {
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

#[derive(Debug)]
pub struct Differential {
    power: f64,
    coast: f64,
    preload: i32
}

impl Differential {
    pub fn load_from_ini(ini_data: &Ini) -> Result<Differential> {
        let power = get_mandatory_field(ini_data, "DIFFERENTIAL", "POWER")?;
        let coast = get_mandatory_field(ini_data, "DIFFERENTIAL", "COAST")?;
        let preload = get_mandatory_field(ini_data, "DIFFERENTIAL", "PRELOAD")?;
        Ok(Differential{ power, coast, preload })
    }
}

#[derive(Debug)]
pub struct ShiftProfile {
    name: String,
    points: Vec<i32>
}

impl ShiftProfile {
    pub fn load_from_ini(ini_data: &Ini, name: &str) -> Result<ShiftProfile> {
        let name = String::from(name);
        let mut points = Vec::new();
        for idx in 0..3 {
            points.push(get_mandatory_field(ini_data, &name, &format!("POINT_{}", idx))?);
        }
        Ok(ShiftProfile { name, points })
    }
}

#[derive(Debug)]
pub struct AutoClutch {
    upshift_profile: Option<ShiftProfile>,
    downshift_profile: Option<ShiftProfile>,
    use_on_changes: i32,
    min_rpm: i32,
    max_rpm: i32,
    forced_on: i32
}

impl AutoClutch {
    pub fn load_from_ini(ini_data: &Ini) -> Result<AutoClutch> {
        let upshift_profile = AutoClutch::load_shift_profile(ini_data, "UPSHIFT_PROFILE")?;
        let downshift_profile = AutoClutch::load_shift_profile(ini_data, "DOWNSHIFT_PROFILE")?;
        let use_on_changes = get_mandatory_field(ini_data, "AUTOCLUTCH", "USE_ON_CHANGES")?;
        let min_rpm = get_mandatory_field(ini_data, "AUTOCLUTCH", "MIN_RPM")?;
        let max_rpm = get_mandatory_field(ini_data, "AUTOCLUTCH", "MAX_RPM")?;
        let forced_on = get_mandatory_field(ini_data, "AUTOCLUTCH", "FORCED_ON")?;

        Ok(AutoClutch {
            upshift_profile,
            downshift_profile,
            use_on_changes,
            min_rpm,
            max_rpm,
            forced_on
        })
    }

    fn load_shift_profile(ini_data: &Ini, key_name: &str) -> Result<Option<ShiftProfile>> {
        if let Some(profile_name) = ini_utils::get_value(ini_data, "AUTOCLUTCH", key_name) {
            let section_name: String = profile_name;
            if section_name.to_lowercase() != "none" {
                return match ShiftProfile::load_from_ini(ini_data, &section_name) {
                    Ok(prof) => { Ok(Some(prof)) },
                    Err(_) => { return Err(mandatory_field_error(key_name, &section_name)); }
                }
            }
        }
        Ok(None)
    }
}

#[derive(Debug)]
pub struct AutoBlip {
    electronic: i32,
    points: Vec<i32>,
    level: f64
}

impl AutoBlip {
    pub fn load_from_ini(ini_data: &Ini) -> Result<AutoBlip> {
        let electronic = get_mandatory_field(ini_data, "AUTOBLIP", "ELECTRONIC")?;
        let level = get_mandatory_field(ini_data, "AUTOBLIP", "LEVEL")?;
        let mut points = Vec::new();
        for idx in 0..3 {
            points.push(get_mandatory_field(ini_data, "AUTOBLIP", &format!("POINT_{}", idx))?);
        }
        Ok(AutoBlip{ electronic, points, level })
    }
}

#[derive(Debug)]
pub struct AutoShifter {
    up: i32,
    down: i32,
    slip_threshold: f64,
    gas_cutoff_time: f64
}

impl AutoShifter {
    pub fn load_from_ini(ini_data: &Ini) -> Result<AutoShifter> {
        let up = get_mandatory_field(ini_data, "AUTO_SHIFTER", "UP")?;
        let down = get_mandatory_field(ini_data, "AUTO_SHIFTER", "DOWN")?;
        let slip_threshold = get_mandatory_field(ini_data, "AUTO_SHIFTER", "SLIP_THRESHOLD")?;
        let gas_cutoff_time = get_mandatory_field(ini_data, "AUTO_SHIFTER", "GAS_CUTOFF_TIME")?;
        Ok(AutoShifter{ up, down, slip_threshold, gas_cutoff_time })
    }
}

#[derive(Debug)]
pub struct DownshiftProtection {
    active: i32,
    debug: i32,
    overrev: i32,
    lock_n: i32
}

impl DownshiftProtection {
    pub fn load_from_ini(ini_data: &Ini) -> Result<DownshiftProtection> {
        Ok(DownshiftProtection{
            active: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "ACTIVE")?,
            debug: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "DEBUG")?,
            overrev: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "OVERREV")?,
            lock_n: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "LOCK_N")?,
        })
    }
}


#[derive(Debug)]
pub struct Drivetrain {
    data_dir: OsString,
    ini_data: Ini
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

    pub fn drive_type(&self) -> Result<DriveType> {
        get_mandatory_field(&self.ini_data, "TRACTION", "TYPE")
    }

    pub fn gearbox(&self) -> Result<Gearbox> {
        Gearbox::load_from_ini(&self.ini_data)
    }

    pub fn differential(&self) -> Result<Differential> {
        Differential::load_from_ini(&self.ini_data)
    }

    pub fn auto_clutch(&self) -> Result<AutoClutch> {
        AutoClutch::load_from_ini(&self.ini_data)
    }

    pub fn auto_blip(&self) -> Result<AutoBlip> {
        AutoBlip::load_from_ini(&self.ini_data)
    }

    pub fn auto_shifter(&self) -> Result<AutoShifter> {
        AutoShifter::load_from_ini(&self.ini_data)
    }

    pub fn downshift_protection(&self) -> Result<DownshiftProtection> {
        DownshiftProtection::load_from_ini(&self.ini_data)
    }
}

fn get_mandatory_field<T: std::str::FromStr>(ini_data: &Ini, section_name: &str, key: &str) -> Result<T> {
    let res: T = match ini_utils::get_value(ini_data, section_name, key) {
        Some(val) => val,
        None => { return Err(mandatory_field_error(section_name, key)); }
    };
    Ok(res)
}

fn mandatory_field_error(section: &str, key: &str) -> Error {
    return Error::new(
        ErrorKind::InvalidCar,
        format!("Missing {}.{} in {}", section, key, Drivetrain::INI_FILENAME)
    )
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
                let differential = drivetrain.differential().unwrap();
                let auto_clutch = drivetrain.auto_clutch().unwrap();
                let auto_blip = drivetrain.auto_blip().unwrap();
                let auto_shifter = drivetrain.auto_shifter().unwrap();
                let downshift_protection = drivetrain.downshift_protection().unwrap();
                Ok(())
            }
            Err(e) => { Err(e.to_string()) }
        }
    }
}