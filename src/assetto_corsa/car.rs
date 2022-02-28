use std::collections::HashMap;
use std::fs;
use std::default::Default;

use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufReader, BufRead, LineWriter, Write, BufWriter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use serde_json::Value;
use walkdir::WalkDir;
use crate::assetto_corsa;
use crate::assetto_corsa::traits::MandatoryCarData;
use crate::assetto_corsa::drivetrain::Drivetrain;
use crate::assetto_corsa::error::{Result, Error, ErrorKind, FieldParseError};
use crate::assetto_corsa::engine::{Engine};
use crate::assetto_corsa::{ini_utils, load_sfx_data};
use crate::assetto_corsa::ini_utils::Ini;

pub fn delete_data_acd_file(car_path: &Path) -> Result<()> {
    let acd_path = car_path.join("data.acd");
    if acd_path.exists() {
        std::fs::remove_file(acd_path).map_err(|io_err| {
            Error::from_io_error(io_err, format!("Failed to delete data.acd file from {}", car_path.display()).as_str())
        })?;
    }
    Ok(())
}

fn fix_car_specific_filenames(car_path: &Path, name_to_change: &str) -> Result<()> {
    let new_car_name = car_path.file_name().unwrap().to_str().unwrap();
    let mut paths_to_update: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&car_path).into_iter().filter_map(|e| e.ok()) {
        if !entry.metadata().unwrap().is_file() {
            continue
        }
        let filename = entry.file_name();
        let filename_string = filename.to_string_lossy();
        if (filename_string.starts_with(name_to_change)) &&
            (filename_string.ends_with(".kn5") || (filename_string.ends_with(".bank"))) {
            paths_to_update.push(entry.path().to_path_buf());
        } else if filename_string == "lods.ini" {
            let mut lod_ini = Ini::load_from_file(entry.path()).map_err(|err| {
                Error::from_io_error(err, "Failed to load lods.ini")
            })?;
            let mut idx = 0;
            loop {
                let current_lod_name = format!("LOD_{}", idx);
                if !lod_ini.section_contains_property(&current_lod_name, "FILE") {
                    break
                }
                let old_value: String = ini_utils::get_value(&lod_ini, &current_lod_name, "FILE").unwrap();
                ini_utils::set_value(&mut lod_ini,
                                     &current_lod_name,
                                     "FILE",
                                     old_value.replace(name_to_change, new_car_name));
                idx += 1;
            }
            lod_ini.write(entry.path()).map_err(|err| {
                Error::from_io_error(err, "Failed to write lods.ini")
            })?;
        }
    }

    for path in paths_to_update {
        let mut new_path = path.clone();
        let new_filename = path.file_name().unwrap().to_str().unwrap().replace(name_to_change, new_car_name);
        new_path.pop();
        new_path.push(new_filename);
        std::fs::rename(&path, &new_path).map_err(|err| {
            Error::new(ErrorKind::Uncategorized,
                       format!("Failed to rename from {} to {}. {}", path.display(), new_path.display(), err.to_string()))
        })?;
    }
    Ok(())
}

pub fn update_car_name(car_path: &Path, new_name: &str) -> Result<()> {
    let mut new_car = Car::load_from_path(car_path)?;
    new_car.set_screen_name(new_name);
    new_car.ui_info.set_name(new_name.to_string());
    new_car.write()?;
    Ok(())
}

fn update_car_sfx(car_path: &Path, name_to_change: &str) -> Result<()> {
    let guids_file_path = car_path.join(PathBuf::from_iter(["sfx", "GUIDs.txt"]));
    let car_name = car_path.file_name().unwrap().to_str().unwrap();

    let mut updated_lines: Vec<String> = Vec::new();
    if guids_file_path.exists() {
        let file = File::open(&guids_file_path).map_err(|err|{
            Error::new(ErrorKind::Uncategorized,
                       String::from(
                           format!("Couldn't open {}. {}", guids_file_path.display(), err.to_string())))
        })?;

        updated_lines = BufReader::new(file).lines().into_iter().filter_map(|res| {
            match res {
                Ok(string) => Some(string.replace(name_to_change, car_name)),
                Err(err) => {
                    println!("Warning: Encountered error reading from {}. {}",
                             guids_file_path.display(),
                             err.to_string());
                    None
                }
            }
        }).collect();

    } else {
        updated_lines = load_sfx_data()?.generate_clone_guid_info(name_to_change, car_name);
    }

    let file = File::create(&guids_file_path).map_err(|err|{
        Error::new(ErrorKind::Uncategorized,
                   String::from(
                       format!("Couldn't re-create {}. {}", guids_file_path.display(), err.to_string())))
    })?;
    let mut file = LineWriter::new(file);
    for line in updated_lines {
        write!(file, "{}\n", line).map_err(|err|{
            Error::new(ErrorKind::Uncategorized,
                       String::from(
                           format!("Couldn't write to {}. {}", guids_file_path.display(), err.to_string())))
        })?;
    }
    Ok(())
}

