use std::ffi::OsString;
use std::path::Path;
use std::str::FromStr;
use crate::assetto_corsa::error::{Result, Error, ErrorKind, FieldParseError};
use crate::assetto_corsa::file_utils::load_ini_file;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::Ini;

#[derive(Debug, Eq, PartialEq)]
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
    pub reverse_gear_ratio: f64,
    pub final_gear_ratio: f64,
    gear_ratios: Vec<f64>,
    pub change_up_time: i32,
    pub change_dn_time: i32,
    pub auto_cutoff_time: i32,
    pub supports_shifter: i32,
    pub valid_shift_rpm_window: i32,
    pub controls_window_gain: f64,
    pub inertia: f64
}

impl Gearbox {
    pub fn update_gears(&mut self, gear_ratios: Vec<f64>, final_drive_ratio: f64) -> Option<(Vec<f64>, f64)> {
        let old_vec = std::mem::replace(&mut self.gear_ratios, gear_ratios);
        let old_final_drive = std::mem::replace(&mut self.final_gear_ratio, final_drive_ratio);
        return match old_vec.len() {
            0 => None,
            _ => Some((old_vec, old_final_drive))
        }
    }

    fn create_gear_key(gear_num: i32) -> String {
        format!("GEAR_{}", gear_num)
    }
}

impl FromIni for Gearbox {
    fn load_from_ini(ini_data: &Ini) -> Result<Gearbox> {
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

impl IniUpdater for Gearbox {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        let current_count_opt: Option<i32> = ini_utils::get_value(ini_data, "GEARS", "COUNT");
        if let Some(current_count) = current_count_opt {
            if current_count != self.gear_count {
                for gear_num in 1..current_count+1 {
                    ini_data.remove_value("GEARS", Gearbox::create_gear_key(gear_num).as_str());
                }
            }
        }
        ini_data.set_value("GEARS", "COUNT", self.gear_count.to_string());
        for gear_num in 1..self.gear_count+1 {
            if let Some(gear_ratio) = self.gear_ratios.get((gear_num-1) as usize) {
                ini_utils::set_float(ini_data,
                                     "GEARS",
                                     Gearbox::create_gear_key(gear_num).as_str(),
                                     *gear_ratio,
                                     3);
            } else {
                return Err(String::from("Warning: gear count doesn't match stored ratios"));
            }
        }
        ini_utils::set_float(ini_data, "GEARS", "GEAR_R", self.reverse_gear_ratio, 3);
        ini_utils::set_float(ini_data, "GEARS", "FINAL", self.final_gear_ratio, 3);
        ini_utils::set_value(ini_data, "GEARBOX", "CHANGE_UP_TIME", self.change_up_time);
        ini_utils::set_value(ini_data, "GEARBOX", "CHANGE_DN_TIME", self.change_dn_time);
        ini_utils::set_value(ini_data, "GEARBOX", "AUTO_CUTOFF_TIME", self.auto_cutoff_time);
        ini_utils::set_value(ini_data, "GEARBOX", "SUPPORTS_SHIFTER", self.supports_shifter);
        ini_utils::set_value(ini_data, "GEARBOX", "VALID_SHIFT_RPM_WINDOW", self.valid_shift_rpm_window);
        ini_utils::set_float(ini_data, "GEARBOX", "CONTROLS_WINDOW_GAIN", self.controls_window_gain, 2);
        ini_utils::set_float(ini_data, "GEARBOX", "INERTIA", self.inertia, 3);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Differential {
    pub power: f64,
    pub coast: f64,
    pub preload: i32
}

impl FromIni for Differential {
    fn load_from_ini(ini_data: &Ini) -> Result<Differential> {
        let power = get_mandatory_field(ini_data, "DIFFERENTIAL", "POWER")?;
        let coast = get_mandatory_field(ini_data, "DIFFERENTIAL", "COAST")?;
        let preload = get_mandatory_field(ini_data, "DIFFERENTIAL", "PRELOAD")?;
        Ok(Differential { power, coast, preload })
    }
}

impl IniUpdater for Differential {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_float(ini_data, "DIFFERENTIAL", "POWER", self.power, 2);
        ini_utils::set_float(ini_data, "DIFFERENTIAL", "COAST", self.coast, 2);
        ini_utils::set_value(ini_data, "DIFFERENTIAL", "PRELOAD", self.preload);
        Ok(())
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
    pub use_on_changes: i32,
    pub min_rpm: i32,
    pub max_rpm: i32,
    pub forced_on: i32
}

impl AutoClutch {
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

impl FromIni for AutoClutch {
    fn load_from_ini(ini_data: &Ini) -> Result<AutoClutch> {
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
}

impl IniUpdater for AutoClutch {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        // TODO Update shift profiles
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "USE_ON_CHANGES", self.use_on_changes);
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "MIN_RPM", self.min_rpm);
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "MAX_RPM", self.max_rpm);
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "FORCED_ON", self.forced_on);
        Ok(())
    }
}

