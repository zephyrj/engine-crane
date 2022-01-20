use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::{fs, io};
use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::rc::{Rc, Weak};
use configparser::ini::Ini;
use toml::Value;
use toml::value::Table;
use crate::assetto_corsa::engine::Source::{AssettoCorsa, Automation};
use crate::assetto_corsa::error::{Result, Error, ErrorKind};
use crate::assetto_corsa::file_utils::{load_ini_file, load_ini_file_rc, load_lut};


struct UiData {
    torque_curve: Vec<Vec<String>>,
    power_curve: Vec<Vec<String>>,
    max_torque: String,
    max_power: String
}

enum Source {
    AssettoCorsa,
    Automation
}

impl Source {
    fn from_str(str: &str) -> Option<Source> {
        match str {
            "ac" => Some(AssettoCorsa),
            "automation" => Some(Automation),
            _ => None
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            AssettoCorsa => "ac",
            Automation => "automation"
        }
    }
}

struct Metadata {
    toml_config: toml::Value,
    boost_curve_data: Option<Vec<(i32, f64)>>,
    fuel_flow_data: Option<Vec<(i32, f64)>>
}

impl Metadata {
    fn load_from_dir(data_dir: &OsStr) -> Result<Option<Metadata>>{
        let metadata_path = Path::new(data_dir).join("engine-metadata.toml");
        if !metadata_path.exists() {
            return Ok(None);
        }

        let metadata_string = match fs::read_to_string(&metadata_path) {
            Ok(str) => { str }
            Err(e) => {
                return Err( Error::new(ErrorKind::InvalidEngineMetadata,
                                       String::from(format!("Failed to read {}: {}",
                                                            metadata_path.display(),
                                                            e.to_string()))) )
            }
        };
        let toml_config = match metadata_string.parse::<Value>() {
            Ok(decoded_toml) => { decoded_toml },
            Err(e) => {
                return Err( Error::new(ErrorKind::InvalidEngineMetadata,
                                       String::from(format!("Failed to decode {}: {}",
                                                            metadata_path.display(),
                                                            e.to_string()))) )
            }
        };

        let mut meta = Metadata{
            toml_config,
            boost_curve_data: Metadata::find_boost_curve_data(data_dir),
            fuel_flow_data: Metadata::find_fuel_flow_data(data_dir)
        };
        Ok(Some(meta))
    }

    fn find_boost_curve_data(data_dir: &OsStr) -> Option<Vec<(i32, f64)>> {
        let boost_curve_path = Path::new(data_dir).join("boost.csv");
        if !boost_curve_path.exists() {
            return None;
        }

        let file = match File::open(&boost_curve_path) {
            Ok(file) => { file }
            Err(e) => {
                println!("Failed to open {}: {}", boost_curve_path.display(), e.to_string());
                return None;
            }
        };

        let mut boost_curve_data: Vec<(i32, f64)> = Vec::new();
        let mut rdr = csv::Reader::from_reader(file);
        for result in rdr.records() {
            match result {
                Ok(record) => {
                    boost_curve_data.push((record.get(0).unwrap().parse::<i32>().unwrap(),
                                           record.get(1).unwrap().parse::<f64>().unwrap()));
                },
                _ => {}
            }
        }
        Some(boost_curve_data)
    }

    fn find_fuel_flow_data(data_dir: &OsStr) -> Option<Vec<(i32, f64)>> {
        let fuel_flow_lut_path = Path::new(data_dir).join("max_flow.lut");
        if !fuel_flow_lut_path.exists() {
            return None;
        }

        match load_lut::<i32, f64>(fuel_flow_lut_path.as_path()) {
            Ok(vec) => { Some(vec) }
            Err(err_str) => {
                println!("Failed to open {}: {}", fuel_flow_lut_path.display(), err_str);
                None
            }
        }
    }

    fn latest_version() -> isize {
        2
    }

    fn version(&self) -> isize {
        if let Some(version_field) = self.toml_config.get("version") {
            if let Some(version_num) = version_field.as_integer() {
                return version_num as isize;
            }
        }
        1
    }

    fn source(&self) -> Option<Source> {
        if let Some(source_field) = self.toml_config.get("source") {
            if let Some(source) = source_field.as_str() {
                return Source::from_str(source)
            }
        }
        None

    }

    fn ui_data(&self) -> Option<UiData> {
        None
    }

    fn mass_kg(&self) -> Option<i64> {
        if let Some(mass_field) = self.toml_config.get("mass_kg") {
            return mass_field.as_integer() ;
        }
        None
    }

    fn boost_curve_data(&self) -> Option<&Vec<(i32, f64)>> {
        match &self.boost_curve_data {
            Some(data) => Some(data),
            None => None
        }
    }

    fn fuel_flow_data(&self) -> Option<&Vec<(i32, f64)>> {
        match &self.fuel_flow_data {
            Some(data) => Some(data),
            None => None
        }
    }

    fn info_map(&self) -> Option<&Table> {
        if let Some(info) = self.toml_config.get("info_dict") {
            if let Some(data) = info.as_table() {
                return Some(data);
            }
        }
        None
    }
}

struct ExtendedFuelConsumptionBaseData {
    idle_throttle: f64,
    idle_cutoff: i32,
    mechanical_efficiency: f64
}

struct FuelConsumptionEfficiency {
    base_data: ExtendedFuelConsumptionBaseData,
    thermal_efficiency: f64,
    thermal_efficiency_dict: Option<HashMap<i32, f64>>,
    fuel_lhv: i32,
    turbo_efficiency: Option<f64>
}