pub fn clone_existing_car(existing_car_path: &Path, new_car_path: &Path) -> Result<()> {
    if existing_car_path == new_car_path {
        return Err(Error::new(ErrorKind::CarAlreadyExists,
                              format!("Cannot clone car to its existing location. ({})",
                                             existing_car_path.display())));
    }

    std::fs::create_dir(&new_car_path).map_err(|err| {
        Error::new(ErrorKind::Uncategorized,
                   format!("Failed to create {}. {}", new_car_path.display(), err.to_string()))
    })?;
    let mut copy_options = fs_extra::dir::CopyOptions::new();
    copy_options.content_only = true;
    fs_extra::dir::copy(&existing_car_path,
                        &new_car_path,
                        &copy_options).map_err(|err|{
        Error::new(ErrorKind::Uncategorized,
                   format!("Failed to copy contents of {} to {}. {}",
                           existing_car_path.display(),
                           new_car_path.display(),
                           err.to_string()))
    })?;

    if let Some(err) = delete_data_acd_file(new_car_path).err(){
        println!("Warning: {}", err.to_string());
    }
    let existing_car_name = existing_car_path.file_name().unwrap().to_str().unwrap();
    let cloned_car_name = new_car_path.file_name().unwrap().to_str().unwrap();
    fix_car_specific_filenames(new_car_path, existing_car_name)?;
    update_car_name(new_car_path, cloned_car_name)?;
    update_car_sfx(new_car_path, existing_car_name)?;
    Ok(())
}

pub fn create_new_car_spec(existing_car_name: &str, spec_name: &str) -> Result<PathBuf>{
    let installed_cars_path = match assetto_corsa::get_installed_cars_path() {
        Some(path) => { path },
        None => {
            return Err(Error::new(ErrorKind::NoSuchCar,
                                  String::from("Can't find installed cars path")));
        }
    };

    let existing_car_path = installed_cars_path.join(existing_car_name);
    if !existing_car_path.exists() {
        return Err(Error::new(ErrorKind::NoSuchCar,
                              format!("Can't find {}", existing_car_path.display())));
    }
    let new_car_name = format!("{}_{}", existing_car_name, spec_name);
    let new_car_path = installed_cars_path.join(&new_car_name);
    if new_car_path.exists() {
        return Err(Error::new(ErrorKind::CarAlreadyExists,
                              format!("{}", new_car_path.display())));
    }

    clone_existing_car(existing_car_path.as_path(), new_car_path.as_path())?;
    Ok(new_car_path)
}

#[derive(Debug)]
pub enum CarVersion {
    One,
    Two,
    CspExtendedPhysics
}

impl Default for CarVersion {
    fn default() -> Self {
        CarVersion::One
    }
}

impl CarVersion {
    pub const VERSION_1 :&'static str = "1";
    pub const VERSION_2 :&'static str = "2";
    pub const CSP_EXTENDED_2 : &'static str = "extended-2";

    fn as_str(&self) -> &'static str {
        match self {
            CarVersion::One => CarVersion::VERSION_1,
            CarVersion::Two => CarVersion::VERSION_2,
            CarVersion::CspExtendedPhysics => CarVersion::CSP_EXTENDED_2
        }
    }
}

