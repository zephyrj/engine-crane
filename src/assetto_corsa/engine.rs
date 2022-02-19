use std::collections::HashMap;
use std::ffi::OsString;
use std::{error, fs, io};
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::ops::Deref;
use std::path::Path;
use std::rc::{Rc, Weak};
use std::str::{FromStr};
use toml::Value;
use toml::value::Table;
use crate::assetto_corsa::engine::Source::{AssettoCorsa, Automation};
use crate::assetto_corsa::error::{Result, Error, ErrorKind, FieldParseError};
use crate::assetto_corsa::file_utils::load_ini_file_rc;
use crate::assetto_corsa::lut_utils::{load_lut_from_path, load_lut_from_reader};
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::Ini;


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

#[derive(Debug)]
struct Metadata {
    toml_config: toml::Value,
    boost_curve_data: Option<Vec<(i32, f64)>>,
    fuel_flow_data: Option<Vec<(i32, f64)>>
}

impl Metadata {
    fn load_from_dir(data_dir: &Path) -> Result<Option<Metadata>>{
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
            torque_curve: match load_lut_from_path::<i32, i32>(lut_path) {
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

#[derive(Debug)]
enum ControllerInput {
    Rpms
}

impl Default for ControllerInput {
    fn default() -> Self { ControllerInput::Rpms }
}

impl ControllerInput {
    pub const RPMS_VALUE :&'static str = "RPMS";

    pub fn as_str(&self) -> &'static str {
        match self {
            ControllerInput::Rpms => ControllerInput::RPMS_VALUE
        }
    }
}

impl FromStr for ControllerInput {
    type Err = FieldParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            ControllerInput::RPMS_VALUE => Ok(ControllerInput::Rpms),
            _ => Err(FieldParseError::new(s))
        }
    }
}

impl ToString for ControllerInput {
    fn to_string(&self) -> String {
        String::from(self.as_str())
    }
}

enum ControllerCombinator {
    Add
}

impl ControllerCombinator {
    pub fn as_str(&self) -> &'static str {
        match self {
            ControllerCombinator::Add => "ADD"
        }
    }
}

#[derive(Debug)]
pub struct ControllerCombinatorParseError{
    invalid_value: String
}

impl ControllerCombinatorParseError {
    pub(crate) fn new(invalid_value: &str) -> ControllerCombinatorParseError {
        ControllerCombinatorParseError{ invalid_value: String::from(invalid_value) }
    }
}

impl Display for ControllerCombinatorParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown value for ControllerCombinator: {}", self.invalid_value)
    }
}

impl error::Error for ControllerCombinatorParseError {}

impl FromStr for ControllerCombinator {
    type Err = ControllerCombinatorParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ADD" => Ok(ControllerCombinator::Add),
            _ => Err(ControllerCombinatorParseError::new(s))
        }
    }
}

impl ToString for ControllerCombinator {
    fn to_string(&self) -> String {
        String::from(self.as_str())
    }
}

#[derive(Debug)]
struct TurboController {
    data_dir: OsString,
    index: isize,
    ini_data: Weak<RefCell<Ini>>,
}

impl TurboController {
    fn input(&self) -> Option<ControllerInput> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,
                                           &TurboController::get_controller_section_name(self.index),
                                           "INPUT")
    }

    fn combinator(&self) -> Option<ControllerCombinator> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,
                                           &TurboController::get_controller_section_name(self.index),
                                           "COMBINATOR")
    }

    fn lut(&self) -> Option<Vec<(i32, f64)>> {
        let data: String = match ini_utils::get_value_from_weak_ref(
            &self.ini_data,
            &TurboController::get_controller_section_name(self.index),
            "LUT")
        {
            Some(str) => { str },
            None => { return None; }
        };
        return if data.starts_with("(") {
            let data_slice = &data[1..(data.len() - 1)];
            match load_lut_from_reader::<i32, f64, _>(data_slice.as_bytes()) {
                Ok(res) => {
                    Some(res)
                },
                Err(_) => {
                    None
                }
            }
        } else {
            match load_lut_from_path::<i32, f64>(Path::new(&self.data_dir).join(data.as_str()).as_path()) {
                Ok(res) => {
                    Some(res)
                }
                Err(_) => {
                    None
                }
            }
        }
    }

    fn filter(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,
                                           &TurboController::get_controller_section_name(self.index),
                                           "FILTER")
    }

    fn up_limit(&self) -> Option<i32> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,
                                           &TurboController::get_controller_section_name(self.index),
                                           "UP_LIMIT")
    }

    fn down_limit(&self) -> Option<i32> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,
                                           &TurboController::get_controller_section_name(self.index),
                                           "DOWN_LIMIT")
    }

    fn get_controller_section_name(index: isize) -> String {
        format!("CONTROLLER_{}", index)
    }
}

#[derive(Debug)]
struct TurboControllers {
    data_dir: OsString,
    index: isize,
    ini_config: Rc<RefCell<Ini>>,
    controllers: Vec<TurboController>
}

