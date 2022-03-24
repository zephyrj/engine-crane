use std::collections::HashMap;
use std::{fs, mem};
use std::default::Default;

use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufReader, BufRead, LineWriter, Write, BufWriter, Read};
use std::ops::Add;
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

pub fn update_car_name(car_path: &Path, new_suffix: &str) -> Result<()> {
    let mut new_car = Car::load_from_path(car_path)?;
    let existing_name = match new_car.screen_name() {
        None => { String::from(car_path.file_name().unwrap().to_str().unwrap()) }
        Some(name) => { name }
    };
    let new_name = existing_name.add(format!(" {}", new_suffix).as_str());
    new_car.set_screen_name(new_name.as_str());
    new_car.ui_info.set_name(new_name);
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
    fix_car_specific_filenames(new_car_path, existing_car_name)?;
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
    let new_car_name = format!("{}_{}", existing_car_name, spec_name.to_lowercase());
    let new_car_path = installed_cars_path.join(&new_car_name);
    if new_car_path.exists() {
        return Err(Error::new(ErrorKind::CarAlreadyExists,
                              format!("{}", new_car_path.display())));
    }
    clone_existing_car(existing_car_path.as_path(), new_car_path.as_path())?;
    update_car_name(new_car_path.as_path(), spec_name)?;
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

    pub fn write(&self) -> Result<()> {
        let out_path = self.root_path.join(["data", "car.ini"].iter().collect::<PathBuf>());
        self.ini_config.write(&out_path).map_err(|err| {
            Error::from_io_error(err,
                                 format!("Failed to parse {}", out_path.display()).as_str())
        })?;
        self.ui_info.write()
    }

    pub fn drivetrain(&self) -> &Drivetrain {
        &self.drivetrain
    }

    pub fn mut_engine(&mut self) -> &mut Engine {
        &mut self.engine
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

/// Credit for this goes to Luigi Auriemma (me@aluigi.org)
/// This is derived from his quickBMS script which can be found at:
/// https://zenhax.com/viewtopic.php?f=9&t=90&sid=330e7fe17c78d2bfe2d7e8b7227c6143
pub fn derive_acd_extraction_key(folder_name: &str) -> String {
    let mut key_list: Vec<String> = Vec::with_capacity(8);
    let mut push_key_component = |val: i64| { key_list.push((val & 0xff).to_string()) };

    let mut key_1 = 0_i64;
    folder_name.chars().for_each(|c| key_1 += u64::from(c) as i64);
    push_key_component(key_1);

    let mut key_2: i64 = 0;
    for idx in (0..folder_name.len()-1).step_by(2) {
        key_2 *= u64::from(folder_name.chars().nth(idx).unwrap()) as i64;
        key_2 -= u64::from(folder_name.chars().nth(idx+1).unwrap()) as i64;
    }
    push_key_component(key_2);

    let mut key_3: i64 = 0;
    for idx in (1..folder_name.len()-3).step_by(3) {
        key_3 *= u64::from(folder_name.chars().nth(idx).unwrap()) as i64;
        key_3 /= (u64::from(folder_name.chars().nth(idx+1).unwrap()) as i64) + 0x1b;
        key_3 += -0x1b - u64::from(folder_name.chars().nth(idx-1).unwrap()) as i64;
    }
    push_key_component(key_3);

    let mut key_4 = 0x1683_i64;
    folder_name[1..].chars().for_each(|c| key_4 -= u64::from(c) as i64);
    push_key_component(key_4);

    let mut key_5 = 0x42_i64;
    for idx in (1..folder_name.len()-4).step_by(4) {
        let mut tmp = u64::from(folder_name.chars().nth(idx).unwrap()) as i64 + 0xf;
        tmp *= key_5;
        let mut tmp2 = u64::from(folder_name.chars().nth(idx-1).unwrap()) as i64 + 0xf;
        tmp2 *= tmp;
        tmp2 += 0x16;
        key_5 = tmp2;
    }
    push_key_component(key_5);

    let mut key_6 = 0x65_i64;
    folder_name[0..folder_name.len()-2].chars().step_by(2).for_each(|c| key_6 -= u64::from(c) as i64 );
    push_key_component(key_6);

    let mut key_7 = 0xab_i64;
    folder_name[0..folder_name.len()-2].chars().step_by(2).for_each(|c| key_7 %= u64::from(c) as i64 );
    push_key_component(key_7);

    let mut key_8 = 0xab;
    for idx in 0..folder_name.len()-1 {
        key_8 /= u64::from(folder_name.chars().nth(idx).unwrap()) as i64;
        key_8 += u64::from(folder_name.chars().nth(idx+1).unwrap()) as i64
    }
    push_key_component(key_8);

    key_list.join("-")
}

/// Credit for this goes to Luigi Auriemma (me@aluigi.org)
/// This is derived from his quickBMS script which can be found at:
/// https://zenhax.com/viewtopic.php?f=9&t=90&sid=330e7fe17c78d2bfe2d7e8b7227c6143
pub fn extract_data_acd(acd_path: &Path, output_directory_path: &Path) {
    let folder_name = String::from(acd_path.parent().unwrap().file_name().unwrap().to_str().unwrap());
    let extraction_key = derive_acd_extraction_key(&folder_name);

    let f = File::open(acd_path).unwrap();
    let mut reader = BufReader::new(f);
    let mut packed_buffer = Vec::new();
    reader.read_to_end(&mut packed_buffer).unwrap();
    if !output_directory_path.is_dir() {
        std::fs::create_dir(output_directory_path).unwrap();
    }

    type LengthField = u32;
    let mut current_pos: usize = 0;
    while current_pos < packed_buffer.len() {
        // 4 bytes contain the length of filename
        let filename_len = LengthField::from_le_bytes(packed_buffer[current_pos..(current_pos+mem::size_of::<LengthField>())].try_into().expect("Failed to parse filename length"));
        current_pos += mem::size_of::<LengthField>();

        // The next 'filename_len' bytes are the filename
        let filename = String::from_utf8(packed_buffer[current_pos..(current_pos + filename_len as usize)].to_owned()).expect("Failed to parse filename");
        current_pos += filename_len as usize;

        // The next 4 bytes contain the length of the file content
        let mut content_length = LengthField::from_le_bytes(packed_buffer[current_pos..(current_pos+mem::size_of::<LengthField>())].try_into().expect("Failed to parse filename length"));
        current_pos += mem::size_of::<LengthField>();

        // The file content is spread out such that each byte of content is stored in 4 bytes.
        // Read each single byte of content, subtract the value of the extraction key from it and store it.
        // Move along the packed data by 4 bytes to the next byte of content, increment the extraction key position by 1 and repeat
        // Loop back to the start of the extraction key if we hit the end.
        // Repeat until we have read the full content for the file
        let mut unpacked_buffer: Vec<u8> = Vec::new();
        let mut key_byte_iter = extraction_key.chars().cycle();
        packed_buffer[current_pos..current_pos+(content_length*4) as usize].iter().step_by(4).for_each(|byte|{
            unpacked_buffer.push(byte - u32::from(key_byte_iter.next().unwrap()) as u8);
        });
        println!("{} - {} bytes", filename, content_length);
        fs::write(output_directory_path.join(filename), unpacked_buffer).unwrap();
        current_pos += (content_length*4) as usize;
    }
}


#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::assetto_corsa::car::{Car, create_new_car_spec, derive_acd_extraction_key, extract_data_acd};

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

    #[test]
    fn derive_acd_key() {
        assert_eq!(derive_acd_extraction_key("abarth500"), "7-248-6-221-246-250-21-49");
    }

    #[test]
    fn extract_acd() {
        let path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/abarth500/data.acd");
        let out_path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/abarth500/data");
        extract_data_acd(path, out_path);
    }
}
