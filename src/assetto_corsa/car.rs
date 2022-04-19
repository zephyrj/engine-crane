use std::collections::HashMap;
use std::fs;
use std::default::Default;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufReader, BufRead, LineWriter, Write, BufWriter};
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use serde_json::{json, Value};
use tracing::{warn, info};
use walkdir::WalkDir;

use crate::assetto_corsa;
use crate::assetto_corsa::traits:: DataInterface;
use crate::assetto_corsa::error::{Result, Error, PropertyParseError, ErrorKind};
use crate::assetto_corsa::{ini_utils, load_sfx_data};
use crate::assetto_corsa::ini_utils::Ini;
use crate::assetto_corsa::acd_utils::AcdArchive;
use crate::assetto_corsa::data::DataFolderInterface;


pub fn clone_existing_car(existing_car_path: &Path, new_car_path: &Path) -> Result<()> {
    if existing_car_path == new_car_path {
        return Err(Error::new(ErrorKind::CarAlreadyExists,
                              format!("Cannot clone car to its existing location. ({})", existing_car_path.display())));
    }

    std::fs::create_dir(&new_car_path)?;
    let mut copy_options = fs_extra::dir::CopyOptions::new();
    copy_options.content_only = true;
    fs_extra::dir::copy(&existing_car_path,
                        &new_car_path,
                        &copy_options)?;

    let existing_car_name = existing_car_path.file_name().unwrap().to_str().unwrap();
    let data_path = new_car_path.join("data");
    let acd_path = new_car_path.join("data.acd");
    if !data_path.exists() {
        if !acd_path.exists() {
            return Err(Error::new(ErrorKind::InvalidCar,
                                  format!("{} doesn't contain a data dir or data.acd file", existing_car_path.display())));
        }
        info!("No data dir present in {}. Data will be extracted from data.acd", new_car_path.display());
        AcdArchive::load_from_path_with_key(acd_path.as_path(), existing_car_name)?.unpack()?;
    }
    info!("Deleting {} as data will be invalid after clone completion", acd_path.display());
    if let Some(err) = delete_data_acd_file(new_car_path).err(){
        warn!("Warning: {}", err.to_string());
    }
    fix_car_specific_filenames(new_car_path, existing_car_name)?;
    update_car_sfx(new_car_path, existing_car_name)?;
    Ok(())
}

pub fn create_new_car_spec(existing_car_name: &str, spec_name: &str) -> Result<PathBuf>{
    let installed_cars_path = match assetto_corsa::get_installed_cars_path() {
        Some(path) => { path },
        None => {
            return Err(Error::new(ErrorKind::NoSuchCar, existing_car_name.to_owned()));
        }
    };
    let existing_car_path = installed_cars_path.join(existing_car_name);
    if !existing_car_path.exists() {
        return Err(Error::new(ErrorKind::NoSuchCar, existing_car_name.to_owned()));
    }
    let new_car_name = format!(
        "{}_{}",
        existing_car_name,
        spec_name.to_lowercase().split_whitespace().collect::<Vec<&str>>().join("_")
    );
    let new_car_path = installed_cars_path.join(&new_car_name);
    if new_car_path.exists() {
        return Err(Error::new(ErrorKind::CarAlreadyExists, new_car_name));
    }
    info!("Cloning {} to {}", existing_car_path.display(), new_car_path.display());
    clone_existing_car(existing_car_path.as_path(), new_car_path.as_path())?;
    update_car_ui_data(new_car_path.as_path(), spec_name, existing_car_name)?;
    Ok(new_car_path)
}
pub fn delete_data_acd_file(car_path: &Path) -> Result<()> {
    let acd_path = car_path.join("data.acd");
    if acd_path.exists() {
        std::fs::remove_file(acd_path)?;
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
            let mut lod_ini = Ini::load_from_file(entry.path())?;
            let mut idx = 0;
            loop {
                let current_lod_name = format!("LOD_{}", idx);
                if !lod_ini.section_contains_property(&current_lod_name, "FILE") {
                    break
                }
                info!("Updating {}", current_lod_name);
                let old_value: String = ini_utils::get_value(&lod_ini, &current_lod_name, "FILE").unwrap();
                ini_utils::set_value(&mut lod_ini,
                                     &current_lod_name,
                                     "FILE",
                                     old_value.replace(name_to_change, new_car_name));
                idx += 1;
            }
            lod_ini.write_to_file(entry.path())?;
        }
    }

    for path in paths_to_update {
        let mut new_path = path.clone();
        let new_filename = path.file_name().unwrap().to_str().unwrap().replace(name_to_change, new_car_name);
        info!("Changing {} to {}", path.display(), new_filename);
        new_path.pop();
        new_path.push(new_filename);
        std::fs::rename(&path, &new_path)?;
    }
    Ok(())
}

