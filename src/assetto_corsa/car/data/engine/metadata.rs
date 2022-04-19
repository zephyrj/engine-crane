use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::path::Path;
use toml::Value;
use toml::value::Table;
use crate::assetto_corsa::error::{Result};
use crate::assetto_corsa::car::lut_utils::load_lut_from_path;


pub enum Source {
    AssettoCorsa,
    Automation
}

impl Source {
    fn from_str(str: &str) -> Option<Source> {
        match str {
            "ac" => Some(Source::AssettoCorsa),
            "automation" => Some(Source::Automation),
            _ => None
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Source::AssettoCorsa => "ac",
            Source::Automation => "automation"
        }
    }
}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}


#[derive(Debug)]
pub struct Metadata {
    toml_config: toml::Value,
    boost_curve_data: Option<Vec<(i32, f64)>>,
    fuel_flow_data: Option<Vec<(i32, f64)>>
}

impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            toml_config: toml::Value::Table(toml::map::Map::new()),
            boost_curve_data: None,
            fuel_flow_data: None
        }
    }

    fn load_from_dir(data_dir: &Path) -> Result<Option<Metadata>>{
        let metadata_path = Path::new(data_dir).join("engine-metadata.toml");
        if !metadata_path.exists() {
            return Ok(None);
        }

        let metadata_string = fs::read_to_string(&metadata_path)?;
        let toml_config = metadata_string.parse::<Value>()?;

        let meta = Metadata{
            toml_config,
            boost_curve_data: Metadata::find_boost_curve_data(data_dir),
            fuel_flow_data: Metadata::find_fuel_flow_data(data_dir)
        };
        Ok(Some(meta))
    }

    fn find_boost_curve_data(data_dir: &Path) -> Option<Vec<(i32, f64)>> {
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

    fn find_fuel_flow_data(data_dir: &Path) -> Option<Vec<(i32, f64)>> {
        let fuel_flow_lut_path = Path::new(data_dir).join("max_flow.lut");
        if !fuel_flow_lut_path.exists() {
            return None;
        }

        match load_lut_from_path::<i32, f64>(fuel_flow_lut_path.as_path()) {
            Ok(vec) => { Some(vec) }
            Err(err_str) => {
                println!("Failed to open {}: {}", fuel_flow_lut_path.display(), err_str);
                None
            }
        }
    }

    fn latest_version() -> i64 {
        2
    }

    pub fn version(&self) -> i64 {
        if let Some(version_field) = self.toml_config.get("version") {
            if let Some(version_num) = version_field.as_integer() {
                return version_num;
            }
        }
        1
    }

    pub fn set_version(&mut self, version: i64) {
        self.set_int_value(String::from("version"), version);
    }

    pub fn source(&self) -> Option<Source> {
        if let Some(source_field) = self.toml_config.get("source") {
            if let Some(source) = source_field.as_str() {
                return Source::from_str(source)
            }
        }
        None
    }

    pub fn set_source(&mut self, source: Source) {
        self.set_string_value(String::from("source"), source.to_string());
    }

    pub fn mass_kg(&self) -> Option<i64> {
        if let Some(mass_field) = self.toml_config.get("mass_kg") {
            return mass_field.as_integer() ;
        }
        None
    }

    pub fn set_mass_kg(&mut self, mass: i64) {
        self.set_int_value(String::from("mass_kg"), mass);
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

    pub fn set_string_value(&mut self, key: String, val: String) -> Option<toml::Value> {
        self.toml_config.as_table_mut().unwrap().insert(key, toml::Value::String(val))
    }

    pub fn set_int_value(&mut self, key: String, val: i64) -> Option<toml::Value> {
        self.toml_config.as_table_mut().unwrap().insert(key, toml::Value::Integer(val))
    }
}