struct FuelConsumptionFlowRate {
    base_data: ExtendedFuelConsumptionBaseData,
    max_fuel_flow_lut: Option<String>,
    max_fuel_flow: i32
}

struct Power {
    lut_path: OsString,
    torque_curve: Vec<(i32, i32)>
}

impl Power {
    fn load_from_lut(lut_path: &Path) -> Result<Power> {
        Ok(Power {
            lut_path: OsString::from(lut_path.as_os_str()),
            torque_curve: match load_lut::<i32, i32>(lut_path) {
                Ok(vec) => { vec },
                Err(err_str) => {
                    return Err(Error::new(ErrorKind::InvalidCar, err_str ));
                }
            }
        })
    }

    fn torque_curve(&self) -> &Vec<(i32, i32)> {
        &self.torque_curve
    }
}

enum CoastSource {
    FromCoastRef
}

struct CoastCurve {
    curve_data_source: CoastSource,
    reference_rpm: i32,
    torque: i32,
    non_linearity: i32
}

enum ControllerInput {
    Rpms
}

enum ControllerCombinator {
    Add
}

struct TurboController {
    index: isize,
    input: ControllerInput,
    combinator: ControllerCombinator,
    lut: Vec<(i32, f64)>,
    filter: f64,
    up_limit: i32,
    down_limit: i32
}

struct TurboControllers {
    index: isize,
    ini_config: Ini,
    controllers: Vec<TurboController>
}

struct TurboSection {
    index: isize,
    ini_data: Weak<RefCell<Ini>>,
    controllers: Option<TurboControllers>
}

impl TurboSection {
    pub fn get_ini_section_name(&self) -> String {
        format!("TURBO_{}", self.index)
    }

    pub fn lag_dn(&self) -> Option<f64> {
        get_value(&self.ini_data,&self.get_ini_section_name(), "LAG_DN")
    }

    pub fn lag_up(&self) -> Option<f64> {
        get_value(&self.ini_data,&self.get_ini_section_name(), "LAG_UP")
    }

    pub fn max_boost(&self) -> Option<f64> {
        get_value(&self.ini_data,&self.get_ini_section_name(), "MAX_BOOST")
    }

    pub fn wastegate(&self) -> Option<f64> {
        get_value(&self.ini_data,&self.get_ini_section_name(), "WASTEGATE")
    }

    pub fn display_max_boost(&self) -> Option<f64> {
        get_value(&self.ini_data,&self.get_ini_section_name(), "DISPLAY_MAX_BOOST")
    }

    pub fn reference_rpm(&self) -> Option<i32> {
        get_value(&self.ini_data,&self.get_ini_section_name(), "REFERENCE_RPM")
    }

    pub fn gamma(&self) -> Option<f64> {
        get_value(&self.ini_data,&self.get_ini_section_name(), "GAMMA")
    }

    pub fn cockpit_adjustable(&self) -> Option<f64> {
        get_value(&self.ini_data,&self.get_ini_section_name(), "COCKPIT_ADJUSTABLE")
    }
}

struct Turbo {
    ini_data: Weak<RefCell<Ini>>,
    sections: Vec<TurboSection>
}

impl Turbo {
    fn load_from_ini_data(ini: &Rc<RefCell<Ini>>) -> Result<Option<Turbo>> {
        {
            let ini_ref = ini.borrow();
            let map_ref = ini_ref.get_map_ref();
            if !map_ref.contains_key("TURBO_0") {
                return Ok(None);
            }
        }
        Ok(Some(Turbo{
            ini_data: Rc::downgrade(ini),
            sections: vec![
                TurboSection { index: 0, ini_data: Rc::downgrade(ini), controllers: None }]
        }))
    }

    pub fn boost_threshold(&self) -> Option<f64> {
        get_value(&self.ini_data,"DAMAGE", "TURBO_BOOST_THRESHOLD")
    }

    pub fn turbo_damage_k(&self) -> Option<i32> {
        get_value(&self.ini_data,"DAMAGE", "TURBO_DAMAGE_K")
    }

    pub fn pressure_threshold(&self) -> Option<f64> {
        get_value(&self.ini_data,"BOV", "PRESSURE_THRESHOLD")
    }
}

fn get_value<T: std::str::FromStr>(ini_data: &Weak<RefCell<Ini>>,
                                   section: &str,
                                   key: &str) -> Option<T> {
    let ini = ini_data.upgrade()?;
    let ini_ref = ini.borrow();
    let item = ini_ref.get(section, key)?;
    match item.parse::<T>() {
        Ok(val) => { Some(val) }
        Err(_) => { None }
    }
}

struct Engine {
    data_dir: OsString,
    ini_data: Rc<RefCell<Ini>>,
    metadata: Option<Metadata>,
    turbo: Option<Turbo>
}

impl Engine {
    const INI_FILENAME: &'static str = "engine.ini";

    fn load_from_dir(data_dir: &OsStr) -> Result<Engine> {
        let eng = Engine {
            data_dir: OsString::from(data_dir),
            ini_data: match load_ini_file_rc(Path::new(data_dir).join(Engine::INI_FILENAME).as_path()) {
                Ok(ini_object) => { ini_object }
                Err(err_str) => {
                    return Err(Error::new(ErrorKind::InvalidCar, err_str ))
                }},
            metadata: match Metadata::load_from_dir(data_dir) {
                Ok(metadata_opt) => { metadata_opt }
                Err(e) => {
                    println!("Warning: Failed to load engine metadata");
                    println!("{}", e.to_string());
                    None
                }
            },
            turbo: None,
        };
        Ok(eng)
    }
}