pub fn update_car_ui_data(car_path: &Path, new_suffix: &str, parent_car_folder_name: &str) -> Result<()> {
    let mut car = Car::load_from_path(car_path)?;
    let mut existing_name = String::new();
    let mut new_name = String::new();
    {
        let mut ini_data = CarIniData::from_car(&mut car)?;
        existing_name = match ini_data.screen_name() {
            None => { String::from(car_path.file_name().unwrap().to_str().unwrap()) }
            Some(name) => { name }
        };
        new_name = existing_name.clone().add(format!(" {}", new_suffix).as_str());
        info!("Updating screen name and ui data from {} to {}", existing_name, new_name);
        ini_data.set_screen_name(new_name.as_str());
        ini_data.write()?;
    }

    {
        let mut ui_data = CarUiData::from_car(&mut car)?;
        ui_data.ui_info.set_name(new_name);
        match ui_data.ui_info.parent() {
            None => {
                info!("Updating parent name");
                ui_data.ui_info.set_parent(String::from(parent_car_folder_name));
            }
            Some(existing_parent) => {
                info!("Parent name already set to {}", existing_parent);
            }
        }
        ui_data.ui_info.add_tag("engine crane".to_owned());
        ui_data.write()?;
    }
    Ok(())
}

fn update_car_sfx(car_path: &Path, name_to_change: &str) -> Result<()> {
    let guids_file_path = car_path.join(PathBuf::from_iter(["sfx", "GUIDs.txt"]));
    let car_name = car_path.file_name().unwrap().to_str().unwrap();

    let mut updated_lines: Vec<String> = Vec::new();
    if guids_file_path.exists() {
        info!("Updating contents of '{}'. Replacing refs to '{}' with '{}'", guids_file_path.display(), name_to_change, car_name);
        let file = File::open(&guids_file_path)?;
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
        info!("Generating new '{}' with contents from the installation sfx data", guids_file_path.display());
        updated_lines = load_sfx_data()?.generate_clone_guid_info(name_to_change, car_name);
    }

    let file = File::create(&guids_file_path)?;
    let mut file = LineWriter::new(file);
    for line in updated_lines {
        write!(file, "{}\n", line)?;
    }
    Ok(())
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
    type Err = PropertyParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            CarVersion::VERSION_1 => Ok(CarVersion::One),
            CarVersion::VERSION_2 => Ok(CarVersion::Two),
            CarVersion::CSP_EXTENDED_2 => Ok(CarVersion::CspExtendedPhysics),
            _ => Err(PropertyParseError::new(s))
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
        let ui_info_string = fs::read_to_string(ui_json_path)?;
        let json_config: serde_json::Value = serde_json::from_str(ui_info_string
            .replace("\r\n", "\n")
            .replace("\n", " ")
            .replace("\t", "  ")
            .as_str())?;
        let ui_info = UiInfo {
            ui_info_path: ui_json_path.to_path_buf(),
            json_config
        };
        Ok(ui_info)
    }

    pub fn write(&self) -> Result<()> {
        let writer = BufWriter::new(File::create(&self.ui_info_path)?);
        serde_json::to_writer_pretty(writer, &self.json_config)?;
        Ok(())
    }

    pub fn name(&self) -> Option<&str> {
        self.get_json_string("name")
    }

    pub fn set_name(&mut self, name: String) {
        self.set_json_string("name", name);
    }

    pub fn parent(&self) -> Option<&str> {
        self.get_json_string("parent")
    }

    pub fn set_parent(&mut self, parent: String) {
        self.set_json_string("parent", parent);
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
                return Some(return_vec);
            }
        }
        None
    }

    pub fn add_tag(&mut self, new_tag: String) {
        let obj = self.json_config.as_object_mut().unwrap();
        if let Some(value) = obj.get_mut("tags") {
            if let Some(list) = value.as_array_mut() {
                list.push(serde_json::Value::String(new_tag));
            }
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
                return Some(return_map);
            }
        }
        None
    }

    pub fn update_spec(&mut self, spec_key: &str, val: String) {
        let obj = self.json_config.as_object_mut().unwrap();
        if let Some(value) = obj.get_mut("specs") {
            let map = value.as_object_mut().unwrap();
            map.remove(spec_key);
            map.insert(String::from(spec_key), serde_json::Value::String(val));
        }
    }

    pub fn torque_curve(&self) -> Option<Vec<Vec<&str>>> {
        self.load_curve_data("torqueCurve")
    }

    pub fn update_torque_curve(&mut self, new_curve_data: Vec<(i32, i32)>) {
        self.update_curve_data("torqueCurve", new_curve_data)
    }

    pub fn power_curve(&self) -> Option<Vec<Vec<&str>>> {
        self.load_curve_data("powerCurve")
    }

    pub fn update_power_curve(&mut self, new_curve_data: Vec<(i32, i32)>) {
        self.update_curve_data("powerCurve", new_curve_data)
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
            None => {
                if let Some(obj) = self.json_config.as_object_mut() {
                    obj.insert(String::from(key), serde_json::Value::String(value));
                }
            }
            Some(val) => {
                match val {
                    Value::String(str) => {
                        std::mem::replace(str, value);
                    }
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
                return Some(outer_vec);
            }
        }
        None
    }

    fn update_curve_data(&mut self, key: &str, new_curve_data: Vec<(i32, i32)>) {
        let mut data_vec: Vec<serde_json::Value> = Vec::new();
        for (rpm, power_bhp) in new_curve_data {
            data_vec.push(json!([format!("{}", rpm), format!("{}", power_bhp)]));
        }
        match self.json_config.get_mut(key) {
            None => {
                let map = self.json_config.as_object_mut().unwrap();
                map.insert(String::from(key),
                           serde_json::Value::Array(data_vec));
            }
            Some(val) => {
                let mut torque_array = val.as_array_mut().unwrap();
                torque_array.clear();
                for val in data_vec {
                    torque_array.push(val);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct CarIniData<'a> {
    car: &'a mut Car,
    ini_config: Ini,
}

impl<'a> CarIniData<'a> {
    pub fn from_car(car: &'a mut Car) -> Result<CarIniData<'a>> {
        let car_ini_data = car.data_interface.get_file_data("car.ini")?;
        Ok(CarIniData {
            car,
            ini_config: Ini::load_from_string(String::from_utf8_lossy(car_ini_data.as_slice()).into_owned())
        })
    }

    pub fn version(&self) -> Option<CarVersion> {
        ini_utils::get_value(&self.ini_config, "HEADER", "VERSION")
    }

    pub fn set_version(&mut self, version: CarVersion) {
        ini_utils::set_value(&mut self.ini_config, "HEADER", "VERSION", version);
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

    pub fn set_total_mass(&mut self, new_mass: u32) {
        ini_utils::set_value(&mut self.ini_config, "BASIC","TOTALMASS", new_mass);
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

    pub fn set_fuel_consumption(&mut self, consumption: f64) {
        ini_utils::set_float(&mut self.ini_config, "FUEL","CONSUMPTION", consumption, 4);
    }

    pub fn clear_fuel_consumption(&mut self) {
        self.ini_config.remove_value("FUEL", "CONSUMPTION");
    }

    pub fn write(&'a mut self) -> Result<()> {
        self.car.mut_data_interface().write_file_data("car.ini", self.ini_config.to_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct CarUiData<'a> {
    car: &'a mut Car,
    pub ui_info: UiInfo
}

impl<'a> CarUiData<'a> {
    pub fn from_car(car: &'a mut Car) -> Result<CarUiData<'a>> {
        let ui_info_path = car.root_path.join(["ui", "ui_car.json"].iter().collect::<PathBuf>());
        let ui_info = UiInfo::load(ui_info_path.as_path())?;
        Ok(CarUiData{
            car,
            ui_info
        })
    }

    pub fn write(&'a mut self) -> Result<()> {
        self.ui_info.write()
    }
}

#[derive(Debug)]
pub struct Car {
    root_path: PathBuf,
    data_interface: Box<dyn DataInterface>,
}

impl Car {
    pub fn load_from_path(car_folder_path: &Path) -> Result<Car> {
        let data_dir_path = car_folder_path.join("data");
        let data_interface = Box::new(DataFolderInterface::new(&data_dir_path));
        Ok(Car{
            root_path: car_folder_path.to_path_buf(),
            data_interface
        })
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn data_interface(&self) -> &dyn DataInterface {
        self.data_interface.as_ref()
    }

    pub fn mut_data_interface(&mut self) -> & mut dyn DataInterface {
        self.data_interface.as_mut()
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