impl TurboControllers {
    fn load_controller_index_from_dir(index: isize, data_dir: &Path) -> Result<Option<TurboControllers>> {
        let ini_config = match load_ini_file_rc(
            Path::new(data_dir).join(TurboControllers::get_controller_ini_filename(index)).as_path())
        {
            Ok(res) => { res }
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    return Ok(None)
                }
                return Err(Error::new(ErrorKind::InvalidEngineTurboController,
                                      format!("Failed to load turbo controller with index {}: {}", index, err )));
            }
        };
        let mut turbo_controller_count: isize = 0;
        {
            let ini_ref = ini_config.borrow();
            turbo_controller_count = TurboControllers::count_turbo_controller_sections(ini_ref.deref());
        }

        let mut controller_vec: Vec<TurboController> = Vec::new();
        for idx in 0..turbo_controller_count {
            controller_vec.push(
                TurboController {
                    data_dir: OsString::from(data_dir),
                    index: idx,
                    ini_data: Rc::downgrade(&ini_config)
                });
        }

        Ok(Some(
            TurboControllers {
                data_dir: OsString::from(data_dir),
                index,
                ini_config,
                controllers: controller_vec
            }
        ))
    }

    fn get_controller_ini_filename(index: isize) -> String {
        format!("ctrl_turbo{}.ini", index)
    }

    fn count_turbo_controller_sections(ini: &Ini) -> isize {
        let mut count = 0;
        loop {
            if !ini.contains_section(TurboController::get_controller_section_name(count).as_str()) {
                return count;
            }
            count += 1;
        }
    }
}

#[derive(Debug)]
struct TurboSection {
    data_dir: OsString,
    index: isize,
    ini_data: Weak<RefCell<Ini>>,
    controllers: Option<TurboControllers>
}

impl TurboSection {
    pub fn load_from_ini(data_dir: &Path,
                         idx: isize,
                         ini: &Rc<RefCell<Ini>>) -> Result<TurboSection> {
        Ok(TurboSection {
            data_dir: OsString::from(data_dir),
            index: idx,
            ini_data: Rc::downgrade(ini),
            controllers: TurboControllers::load_controller_index_from_dir(idx, data_dir)? })
    }

    pub fn get_ini_section_name(&self) -> String {
        format!("TURBO_{}", self.index)
    }

    pub fn lag_dn(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,&self.get_ini_section_name(), "LAG_DN")
    }

    pub fn lag_up(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,&self.get_ini_section_name(), "LAG_UP")
    }

    pub fn max_boost(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,&self.get_ini_section_name(), "MAX_BOOST")
    }

    pub fn wastegate(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,&self.get_ini_section_name(), "WASTEGATE")
    }

    pub fn display_max_boost(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,&self.get_ini_section_name(), "DISPLAY_MAX_BOOST")
    }

    pub fn reference_rpm(&self) -> Option<i32> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,&self.get_ini_section_name(), "REFERENCE_RPM")
    }

    pub fn gamma(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,&self.get_ini_section_name(), "GAMMA")
    }

    pub fn cockpit_adjustable(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,&self.get_ini_section_name(), "COCKPIT_ADJUSTABLE")
    }
}

#[derive(Debug)]
struct Turbo {
    data_dir: OsString,
    ini_data: Weak<RefCell<Ini>>,
    sections: Vec<TurboSection>
}

impl Turbo {
    fn load_from_ini_data(data_dir: &Path, ini: &Rc<RefCell<Ini>>) -> Result<Option<Turbo>> {
        let mut turbo_count: isize = 0;
        {
            let ini_ref = ini.borrow();
            turbo_count = Turbo::count_turbo_sections(ini_ref.deref());
        }
        if turbo_count == 0 {
            return Ok(None);
        }
        let mut section_vec: Vec<TurboSection> = Vec::new();
        for idx in 0..turbo_count {
            section_vec.push(TurboSection::load_from_ini(data_dir, idx, ini)?);
        }
        Ok(Some(Turbo{
            data_dir: OsString::from(data_dir),
            ini_data: Rc::downgrade(ini),
            sections: section_vec
        }))
    }

    fn count_turbo_sections(ini: &Ini) -> isize {
        let mut count = 0;
        loop {
            if !ini.contains_section(format!("TURBO_{}", count).as_str()) {
                return count;
            }
            count += 1;
        }
    }

    pub fn boost_threshold(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,"DAMAGE", "TURBO_BOOST_THRESHOLD")
    }

    pub fn turbo_damage_k(&self) -> Option<i32> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,"DAMAGE", "TURBO_DAMAGE_K")
    }

    pub fn pressure_threshold(&self) -> Option<f64> {
        ini_utils::get_value_from_weak_ref(&self.ini_data,"BOV", "PRESSURE_THRESHOLD")
    }
}

#[derive(Debug)]
pub struct Engine {
    data_dir: OsString,
    ini_data: Rc<RefCell<Ini>>,
    metadata: Option<Metadata>,
    turbo: Option<Turbo>
}

impl Engine {
    const INI_FILENAME: &'static str = "engine.ini";

    pub fn load_from_dir(data_dir: &Path) -> Result<Engine> {
        let ini_data = match load_ini_file_rc(data_dir.join(Engine::INI_FILENAME).as_path()) {
            Ok(ini_object) => { ini_object }
            Err(err) => {
                return Err(Error::new(ErrorKind::InvalidCar, err.to_string() ));
            }
        };
        let turbo_option = match Turbo::load_from_ini_data(Path::new(data_dir),
                                                           &ini_data) {
            Ok(res) => { res }
            Err(err) => {
                return Err(Error::new(ErrorKind::InvalidCar, err.to_string() ));
            }
        };

        let eng = Engine {
            data_dir: OsString::from(data_dir),
            ini_data,
            metadata: match Metadata::load_from_dir(data_dir) {
                Ok(metadata_opt) => { metadata_opt }
                Err(e) => {
                    println!("Warning: Failed to load engine metadata");
                    println!("{}", e.to_string());
                    None
                }
            },
            turbo: turbo_option
        };
        Ok(eng)
    }
}


#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use crate::assetto_corsa::engine::Engine;

    #[test]
    fn load_engine() -> Result<(), String> {
        let path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/a1_science_car/data");
        match Engine::load_from_dir(&path) {
            Ok(_) => {
                Ok(())
            }
            Err(e) => { Err(e.to_string()) }
        }
    }
}