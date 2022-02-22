use std::collections::HashMap;
use std::ffi::OsString;
use std::{error, fs, io};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::{FromStr};
use toml::Value;
use toml::value::Table;
use crate::assetto_corsa::error::{Result, Error, ErrorKind, FieldParseError};
use crate::assetto_corsa::file_utils::load_ini_file;
use crate::assetto_corsa::lut_utils;
use crate::assetto_corsa::lut_utils::{load_lut_from_path, load_lut_from_reader};
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::{FromIni, get_mandatory_property, Ini, IniUpdater};


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

pub struct PowerCurve {
    lut_path: OsString,
    pub torque_curve: Vec<(i32, i32)>
}

impl PowerCurve {
    fn load_from_ini(ini_data: &Ini, data_dir: &Path) -> Result<Self> where Self: Sized {
        let power_lut_path: String = get_mandatory_property(ini_data, "HEADER", "POWER_CURVE")?;
        PowerCurve::load_from_lut(data_dir.join(Path::new(power_lut_path.as_str())).as_path())
    }

    fn load_from_lut(lut_path: &Path) -> Result<PowerCurve> {
        Ok(PowerCurve {
            lut_path: OsString::from(lut_path.as_os_str()),
            torque_curve: match load_lut_from_path::<i32, i32>(lut_path) {
                Ok(vec) => { vec },
                Err(err_str) => {
                    return Err(Error::new(ErrorKind::InvalidCar, err_str ));
                }
            }
        })
    }
}

impl IniUpdater for PowerCurve {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        lut_utils::write_lut_to_path(&self.torque_curve, PathBuf::from(&self.lut_path).as_path())?;
        ini_utils::set_value(ini_data, "HEADER", "POWER_CURVE", PathBuf::from(&self.lut_path).display());
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EngineData {
    altitude_sensitivity: f64,
    inertia: f64,
    limiter: i32,
    limiter_hz: i32,
    minimum: i32
}

impl EngineData {
    const SECTION_NAME: &'static str = "ENGINE_DATA";
}

impl FromIni for EngineData {
    fn load_from_ini(ini: &Ini) -> Result<EngineData> {
        Ok(EngineData{
            altitude_sensitivity: ini_utils::get_mandatory_property(ini, Self::SECTION_NAME, "ALTITUDE_SENSITIVITY")?,
            inertia: ini_utils::get_mandatory_property(ini, Self::SECTION_NAME, "INERTIA")?,
            limiter: ini_utils::get_mandatory_property(ini, Self::SECTION_NAME, "LIMITER")?,
            limiter_hz: ini_utils::get_mandatory_property(ini, Self::SECTION_NAME, "LIMITER_HZ")?,
            minimum: ini_utils::get_mandatory_property(ini, Self::SECTION_NAME, "MINIMUM")?
        })
    }
}

impl IniUpdater for EngineData {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_float(ini_data, Self::SECTION_NAME, "ALTITUDE_SENSITIVITY", self.altitude_sensitivity, 2);
        ini_utils::set_float(ini_data, Self::SECTION_NAME, "INERTIA", self.inertia, 3);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "LIMITER", self.limiter);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "LIMITER_HZ", self.limiter_hz);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "MINIMUM", self.minimum);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Damage {
    turbo_boost_threshold: f64,
    turbo_damage_k: i32,
    rpm_threshold: i32,
    rpm_damage_k: i32
}

impl Damage {
    const SECTION_NAME: &'static str = "DAMAGE";
}

impl FromIni for Damage {
    fn load_from_ini(ini: &Ini) -> Result<Damage> {
        Ok(Damage{
            turbo_boost_threshold: ini_utils::get_mandatory_property(ini, Self::SECTION_NAME, "TURBO_BOOST_THRESHOLD")?,
            turbo_damage_k: ini_utils::get_mandatory_property(ini, Self::SECTION_NAME, "TURBO_DAMAGE_K")?,
            rpm_threshold: ini_utils::get_mandatory_property(ini, Self::SECTION_NAME, "RPM_THRESHOLD")?,
            rpm_damage_k: ini_utils::get_mandatory_property(ini, Self::SECTION_NAME, "RPM_DAMAGE_K")?,
        })
    }
}

impl IniUpdater for Damage {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_float(ini_data, Self::SECTION_NAME, "TURBO_BOOST_THRESHOLD", self.turbo_boost_threshold, 2);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "TURBO_DAMAGE_K", self.turbo_damage_k);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "RPM_THRESHOLD", self.rpm_threshold);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "RPM_DAMAGE_K", self.rpm_damage_k);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CoastSource {
    FromCoastRef
}

impl CoastSource {
    pub const FROM_COAST_REF_VALUE: &'static str = "FROM_COAST_REF";
    pub const COAST_REF_SECTION_NAME: &'static str = "COAST_REF";