impl FromStr for CarVersion {
    type Err = FieldParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            CarVersion::VERSION_1 => Ok(CarVersion::One),
            CarVersion::VERSION_2 => Ok(CarVersion::Two),
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
    ui_info_path: PathBuf,
    json_config: serde_json::Value
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
        let json_config = match serde_json::from_str(ui_info_string.replace("\r\n", "\n").replace("\n", " ").replace("\t", "  ").as_str()) {
            Ok(decoded_json) => { decoded_json },
            Err(e) => {
                return Err( Error::new(ErrorKind::InvalidCar,
                                       String::from(format!("Failed to decode {}: {}",
                                                            ui_json_path.display(),
                                                            e.to_string()))) )
            }
        };
        let ui_info = UiInfo {
            ui_info_path: ui_json_path.to_path_buf(),
            json_config
        };
        Ok(ui_info)
    }

    pub fn write(&self) -> Result<()> {
        let writer = BufWriter::new(File::create(&self.ui_info_path).map_err(|err| {
            Error::from_io_error(err,
                                 format!("Failed to create {}", &self.ui_info_path.display()).as_str())
        })?);
        serde_json::to_writer_pretty(writer, &self.json_config).map_err(|err| {
            Error::new(ErrorKind::IOError,
                       format!("Failed to write {}. {}",
                               &self.ui_info_path.display(),
                               err.to_string()))
        })?;
        Ok(())
    }

    pub fn name(&self) -> Option<&str> {
        self.get_json_string("name")
    }

    pub fn set_name(&mut self, name: String) {
        self.set_json_string("name", name);
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

    fn set_json_string(&mut self, key: &str, value: String) {
        match self.json_config.get_mut(key) {
            None => {}
            Some(val) => {
                match val {
                    Value::String(str) => {
                        std::mem::replace(str, value); }
                    _ => {}
                }
            }
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
    root_path: PathBuf,
    ini_config: Ini,
    pub ui_info: UiInfo,
    engine: Engine,
    drivetrain: Drivetrain
}

impl Car {
    pub fn version(&self) -> Option<CarVersion> {
        ini_utils::get_value(&self.ini_config, "HEADER", "VERSION")
    }

    pub fn screen_name(&self) -> Option<String> {
        ini_utils::get_value(&self.ini_config, "INFO","SCREEN_NAME")
    }

    pub fn set_screen_name(&mut self, name: &str) {
        ini_utils::set_value(&mut self.ini_config, "INFO","SCREEN_NAME", name);
    }

    pub fn total_mass(&self) -> Option<u32> {
        ini_utils::get_value(&self.ini_config, "BASIC","TOTALMASS")
    }

    pub fn default_fuel(&self) -> Option<u32> {
        ini_utils::get_value(&self.ini_config, "FUEL","FUEL")
    }

    pub fn max_fuel(&self) -> Option<u32> {
        ini_utils::get_value(&self.ini_config, "FUEL","MAX_FUEL")
    }

    pub fn fuel_consumption(&self) -> Option<f64> {
        ini_utils::get_value(&self.ini_config, "FUEL","CONSUMPTION")
    }

    pub fn write(&self) -> Result<()> {
        let out_path = self.root_path.join(["data", "car.ini"].iter().collect::<PathBuf>());
        self.ini_config.write(&out_path).map_err(|err| {
            Error::from_io_error(err,
                                 format!("Failed to parse {}", out_path.display()).as_str())
        })?;
        self.ui_info.write()
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
        let car = Car {
            root_path: car_folder_path.to_path_buf(),
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
    use crate::assetto_corsa::car::{Car, create_new_car_spec};

    #[test]
    fn load_car() -> Result<(), String> {
        let this_file = Path::new(file!());
        let this_dir = this_file.parent().unwrap();
        let path = this_dir.join("test-data/car-with-turbo-with-ctrls");
        let car = match Car::load_from_path(&path) {
            Ok(car) => {
                car
            }
            Err(e) => {  return Err(e.to_string()) }
        };
        let ui_info = &car.ui_info;
        assert_eq!(ui_info.name().unwrap(), "Turbo with CTRL");
        assert_eq!(ui_info.brand().unwrap(), "Test");
        assert_eq!(ui_info.class().unwrap(), "street");
        assert_eq!(ui_info.tags().unwrap(), Vec::from(["#Supercars", "awd", "semiautomatic", "street", "turbo", "germany"]));
        let specs = ui_info.specs().unwrap();
        Ok(())
    }

    #[test]
    fn clone_car() {
        let new_car_path = create_new_car_spec("zephyr_za401", "test").unwrap();
        println!("{}", new_car_path.display());
    }
}