#[derive(Debug)]
pub struct AutoBlip {
    pub electronic: i32,
    pub points: Vec<i32>,
    pub level: f64
}

impl AutoBlip {
    fn get_point_key<T: std::fmt::Display>(idx: T) -> String {
        format!("POINT_{}", idx)
    }
}

impl FromIni for AutoBlip {
    fn load_from_ini(ini_data: &Ini) -> Result<AutoBlip> {
        let electronic = get_mandatory_field(ini_data, "AUTOBLIP", "ELECTRONIC")?;
        let level = get_mandatory_field(ini_data, "AUTOBLIP", "LEVEL")?;
        let mut points = Vec::new();
        for idx in 0..3 {
            points.push(get_mandatory_field(ini_data,
                                            "AUTOBLIP",
                                            AutoBlip::get_point_key(idx).as_str())?);
        }
        Ok(AutoBlip{ electronic, points, level })
    }
}

impl IniUpdater for AutoBlip {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_value(ini_data, "AUTOBLIP", "ELECTRONIC", self.electronic);
        ini_utils::set_float(ini_data, "AUTOBLIP", "LEVEL", self.level, 2);
        for (idx, point) in self.points.iter().enumerate() {
            ini_utils::set_value(ini_data,
                                 "AUTOBLIP",
                                 AutoBlip::get_point_key(idx).as_str(),
                                 point);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct AutoShifter {
    pub up: i32,
    pub down: i32,
    pub slip_threshold: f64,
    pub gas_cutoff_time: f64
}

impl FromIni for AutoShifter {
    fn load_from_ini(ini_data: &Ini) -> Result<AutoShifter> {
        let up = get_mandatory_field(ini_data, "AUTO_SHIFTER", "UP")?;
        let down = get_mandatory_field(ini_data, "AUTO_SHIFTER", "DOWN")?;
        let slip_threshold = get_mandatory_field(ini_data, "AUTO_SHIFTER", "SLIP_THRESHOLD")?;
        let gas_cutoff_time = get_mandatory_field(ini_data, "AUTO_SHIFTER", "GAS_CUTOFF_TIME")?;
        Ok(AutoShifter{ up, down, slip_threshold, gas_cutoff_time })
    }
}

impl IniUpdater for AutoShifter {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_value(ini_data, "AUTO_SHIFTER", "UP", self.up);
        ini_utils::set_value(ini_data, "AUTO_SHIFTER", "DOWN", self.down);
        ini_utils::set_float(ini_data, "AUTO_SHIFTER", "SLIP_THRESHOLD", self.slip_threshold, 2);
        ini_utils::set_float(ini_data, "AUTO_SHIFTER", "GAS_CUTOFF_TIME", self.gas_cutoff_time, 2);
        Ok(())
    }
}

#[derive(Debug)]
pub struct DownshiftProtection {
    pub active: i32,
    pub debug: i32,
    pub overrev: i32,
    pub lock_n: i32
}

impl FromIni for DownshiftProtection {
    fn load_from_ini(ini_data: &Ini) -> Result<DownshiftProtection> {
        Ok(DownshiftProtection{
            active: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "ACTIVE")?,
            debug: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "DEBUG")?,
            overrev: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "OVERREV")?,
            lock_n: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "LOCK_N")?,
        })
    }
}