    pub fn as_str(&self) -> &'static str {
        match self { CoastSource::FromCoastRef => CoastSource::FROM_COAST_REF_VALUE }
    }

    pub fn get_section_name(&self) -> &'static str {
        match self { CoastSource::FromCoastRef => CoastSource::COAST_REF_SECTION_NAME }
    }
}

impl FromStr for CoastSource {
    type Err = FieldParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            CoastSource::FROM_COAST_REF_VALUE => Ok(CoastSource::FromCoastRef),
            _ => Err(FieldParseError::new(s))
        }
    }
}

impl Display for CoastSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
pub struct CoastCurve {
    curve_data_source: CoastSource,
    reference_rpm: i32,
    torque: i32,
    non_linearity: f64
}

impl FromIni for CoastCurve {
    fn load_from_ini(ini: &Ini) -> Result<CoastCurve> {
        let curve_data_source: CoastSource = ini_utils::get_mandatory_property(ini, "HEADER", "COAST_CURVE")?;
        let section_name = curve_data_source.get_section_name();
        Ok(CoastCurve{
            curve_data_source,
            reference_rpm: ini_utils::get_mandatory_property(ini, section_name, "RPM")?,
            torque: ini_utils::get_mandatory_property(ini, section_name, "TORQUE")?,
            non_linearity: ini_utils::get_mandatory_property(ini, section_name, "NON_LINEARITY")?,
        })
    }
}

impl IniUpdater for CoastCurve {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        return match self.curve_data_source {
            CoastSource::FromCoastRef => {
                let section_name = self.curve_data_source.get_section_name();
                ini_utils::set_value(ini_data, "HEADER", "COAST_CURVE", &self.curve_data_source);
                ini_utils::set_value(ini_data, section_name, "RPM", self.reference_rpm);
                ini_utils::set_value(ini_data, section_name, "TORQUE", self.torque);
                ini_utils::set_float(ini_data, section_name, "NON_LINEARITY", self.non_linearity, 2);
                Ok(())
            }
        }
    }
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

impl Display for ControllerInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
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


impl FromStr for ControllerCombinator {
    type Err = FieldParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ADD" => Ok(ControllerCombinator::Add),
            _ => Err(FieldParseError::new(s))
        }
    }
}

impl Display for ControllerCombinator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
struct TurboController {
    data_dir: OsString,
    index: isize,
    input: ControllerInput,
    combinator: ControllerCombinator,
    lut: Vec<(i32, f64)>,
    filter: f64,
    up_limit: i32,
    down_limit: i32
}

impl TurboController {
    pub fn load_from_ini(ini: &Ini, idx: isize, data_dir: &Path) -> Result<TurboController> {
        let section_name = TurboController::get_controller_section_name(idx);
        let lut = lut_utils::load_lut_from_property_value(
            ini_utils::get_mandatory_property(ini, &section_name, "LUT")?,
            data_dir
        ).map_err(
            |err_str| {
                Error::new(ErrorKind::InvalidEngineTurboController,
                           format!("Failed to load turbo controller with index {}: {}", idx, err_str ))
            })?;

        Ok(TurboController {
            data_dir: OsString::from(data_dir),
            index: idx,
            input: ini_utils::get_mandatory_property(ini, &section_name, "INPUT")?,
            combinator: ini_utils::get_mandatory_property(ini, &section_name, "COMBINATOR")?,
            lut,
            filter: ini_utils::get_mandatory_property(ini, &section_name, "FILTER")?,
            up_limit: ini_utils::get_mandatory_property(ini, &section_name, "UP_LIMIT")?,
            down_limit: ini_utils::get_mandatory_property(ini, &section_name, "DOWN_LIMIT")?
        })
    }

    fn get_controller_section_name(index: isize) -> String {
        format!("CONTROLLER_{}", index)
    }
}

#[derive(Debug)]
struct TurboControllers {
    data_dir: OsString,
    index: isize,
    ini_config: Ini,
    controllers: Vec<TurboController>
}