impl IniUpdater for DownshiftProtection {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "ACTIVE", self.active);
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "DEBUG", self.debug);
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "OVERREV", self.overrev);
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "LOCK_N", self.lock_n);
        Ok(())
    }
}

pub trait IniUpdater {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String>;
}

pub trait FromIni {
    fn load_from_ini(ini_data: &Ini) -> Result<Self> where Self: Sized;
}


#[derive(Debug)]
pub struct Drivetrain {
    data_dir: OsString,
    ini_data: Ini
}

impl Drivetrain {
    const INI_FILENAME: &'static str = "drivetrain.ini";

    pub fn load_from_file(ini_path: &Path) -> Result<Drivetrain> {
        let ini_data = match load_ini_file(ini_path) {
            Ok(ini_object) => { ini_object }
            Err(err) => {
                return Err(Error::new(ErrorKind::InvalidCar, err.to_string() ));
            }
        };
        Ok(Drivetrain {
            data_dir: OsString::from(ini_path.parent().unwrap()),
            ini_data
        })
    }

    pub fn load_from_path(data_dir: &Path) -> Result<Drivetrain> {
        Drivetrain::load_from_file(data_dir.join(Drivetrain::INI_FILENAME).as_path())
    }

    pub fn write(&mut self) -> std::io::Result<()> {
        self.ini_data.write(&Path::new(&self.data_dir).join(Drivetrain::INI_FILENAME))
    }

    pub fn drive_type(&self) -> Result<DriveType> {
        get_mandatory_field(&self.ini_data, "TRACTION", "TYPE")
    }

    pub fn set_drive_type(&mut self, drive_type: DriveType) -> Result<()> {
        let _ = self.ini_data.set_value("TRACTION",
                                        "TYPE",
                                        drive_type.to_string());
        Ok(())
    }

    pub fn load_component<T: FromIni>(&self) -> Result<T> {
        T::load_from_ini(&self.ini_data)
    }

    pub fn update_component<T: IniUpdater>(&mut self, component: &T) -> Result<()> {
        component.update_ini(&mut self.ini_data).map_err(|err_string| {
            Error::new(ErrorKind::InvalidUpdate, err_string)
        })
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
    use std::fs;
    use std::path::Path;
    use crate::assetto_corsa::drivetrain::{AutoBlip, AutoClutch, AutoShifter, Differential, DownshiftProtection, Drivetrain, DriveType, FromIni, Gearbox, IniUpdater};

    const TEST_INPUT_FILENAME: &'static str = "/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/zephyr_za401/data";
    const TEST_OUTPUT_FILENAME: &'static str = "test.ini";

    #[test]
    fn update_drive_type() -> Result<(), String> {
        let new_drive_type = DriveType::FWD;

        let _exit = TidyTestFiles;
        let mut drivetrain = Drivetrain::load_from_path(&Path::new(TEST_INPUT_FILENAME)).map_err(|err| format!("{}", err.to_string()))?;
        match drivetrain.set_drive_type(DriveType::FWD) {
            Ok(_) => {}
            Err(e) => { return Err(e.to_string()); }
        };
        drivetrain.ini_data.write(Path::new(TEST_OUTPUT_FILENAME)).map_err(|err| format!("{}", err.to_string()))?;
        let new_drivetrain = Drivetrain::load_from_file(&Path::new(TEST_OUTPUT_FILENAME)).map_err(|err| format!("{}", err.to_string()))?;
        assert_eq!(new_drivetrain.drive_type().unwrap(), new_drive_type, "Drive type is correct");
        Ok(())
    }

    #[test]
    fn update_gearbox() -> Result<(), String> {
        let new_change_up_time = 140;
        let new_change_dn_time = 190;
        let new_auto_cutoof_time = 160;
        let new_supports_shifter = 1;
        let new_valid_shift_window = 700;
        let new_controls_window_gain = 0.3;
        let new_inertia = 0.02;
        let new_ratios: Vec<f64> = vec!(2.40, 1.9, 1.61, 1.33, 1.12, 0.99);
        let new_final_drive = 3.2;
        let new_reverse_drive = -3.700;

        let _exit = TidyTestFiles;
        component_update_test(|gearbox: &mut Gearbox| {
            gearbox.change_up_time = new_change_up_time;
            gearbox.change_dn_time = new_change_dn_time;
            gearbox.auto_cutoff_time = new_auto_cutoof_time;
            gearbox.supports_shifter = new_supports_shifter;
            gearbox.valid_shift_rpm_window = new_valid_shift_window;
            gearbox.controls_window_gain = new_controls_window_gain;
            gearbox.inertia = new_inertia;
            gearbox.reverse_gear_ratio = new_reverse_drive;
            gearbox.update_gears(new_ratios.clone(), new_final_drive);
        })?;
        validate_component(|gearbox: &Gearbox| {
            assert_eq!(gearbox.change_up_time, new_change_up_time, "Change up time is correct");
            assert_eq!(gearbox.change_dn_time, new_change_dn_time, "Change dn time is correct");
            assert_eq!(gearbox.auto_cutoff_time, new_auto_cutoof_time, "Auto cutoff time is correct");
            assert_eq!(gearbox.supports_shifter, new_supports_shifter, "Supports shifter is correct");
            assert_eq!(gearbox.valid_shift_rpm_window, new_valid_shift_window, "Valid shift rpm window is correct");
            assert_eq!(gearbox.controls_window_gain, new_controls_window_gain, "Controls window gain is correct");
            assert_eq!(gearbox.reverse_gear_ratio, new_reverse_drive, "Reverse gear ratio is correct");
            assert_eq!(gearbox.inertia, new_inertia, "Inertia is correct");
            assert_eq!(gearbox.gear_ratios, new_ratios, "New ratios are correct");
            assert_eq!(gearbox.final_gear_ratio, new_final_drive, "Final drive correct");
            assert_eq!(gearbox.gear_count, new_ratios.len() as i32, "Gear count is correct");
        })
    }

    #[test]
    fn update_differential() -> Result<(), String> {
        let new_preload = 15;
        let new_power = 0.2;
        let new_coast = 0.8;

        let _exit = TidyTestFiles;
        component_update_test(|differential: &mut Differential|{
            differential.preload = new_preload;
            differential.power = new_power;
            differential.coast = new_coast;
        })?;
        validate_component(|differential: &Differential| {
            assert_eq!(differential.preload, new_preload, "Preload is correct");
            assert_eq!(differential.power, new_power, "Power is correct");
            assert_eq!(differential.coast, new_coast, "Coast is correct");
        })
    }

    #[test]
    fn update_auto_clutch() -> Result<(), String> {
        let new_min_rpm = 2250;
        let new_max_rpm = 3250;
        let new_use_on_changes = 0;
        let new_forced_on = 1;

        let _exit = TidyTestFiles;
        component_update_test(|auto_clutch: &mut AutoClutch|{
            auto_clutch.min_rpm = new_min_rpm;
            auto_clutch.max_rpm = new_max_rpm;
            auto_clutch.use_on_changes = new_use_on_changes;
            auto_clutch.forced_on = new_forced_on;
        })?;
        validate_component(|auto_clutch: &AutoClutch| {
            assert_eq!(auto_clutch.min_rpm, new_min_rpm, "MinRpm is correct");
            assert_eq!(auto_clutch.max_rpm, new_max_rpm, "MaxRpm is correct");
            assert_eq!(auto_clutch.use_on_changes, new_use_on_changes, "new_use_on_changes is correct");
            assert_eq!(auto_clutch.forced_on, new_forced_on, "new_forced_on is correct");
        })
    }

    #[test]
    fn update_auto_blip() -> Result<(), String> {
        let new_level = 0.8;
        let new_points = vec![20, 200, 260];
        let new_electronic = 0;

        let _exit = TidyTestFiles;
        component_update_test(|auto_blip: &mut AutoBlip|{
            auto_blip.level = new_level;
            auto_blip.points = new_points.clone();
            auto_blip.electronic = new_electronic;
        })?;
        validate_component(|auto_blip: &AutoBlip| {
            assert_eq!(auto_blip.level, new_level, "Level is correct");
            assert_eq!(auto_blip.points, new_points, "Points are correct");
            assert_eq!(auto_blip.electronic, new_electronic, "Electronic is correct");
        })
    }

    #[test]
    fn update_auto_shifter() -> Result<(), String> {
        let new_up = 6000;
        let new_down = 4400;
        let new_slip_threshold = 1.0;
        let new_gas_cutoff_time = 0.3;

        let _exit = TidyTestFiles;
        component_update_test(|auto_shifter: &mut AutoShifter|{
            auto_shifter.up = new_up;
            auto_shifter.down = new_down;
            auto_shifter.slip_threshold = new_slip_threshold;
            auto_shifter.gas_cutoff_time = new_gas_cutoff_time;
        })?;
        validate_component(|auto_shifter: &AutoShifter| {
            assert_eq!(auto_shifter.up, new_up, "Up is correct");
            assert_eq!(auto_shifter.down, new_down, "Down are correct");
            assert_eq!(auto_shifter.slip_threshold, new_slip_threshold, "Slip threshold is correct");
            assert_eq!(auto_shifter.gas_cutoff_time, new_gas_cutoff_time, "Gas cutoff time is correct");
        })
    }

    #[test]
    fn update_downshift_protection() -> Result<(), String> {
        let new_active = 0;
        let new_debug = 1;
        let new_overrev = 300;
        let new_lock_n = 0;

        let _exit = TidyTestFiles;
        component_update_test(|downshift_protection: &mut DownshiftProtection| {
            downshift_protection.active = new_active;
            downshift_protection.debug = new_debug;
            downshift_protection.overrev = new_overrev;
            downshift_protection.lock_n = new_lock_n;
        })?;
        validate_component(|downshift_protection: &DownshiftProtection| {
            assert_eq!(downshift_protection.active, new_active, "Active is correct");
            assert_eq!(downshift_protection.debug, new_debug, "Debug is correct");
            assert_eq!(downshift_protection.overrev, new_overrev, "Overrev is correct");
            assert_eq!(downshift_protection.lock_n, new_lock_n, "Lock N is correct");
        })
    }

    fn component_update_test<T: IniUpdater + FromIni, F: FnOnce(&mut T)>(component_update_fn: F) -> Result<(), String> {
        let load_path = Path::new(TEST_INPUT_FILENAME);
        let mut drivetrain = Drivetrain::load_from_path(&load_path).map_err(|err| format!("{}", err.to_string()))?;
        let mut component = drivetrain.load_component::<T>().unwrap();
        component_update_fn(&mut component);
        drivetrain.update_component(&component).map_err(|err| format!("{}", err.to_string()))?;
        drivetrain.ini_data.write(Path::new(TEST_OUTPUT_FILENAME)).map_err(|err| format!("{}", err.to_string()))?;
        Ok(())
    }

    fn validate_component<T, F>(component_validation_fn: F) -> Result<(), String>
    where T: FromIni,
          F: FnOnce(&T)
    {
        let drivetrain = Drivetrain::load_from_file(Path::new(TEST_OUTPUT_FILENAME)).map_err(|err| format!("{}", err.to_string()))?;
        let component = drivetrain.load_component::<T>().map_err(|err| format!("{}", err.to_string()))?;
        component_validation_fn(&component);
        Ok(())
    }

    struct TidyTestFiles;
    impl Drop for TidyTestFiles {
        fn drop(&mut self) {
            if Path::new(TEST_OUTPUT_FILENAME).exists() {
                fs::remove_file(TEST_OUTPUT_FILENAME).expect("Failed to clear up test files");
            }
        }
    }
}