impl TurboControllers {
    fn load_controller_index_from_dir(index: isize, data_dir: &Path) -> Result<Option<TurboControllers>> {
        let ini_config = match load_ini_file(
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
        let mut turbo_controller_count: isize = TurboControllers::count_turbo_controller_sections(&ini_config);

        let mut controller_vec: Vec<TurboController> = Vec::new();
        for idx in 0..turbo_controller_count {
            controller_vec.push(TurboController::load_from_ini(&ini_config, idx, data_dir)?);
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
    lag_dn: f64,
    lag_up: f64,
    max_boost: f64,
    wastegate: f64,
    display_max_boost: f64,
    reference_rpm: i32,
    gamma: f64,
    cockpit_adjustable: i32,
    controllers: Option<TurboControllers>
}

impl TurboSection {
    pub fn load_from_ini(data_dir: &Path,
                         idx: isize,
                         ini: &Ini) -> Result<TurboSection> {
        let section_name = TurboSection::get_ini_section_name(idx);
        Ok(TurboSection {
            data_dir: OsString::from(data_dir),
            index: idx,
            lag_dn: ini_utils::get_mandatory_property(ini, &section_name, "LAG_DN")?,
            lag_up: ini_utils::get_mandatory_property(ini, &section_name, "LAG_UP")?,
            max_boost: ini_utils::get_mandatory_property(ini, &section_name, "MAX_BOOST")?,
            wastegate: ini_utils::get_mandatory_property(ini, &section_name, "WASTEGATE")?,
            display_max_boost: ini_utils::get_mandatory_property(ini, &section_name, "DISPLAY_MAX_BOOST")?,
            reference_rpm: ini_utils::get_mandatory_property(ini, &section_name, "REFERENCE_RPM")?,
            gamma: ini_utils::get_mandatory_property(ini, &section_name, "GAMMA")?,
            cockpit_adjustable: ini_utils::get_mandatory_property(ini, &section_name, "COCKPIT_ADJUSTABLE")?,
            controllers: TurboControllers::load_controller_index_from_dir(idx, data_dir)?
        })
    }

    pub fn get_ini_section_name(idx: isize) -> String {
        format!("TURBO_{}", idx)
    }
}

#[derive(Debug)]
pub struct Turbo {
    data_dir: OsString,
    bov_pressure_threshold: f64,
    sections: Vec<TurboSection>
}

impl Turbo {
    fn load_from_ini_data(data_dir: &Path, ini: &Ini) -> Result<Option<Turbo>> {
        let mut turbo_count: isize = Turbo::count_turbo_sections(ini);
        if turbo_count == 0 {
            return Ok(None);
        }
        let pressure_threshold = ini_utils::get_mandatory_property(ini, "BOV", "PRESSURE_THRESHOLD")?;
        let mut section_vec: Vec<TurboSection> = Vec::new();
        for idx in 0..turbo_count {
            section_vec.push(TurboSection::load_from_ini(data_dir, idx, ini)?);
        }
        Ok(Some(Turbo{
            data_dir: OsString::from(data_dir),
            bov_pressure_threshold: pressure_threshold,
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
}

#[derive(Debug)]
pub struct Engine {
    data_dir: OsString,
    ini_data: Ini
}

impl Engine {
    const INI_FILENAME: &'static str = "engine.ini";

    pub fn load_from_dir(data_dir: &Path) -> Result<Engine> {
        let ini_data = match load_ini_file(data_dir.join(Engine::INI_FILENAME).as_path()) {
            Ok(ini_object) => { ini_object }
            Err(err) => {
                return Err(Error::new(ErrorKind::InvalidCar, err.to_string() ));
            }
        };

        let eng = Engine {
            data_dir: OsString::from(data_dir),
            ini_data
        };
        Ok(eng)
    }

    pub fn metadata(&self) -> Result<Option<Metadata>> {
        Metadata::load_from_dir(Path::new(&self.data_dir.as_os_str()))
    }

    pub fn power_curve(&self) -> Result<PowerCurve> {
        PowerCurve::load_from_ini(&self.ini_data, Path::new(self.data_dir.as_os_str()))
    }

    pub fn coast_curve(&self) -> Result<CoastCurve> {
        CoastCurve::load_from_ini(&self.ini_data)
    }

    pub fn engine_data(&self) -> Result<EngineData> {
        EngineData::load_from_ini(&self.ini_data)
    }

    pub fn damage(&self) -> Result<Damage> {
        Damage::load_from_ini(&self.ini_data)
    }

    pub fn turbo(&self) -> Result<Option<Turbo>> {
        Turbo::load_from_ini_data(Path::new(&self.data_dir.as_os_str()),
                                  &self.ini_data)
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
            Ok(engine) => {
                let metadata = engine.metadata().map_err(|err|{
                    err.to_string()
                })?;
                let power_curve = engine.power_curve().map_err(|err|{
                    err.to_string()
                })?;
                let coast_curve = engine.coast_curve().map_err(|err|{
                    err.to_string()
                })?;
                let engine_data = engine.engine_data().map_err(|err|{
                    err.to_string()
                })?;
                let damage = engine.damage().map_err(|err|{
                    err.to_string()
                })?;
                let turbo = engine.turbo().map_err(|err|{
                    err.to_string()
                })?;
                assert!(turbo.is_some());
                Ok(())
            }
            Err(e) => { Err(e.to_string()) }
        }
    }
}