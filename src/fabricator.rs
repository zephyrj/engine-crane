/*
 * Copyright (c):
 * 2023 zephyrj
 * zephyrj@protonmail.com
 *
 * This file is part of engine-crane.
 *
 * engine-crane is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * engine-crane is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
 */

use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::path::Path;
use itertools::Itertools;
use serde_hjson;
use sha2::{Sha256, Digest};
use tracing::{debug, error, info, warn};

use assetto_corsa::car::data::engine::{CoastCurve, Damage, EngineData, PowerCurve};
use utils::numeric::{round_float_to, round_up_to_nearest_multiple};
use crate::{beam_ng, utils};
use crate::assetto_corsa::Car;
use crate::assetto_corsa::car::data;
use crate::assetto_corsa::car::data::ai::Ai;
use crate::assetto_corsa::car::data::CarIniData;
use crate::assetto_corsa::car::data::car_ini_data::CarVersion;
use crate::assetto_corsa::car::data::digital_instruments::DigitalInstruments;
use crate::assetto_corsa::car::data::digital_instruments::shift_lights::ShiftLights;
use crate::assetto_corsa::car::ui::CarUiData;
use crate::assetto_corsa::car::data::Drivetrain;
use crate::assetto_corsa::car::data::Engine;
use crate::assetto_corsa::car::data::engine;
use crate::assetto_corsa::car::data::engine::turbo_ctrl::delete_all_turbo_controllers_from_car;
use crate::data::{AutomationSandboxCrossChecker, CrateEngine};

use crate::assetto_corsa::traits::{extract_mandatory_section, extract_optional_section, OptionalDataSection, update_car_data};
use crate::automation::car::{CarFile};
use crate::automation::sandbox::{EngineV1, load_engine_by_uuid, SandboxVersion};
use crate::fabricator::FabricationError::{InvalidData, MissingDataSection, MissingDataSource};

#[derive(thiserror::Error, Debug)]
pub enum FabricationError {
    #[error("io error")]
    IoError(#[from] io::Error),
    #[error("assetto corsa data error")]
    ACDataError(#[from] assetto_corsa::error::Error),
    #[error("BeamNG mod data error. `{0}`")]
    BeamNGModDataError(String),
    #[error("jbeam encoding error")]
    JBeamError(#[from] serde_hjson::Error),
    #[error("invalid data `{0}`. `{1}`")]
    InvalidData(String, String),
    #[error("missing data source `{0}`")]
    MissingDataSource(String),
    #[error("missing data section `{0}` from `{1}`")]
    MissingDataSection(String, String),
    #[error("failed to update `{0}` in `{1}`. `{2}`")]
    FailedToUpdate(String, String, String),
    #[error("failed to load `{0}`. `{1}`")]
    FailedToLoad(String, String),
    #[error("failed to write `{0}`. `{1}`")]
    FailedToWrite(String, String),
    #[error("Data validation failure. `{0}`")]
    ValidationError(String),
    #[error("fabrication error: `{0}`")]
    Other(String)
}

enum ACEngineParameterVersion {
    V1
}

impl ACEngineParameterVersion {
    pub const VERSION_1_STRING: &'static str = "v1";

    pub fn as_str(&self) -> &'static str {
        match self {
            ACEngineParameterVersion::V1 => ACEngineParameterVersion::VERSION_1_STRING
        }
    }
}

impl Display for ACEngineParameterVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AssettoCorsaPhysicsLevel {
    BaseGame,
    CspExtendedPhysics
}

impl AssettoCorsaPhysicsLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssettoCorsaPhysicsLevel::BaseGame => { "Base game physics"}
            AssettoCorsaPhysicsLevel::CspExtendedPhysics => { "CSP extended physics" }
        }
    }
}

impl Default for AssettoCorsaPhysicsLevel {
    fn default() -> Self {
        AssettoCorsaPhysicsLevel::BaseGame
    }
}

impl Display for AssettoCorsaPhysicsLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct AssettoCorsaCarSettings {
    pub minimum_physics_level: AssettoCorsaPhysicsLevel,
    pub auto_adjust_clutch: bool
}

impl Default for AssettoCorsaCarSettings {
    fn default() -> AssettoCorsaCarSettings {
        AssettoCorsaCarSettings {
            minimum_physics_level: AssettoCorsaPhysicsLevel::default(),
            auto_adjust_clutch: true
        }
    }
}

pub struct AdditionalAcCarData {
    engine_weight: Option<u32>
}

impl AdditionalAcCarData {
    pub fn new(engine_weight: Option<u32>) -> AdditionalAcCarData {
        AdditionalAcCarData{engine_weight}
    }

    pub fn default() -> AdditionalAcCarData {
        AdditionalAcCarData{engine_weight: None}
    }

    pub fn engine_weight(&self) -> Option<u32> {
        self.engine_weight
    }
}

pub fn normalise_boost_value(boost_value: f64, decimal_places: u32) -> f64 {
    round_float_to(vec![0.0, boost_value].into_iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(), decimal_places)
}

pub fn kw_to_bhp(power_kw: f64) -> f64 {
    power_kw * 1.341
}

#[derive(Debug)]
pub(crate) struct AcEngineParameterCalculatorV1 {
    automation_car_file: CarFile,
    engine_jbeam_data: serde_hjson::Map<String, serde_hjson::Value>,
    engine_sqlite_data: EngineV1
}

impl AcEngineParameterCalculatorV1 {
    pub fn from_crate_engine(crate_eng_path: &Path) -> Result<AcEngineParameterCalculatorV1, FabricationError> {
        use FabricationError::*;
        info!("Creating AC parameter calculator for crate engine {}", crate_eng_path.display());
        let mut file = File::open(crate_eng_path)?;
        let crate_eng = CrateEngine::deserialize_from(&mut file).map_err(|reason|{
            FailedToLoad(crate_eng_path.display().to_string(), reason)
        })?;
        info!("Loaded {} from eng file", crate_eng.name());

        info!("Loading Automation car file");
        let automation_car_file_data = crate_eng.get_automation_car_file_data().clone();
        if automation_car_file_data.is_empty() {
            return Err(MissingDataSource("Automation car file".to_string()))
        }
        let automation_car_file = CarFile::from_bytes(automation_car_file_data).map_err(|reason|{
            FailedToLoad("Automation car file".to_string(), reason)
        })?;

        info!("Loading main engine JBeam file");
        let engine_jbeam_bytes = crate_eng.get_engine_jbeam_data().ok_or_else(||{
            MissingDataSource("Main engine JBeam file".to_string())
        })?;
        Ok(AcEngineParameterCalculatorV1 {
            automation_car_file,
            engine_jbeam_data: serde_hjson::from_slice(engine_jbeam_bytes)?,
            engine_sqlite_data: crate_eng.get_automation_engine_data().clone()
        })
    }

    pub fn from_beam_ng_mod(beam_ng_mod_path: &Path) -> Result<AcEngineParameterCalculatorV1, FabricationError> {
        use FabricationError::*;
        info!("Creating AC parameter calculator for BeamNG mod {}", beam_ng_mod_path.to_path_buf().display());
        let mut mod_data = beam_ng::ModData::from_path(beam_ng_mod_path).map_err(
            BeamNGModDataError
        )?;

        info!("Loading Automation car file");
        let automation_car_file_data = mod_data.get_automation_car_file_data().ok_or_else(||{
            MissingDataSource("Automation car file".to_string())
        })?.clone();
        let automation_car_file = CarFile::from_bytes(automation_car_file_data).map_err(|reason|{
            FailedToLoad("Automation car file".to_string(), reason)
        })?;

        let car_section = automation_car_file.get_section("Car").ok_or_else(||{
            MissingDataSection("'Car'".to_string(), format!("Automation .car file in {}", beam_ng_mod_path.display()))
        })?;
        let variant_info = car_section.get_section("Variant").ok_or_else(||{
            MissingDataSection("'Car.Variant'".to_string(), format!("Automation .car file in {}", beam_ng_mod_path.display()))
        })?;
        let version_num_attr = variant_info.get_attribute("GameVersion").ok_or_else(||{
            MissingDataSection("'Car.Variant.GameVersion'".to_string(), format!("Automation .car file in {}", beam_ng_mod_path.display()))
        })?;
        let version_num = version_num_attr.value.as_num().map_err(|err|{
            FailedToLoad("'Car.Variant.GameVersion'".to_string(), err)
        })?;

        info!("Engine version number: {}", version_num);
        let version = SandboxVersion::from_version_number(version_num as u64);
        info!("Deduced as {}", version);

        let uid_attr = variant_info.get_attribute("UID").ok_or_else(||{
            MissingDataSection("'Car.Variant.UID'".to_string(), format!("Automation .car file in {}", beam_ng_mod_path.display()))
        })?;
        let uid= uid_attr.value.as_str();
        info!("Engine uuid: {}", uid);
        let expected_key = &uid[0..5];
        let engine_jbeam_data = mod_data.get_engine_jbeam_data(Some(expected_key)).map_err(|e|{
            FailedToLoad("Main engine JBeam".to_string(), e)
        })?;

        let engine_sqlite_data = load_engine_by_uuid(uid, version).map_err(|e|{
            FailedToLoad(format!("Sandbox db engine {}", uid), e)
        })?.ok_or_else(||{
            MissingDataSection(format!("engine {}", uid), format!("sandbox db"))
        })?;

        {
            AutomationSandboxCrossChecker::new(&automation_car_file, &engine_sqlite_data).validate().map_err(|err|{
                ValidationError(format!("{}. The engine data saved in Automation doesn't match the BeamNG mod data.\
                                         The mod may be out-of-date; try recreating a mod with the latest engine version", err))
            })?;
        }
        if engine_sqlite_data.rpm_curve.is_empty() {
            return Err(MissingDataSection("curve data".to_string(), "sandbox db".to_string()));
        }
        Ok(AcEngineParameterCalculatorV1 {
            automation_car_file,
            engine_jbeam_data: engine_jbeam_data.clone(),
            engine_sqlite_data
        })
    }

    pub fn engine_weight(&self) -> u32 {
        self.engine_sqlite_data.weight.round() as u32
    }

    pub fn get_engine_jbeam_key(&self) -> String {
        let mut engine_key = String::from("Camso_Engine");
        let test_key = String::from(engine_key.clone() + "_");
        for key in self.engine_jbeam_data.keys() {
            if key.starts_with(&test_key) {
                engine_key = String::from(key);
                break;
            }
        }
        engine_key
    }

    pub fn get_main_engine_jbeam_map(&self) -> Result<&serde_hjson::Map<String, serde_hjson::Value>, FabricationError> {
        let section_name = self.get_engine_jbeam_key();
        let eng_section_object = get_object_from_jbeam_map(
            &self.engine_jbeam_data,
            &section_name,
            "main jbeam engine file"
        )?;
        Ok(get_object_from_jbeam_map(
            eng_section_object,
            "mainEngine",
            "main jbeam engine file"
        )?)
    }

    pub fn inertia(&self) -> Result<f64, FabricationError> {
        let eng_map = self.get_main_engine_jbeam_map()?;
        let inertia_val = eng_map.get("inertia").ok_or_else(||{
            MissingDataSection("inertia".to_string(), "mainEngine".to_string())
        })?;
        match inertia_val {
            serde_hjson::Value::F64(inertia) => Ok(*inertia),
            serde_hjson::Value::String(inertia_str) => {
                let end_trimmed_data = inertia_str.split("*$").collect_vec();
                debug!("End trimmed inertia is {:?}", end_trimmed_data);
                let trimmed_data = end_trimmed_data[0].rsplit("$=").collect_vec();
                debug!("Trimmed inertia is {:?}", trimmed_data);
                match trimmed_data[0].parse::<f64>() {
                    Ok(val) => {
                        debug!("inertia is {}", val);
                        Ok(val)
                    }
                    Err(_) => Err(InvalidData("inertia".to_string(), format!("couldn't parse f64 from {}", inertia_str)))
                }
            }
            _ => Err(InvalidData("inertia".to_string(), "expected to be an f64 or string".to_string()))
        }
    }

    pub fn idle_speed(&self) -> Option<f64> {
        let values = vec![self.engine_sqlite_data.idle_speed, self.engine_sqlite_data.rpm_curve[0]];
        values.into_iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
    }

    pub fn limiter(&self) -> f64 {
        self.engine_sqlite_data.max_rpm
    }

    pub fn basic_fuel_consumption(&self) -> f64 {
        // From https://buildingclub.info/calculator/g-kwh-to-l-h-online-from-gram-kwh-to-liters-per-hour/
        // Fuel Use (l/h) = (Engine Power (kW) * BSFC@Power) / Fuel density kg/m3
        let fuel_use_per_hour = (self.engine_sqlite_data.peak_power * self.engine_sqlite_data.econ) / 750f64;
        let fuel_use_per_sec = fuel_use_per_hour / 3600f64;

        // Assetto Corsa calculation:
        // In one second the consumption is (rpm*gas*CONSUMPTION)/1000
        // fuel_use_per_sec = (engine_data["PeakPowerRPM"] * 1 * C) / 1000
        // fuel_use_per_sec * 1000 =  engine_data["PeakPowerRPM"]*C
        // C = (fuel_use_per_sec * 1000) / engine_data["PeakPowerRPM"]
        // In theory this is being generous as the Econ value is an average over all engine RPMs
        // rather than the consumption at max power but the values still seem to be higher than
        // the values of other AC engines
        // # TODO refine this
        return (fuel_use_per_sec * 1000f64) / self.engine_sqlite_data.peak_power_rpm
    }

    fn get_fuel_use_per_sec_at_rpm(&self, rpm_index: usize) -> f64 {
        // https://en.wikipedia.org/wiki/Brake-specific_fuel_consumption
        // BSFC (g/J) = fuel_consumption (g/s) / power (watts)
        // fuel_consumption (g/s) = BSFC * power (watts)
        // BSFC value stored in econ curve as g/kWh
        // BSFC [g/(kW⋅h)] = BSFC [g/J] × (3.6 × 106)
        (self.engine_sqlite_data.econ_curve[rpm_index] / 3600000_f64) *
            (self.engine_sqlite_data.power_curve[rpm_index] * 1000_f64) // power curve contains kW
    }

    pub fn fuel_flow_consumption(&self, mechanical_efficiency: f64) -> data::engine::FuelConsumptionFlowRate {
        // The lut values should be: rpm, kg/hr
        // The max-flow should be weighted to the upper end of the rev-range as racing is usually done in that range.
        // This is probably enough of a fallback as this will only be used if a lut isn't found and that will be
        // calculated below
        let max_flow_entry_index = (self.engine_sqlite_data.rpm_curve.len() as f64 * 0.70).round() as usize;
        let max_fuel_flow = (self.get_fuel_use_per_sec_at_rpm(max_flow_entry_index) * 3.6).round() as i32;

        let mut max_flow_lut: Vec<(i32, i32)> = Vec::new();
        for (rpm_idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
            max_flow_lut.push((*rpm as i32, (self.get_fuel_use_per_sec_at_rpm(rpm_idx) * 3.6).round() as i32))
        }
        data::engine::FuelConsumptionFlowRate::new(
            0.03,
            (self.idle_speed().unwrap() + 100_f64).round() as i32,
            mechanical_efficiency,
            Some(max_flow_lut),
            max_fuel_flow
        )
    }

    /// Return a vector containing pairs of RPM, Torque (NM)
    pub fn engine_torque_curve(&self) -> Vec<(i32, i32)> {
        let mut out_vec = Vec::new();
        for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
            out_vec.push(((*rpm as i32), self.engine_sqlite_data.torque_curve[idx].round() as i32));
        }
        out_vec
    }

    pub fn peak_torque(&self) -> i32 {
        self.engine_sqlite_data.peak_torque.round() as i32
    }

    /// Return a vector containing pairs of RPM, Power (BHP)
    pub fn engine_bhp_power_curve(&self) -> Vec<(i32, i32)> {
        let mut out_vec = Vec::new();
        for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
            out_vec.push(((*rpm as i32), kw_to_bhp(self.engine_sqlite_data.power_curve[idx]).round() as i32));
        }
        out_vec
    }

    pub fn peak_bhp(&self) -> i32 {
        kw_to_bhp(self.engine_sqlite_data.peak_power).round() as i32
    }

    pub fn naturally_aspirated_wheel_torque_curve(&self, drivetrain_efficiency: f64) -> Vec<(i32, f64)> {
        let mut out_vec: Vec<(i32, f64)> = Vec::new();
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            info!("Writing torque curve for NA engine");
            for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
                let wheel_torque = self.engine_sqlite_data.torque_curve[idx] * drivetrain_efficiency;
                out_vec.push(((*rpm as i32), wheel_torque.round()));
            }
        }
        else {
            info!("Writing torque curve for Turbo engine");
            for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
                let boost_pressure = vec![0.0, self.engine_sqlite_data.boost_curve[idx]].into_iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
                info!("Adjusting {}@{} for boost pressure {}", self.engine_sqlite_data.torque_curve[idx], *rpm as i32, 1f64+boost_pressure);
                let adjusted_value = ((self.engine_sqlite_data.torque_curve[idx] / (1f64+boost_pressure)) * drivetrain_efficiency).round();
                info!("Adjusted value = {}", adjusted_value);
                out_vec.push(((*rpm as i32), adjusted_value));
            }
        }
        // Taper curve down to 0
        let mut rpm_increment = 100;
        {
            let last_item = out_vec.last().unwrap();
            let penultimate_item = out_vec.get(out_vec.len()-2).unwrap();
            rpm_increment = last_item.0 - penultimate_item.0;
        }
        out_vec.push((out_vec.last().unwrap().0 + rpm_increment, out_vec.last().unwrap().1/2f64));
        out_vec.push((out_vec.last().unwrap().0 + rpm_increment, 0f64));
        out_vec
    }

    pub fn get_max_boost_params(&self, decimal_place_precision: u32) -> (i32, f64) {
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            return (0, 0.0);
        }
        let (ref_rpm_idx, max_boost) = self.engine_sqlite_data.boost_curve.iter().enumerate().fold(
            (0usize, normalise_boost_value(self.engine_sqlite_data.boost_curve[0], decimal_place_precision)),
            |(idx_max, max_val), (idx, val)| {
                if normalise_boost_value(*val, 2) > max_val {
                    if normalise_boost_value(*val, 1) > normalise_boost_value(max_val, 1) {
                        return (idx, *val);
                    }
                    return (idx_max, *val);
                }
                (idx_max, max_val)
            }
        );
        (self.engine_sqlite_data.rpm_curve[ref_rpm_idx].round() as i32,
         round_float_to(max_boost, decimal_place_precision))
    }

    pub fn create_turbo(&self) -> Option<engine::Turbo> {
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            return None;
        }
        // todo update this to take into account the boost amount set and ignore any overboost that may skew the turbo section calculation
        let (ref_rpm, max_boost) = self.get_max_boost_params(3);
        let mut t = engine::Turbo::new();
        t.add_section(engine::turbo::TurboSection::new(
            0,
            // TODO work out how to better approximate these
            0.99,
            0.965,
            max_boost,
            max_boost,
            (max_boost * 10_f64).ceil() / 10_f64,
            ref_rpm,
            2.5,
            0)
        );
        Some(t)
    }

    pub fn create_turbo_controller(&self) -> Option<engine::turbo_ctrl::TurboController> {
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            return None;
        }

        let mut lut: Vec<(f64, f64)> = Vec::new();
        for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
            let mut boost_val = 0.0;
            if self.engine_sqlite_data.boost_curve[idx] > boost_val {
                boost_val = round_float_to(self.engine_sqlite_data.boost_curve[idx], 3);
            }
            lut.push((*rpm, boost_val));
        }
        let controller = engine::turbo_ctrl::TurboController::new(
            0,
            engine::turbo_ctrl::ControllerInput::Rpms,
            engine::turbo_ctrl::ControllerCombinator::Add,
            lut,
            0.95,
            10000_f64,
            0_f64
        );
        Some(controller)
    }

    pub fn coast_data(&self) -> Result<engine::CoastCurve, FabricationError> {
        let variant_info = self.automation_car_file.get_section("Car").unwrap().get_section("Variant").unwrap();
        let version_num = variant_info.get_attribute("GameVersion").unwrap().value.as_num().unwrap() as u64;
        if version_num < 2209220000 {
            info!("Using v1 coast calculation for version {}", version_num);
            return self.coast_data_v1();
        } else if version_num >= 2301100000 {
            info!("Using v3 coast calculation for version {}", version_num);
            return self.coast_data_v3();
        }
        info!("Using v2 coast calculation for version {}", version_num);
        return self.coast_data_v2();
    }

    pub fn coast_data_v1(&self) -> Result<engine::CoastCurve, FabricationError> {
        //   The following data is available from the engine.jbeam exported file
        //   The dynamic friction torque on the engine in Nm/s.
        //   This is a friction torque which increases proportional to engine AV (rad/s).
        //   AV = (2pi * RPM) / 60
        //   friction torque = (AV * dynamicFriction) + 2*staticFriction
        //
        //   #### NOTE ####
        //   I'm assuming that all of the sources of friction are being taken into account in the BeamNG parameters used above
        //   this may not be correct.
        let eng_map = self.get_main_engine_jbeam_map()?;
        let dynamic_friction = get_f64_from_jbeam_map(eng_map, "dynamicFriction", "mainEngine")?;
        let static_friction = get_f64_from_jbeam_map(eng_map, "friction", "mainEngine")?;
        let angular_velocity_at_max_rpm = (self.engine_sqlite_data.max_rpm * 2_f64 * std::f64::consts::PI) / 60_f64;
        let friction_torque = (angular_velocity_at_max_rpm * dynamic_friction) + (2_f64 * static_friction);
        Ok(engine::CoastCurve::new_from_coast_ref(self.engine_sqlite_data.max_rpm.round() as i32,
                                                    friction_torque.round() as i32,
                                                    0.0))
    }

    pub fn coast_data_v2(&self) -> Result<engine::CoastCurve, FabricationError> {
        let eng_map = self.get_main_engine_jbeam_map()?;
        let dynamic_friction = get_f64_from_jbeam_map(eng_map, "dynamicFriction", "mainEngine")?;
        // Not sure if this is set correctly in the outputted jbeam files but the best we can work with atm
        let static_friction = get_f64_from_jbeam_map(eng_map, "engineBrakeTorque", "mainEngine")?;
        let angular_velocity_at_max_rpm = (self.engine_sqlite_data.max_rpm / 60_f64) * 2_f64 * std::f64::consts::PI;
        // TODO Assuming the jbeam files are correct I think this should be:
        // friction + dynamicFriction * engineAV + engineBrakeTorque
        // however friction and engineBrakeTorque are the same in the output jbeam files which
        // would result in too high a value. Add only engineBrakeTorque for now
        let friction_torque = (angular_velocity_at_max_rpm * dynamic_friction) + static_friction;
        Ok(engine::CoastCurve::new_from_coast_ref(self.engine_sqlite_data.max_rpm.round() as i32,
                                                  friction_torque.round() as i32,
                                                  0.0))
    }

    pub fn coast_data_v3(&self) -> Result<engine::CoastCurve, FabricationError> {
        //   The following data is available from the engine.jbeam exported file
        //   The dynamic friction torque on the engine in Nm/s.
        //   This is a friction torque which increases proportional to engine AV (rad/s).
        //   AV = (2pi * RPM) / 60
        //   friction torque = (AV * dynamicFriction) + engineBrakeTorque + staticFriction
        //
        //   #### NOTE ####
        //   I'm assuming that all of the sources of friction are being taken into account in the BeamNG parameters used above
        //   this may not be correct.
        let eng_map = self.get_main_engine_jbeam_map()?;
        let dynamic_friction = get_f64_from_jbeam_map(eng_map, "dynamicFriction", "mainEngine")?;
        // Not sure if this is set correctly in the outputted jbeam files but the best we can work with atm
        let static_friction = get_f64_from_jbeam_map(eng_map, "friction", "mainEngine")?;
        let engine_brake_torque = get_f64_from_jbeam_map(eng_map, "engineBrakeTorque", "mainEngine")?;
        let angular_velocity_at_max_rpm = (self.engine_sqlite_data.max_rpm * 2_f64 * std::f64::consts::PI) / 60_f64;
        let friction_torque = (angular_velocity_at_max_rpm * dynamic_friction) + engine_brake_torque + static_friction;
        Ok(engine::CoastCurve::new_from_coast_ref(self.engine_sqlite_data.max_rpm.round() as i32,
                                                  friction_torque.round() as i32,
                                                  0.0))
    }

    pub fn damage(&self) -> engine::Damage {
        let (_, max_boost) = self.get_max_boost_params(2);
        engine::Damage::new(
            (self.limiter()+200_f64).round() as i32,
            1,
            Some(max_boost.ceil()),
            match self.engine_sqlite_data.aspiration.as_str() {
                "Aspiration_Natural" => { Some(0) },
                _ => { Some(4) }
            }
        )
    }

    pub fn create_metadata(&self) -> engine::Metadata {
        let mut m = engine::Metadata::new();
        m.set_version(2);
        m.set_source(engine::metadata::Source::Automation);
        m.set_mass_kg(self.engine_sqlite_data.weight.round() as i64);
        m
    }
}

pub fn update_ac_engine_parameters(ac_car_path: &Path,
                                   calculator: AcEngineParameterCalculatorV1,
                                   settings: AssettoCorsaCarSettings,
                                   additional_car_data: AdditionalAcCarData) -> Result<(), FabricationError> {
    use FabricationError::*;

    info!("Loading car {}", ac_car_path.display());
    let mut car = Car::load_from_path(ac_car_path).map_err(|err|{
        FailedToLoad(ac_car_path.display().to_string(), err.to_string())
    })?;

    let drive_type;
    {
        let drivetrain = Drivetrain::from_car(&mut car).map_err(|e|{
            FailedToLoad(Drivetrain::INI_FILENAME.to_string(), e.to_string())
        })?;
        drive_type = extract_mandatory_section::<data::drivetrain::Traction>(&drivetrain).map_err(|err|{
            MissingDataSection("Traction".to_string(), Drivetrain::INI_FILENAME.to_string())
        })?.drive_type
    }
    info!("Existing car is {} with assumed mechanical efficiency of {}", drive_type, drive_type.mechanical_efficiency());

    let mut mass = None;
    let mut old_limiter = 0;
    let new_limiter = calculator.limiter().round() as i32;

    {
        let mut ini_data = CarIniData::from_car(&mut car).map_err(|err|{
            FailedToLoad(CarIniData::FILENAME.to_string(), err.to_string())
        })?;
        match settings.minimum_physics_level {
            AssettoCorsaPhysicsLevel::BaseGame => {
                info!("Using base game physics");
                ini_data.set_fuel_consumption(calculator.basic_fuel_consumption());
            }
            AssettoCorsaPhysicsLevel::CspExtendedPhysics => {
                info!("Using CSP extended physics");
                {
                    ini_data.set_version(CarVersion::CspExtendedPhysics);
                    ini_data.clear_fuel_consumption();
                }
            }
        }

        if let Some(current_engine_weight) = additional_car_data.engine_weight() {
            if let Some(current_car_mass) = ini_data.total_mass() {
                let new_engine_delta: i32 = calculator.engine_weight() as i32 - current_engine_weight as i32;
                if new_engine_delta < 0 && new_engine_delta.abs() as u32 >= current_car_mass {
                    error!("Invalid existing engine weight ({}). Would result in negative total mass", current_engine_weight);
                } else {
                    let new_mass = (current_car_mass as i32 + new_engine_delta) as u32;
                    info!("Updating total mass to {} based off a provided existing engine weight of {}", new_mass, current_engine_weight);
                    ini_data.set_total_mass(new_mass);
                }
            } else {
                error!("Existing car doesn't have a total mass property")
            }
        }
        info!("Writing car ini files");
        mass = ini_data.total_mass();
        ini_data.write().map_err(|e| {
            FailedToWrite(CarIniData::FILENAME.to_string(), e.to_string())
        })?;
    }

    info!("Clearing existing turbo controllers");
    let res = delete_all_turbo_controllers_from_car(&mut car);
    if let Some(err) = res.err() {
        warn!("Failed to clear turbo controllers. {}", err.to_string());
    }

    {
        let mut engine = Engine::from_car(&mut car).map_err(|err| {
            FailedToLoad(Engine::INI_FILENAME.to_string(), err.to_string())
        })?;
        match settings.minimum_physics_level {
            AssettoCorsaPhysicsLevel::CspExtendedPhysics => {
                update_car_data(&mut engine,
                                &calculator.fuel_flow_consumption(drive_type.mechanical_efficiency()))
                    .map_err(|err| {
                        FailedToUpdate(engine::FuelConsumptionFlowRate::SECTION_NAME.to_string(),
                                       Engine::INI_FILENAME.to_string(),
                                       err.to_string())
                    })?
            }
            _ => {}
        }

        let mut engine_data = extract_mandatory_section::<data::engine::EngineData>(&engine).map_err(|err|{
            FailedToLoad(EngineData::SECTION_NAME.to_string(), err.to_string())
        })?;

        match calculator.inertia() {
            Ok(inertia) => engine_data.inertia = inertia,
            Err(e) => warn!("Failed to calculate new inertia value. {}. existing value will be used", e.to_string())
        };

        old_limiter = engine_data.limiter;
        engine_data.limiter = new_limiter;
        engine_data.minimum = match calculator.idle_speed() {
            Some(idle) => idle.round() as i32,
            None => {
                warn!("Failed to calculate idle rpm. Using 500 as value");
                500
            }
        };
        update_car_data(&mut engine, &engine_data).map_err(|err|{
            FailedToUpdate(EngineData::SECTION_NAME.to_string(),
                           Engine::INI_FILENAME.to_string(),
                           err.to_string())
        })?;
        update_car_data(&mut engine, &calculator.damage()).map_err(|err|{
            FailedToUpdate(Damage::SECTION_NAME.to_string(),
                           Engine::INI_FILENAME.to_string(),
                           err.to_string())
        })?;

        let coast_data = calculator.coast_data()?;
        update_car_data(&mut engine, &coast_data).map_err(|err|{
            FailedToUpdate(CoastCurve::COAST_REF_SECTION_NAME.to_string(),
                           Engine::INI_FILENAME.to_string(),
                           err.to_string())
        })?;

        let mut power_curve = extract_mandatory_section::<engine::PowerCurve>(&engine).map_err(|err|{
            MissingDataSection(PowerCurve::SECTION_NAME.to_string(),
                               Engine::INI_FILENAME.to_string())
        })?;
        power_curve.update(calculator.naturally_aspirated_wheel_torque_curve(drive_type.mechanical_efficiency()));
        update_car_data(&mut engine, &power_curve).map_err(|err|{
            FailedToUpdate(PowerCurve::SECTION_NAME.to_string(),
                           Engine::INI_FILENAME.to_string(),
                           err.to_string())
        })?;

        match calculator.create_turbo() {
            None => {
                info!("The new engine doesn't have a turbo");
                let old_turbo = extract_optional_section::<engine::Turbo>(&engine).map_err(|e|
                    FailedToLoad(format!("Turbo from {}", Engine::INI_FILENAME), e.to_string())
                )?;
                if let Some(mut turbo) = old_turbo {
                    info!("Removing old engine turbo parameters");
                    turbo.clear_sections();
                    turbo.clear_bov_threshold();
                    update_car_data(&mut engine, &turbo).map_err(|err|{
                        FailedToUpdate("TURBO".to_string(),
                                       Engine::INI_FILENAME.to_string(),
                                       err.to_string())
                    })?;
                }
            }
            Some(new_turbo) => {
                info!("The new engine has a turbo");
                update_car_data(&mut engine, &new_turbo).map_err(|err|{
                    FailedToUpdate("TURBO".to_string(),
                                   Engine::INI_FILENAME.to_string(),
                                   err.to_string())
                })?;
            }
        }

        info!("Writing engine ini files");
        engine.write().map_err(|err| {
            FailedToWrite(Engine::INI_FILENAME.to_string(), err.to_string())
        })?;
    }

    if let Some(turbo_ctrl) = calculator.create_turbo_controller() {
        info!("Writing turbo controller with index 0");
        let mut controller_file = engine::TurboControllerFile::new(&mut car, 0);
        update_car_data(&mut controller_file, &turbo_ctrl).map_err(|err|{
            FailedToUpdate("boost curve".to_string(),
                           controller_file.filename(),
                           err.to_string())
        })?;
        controller_file.write().map_err(|err| {
            FailedToWrite(controller_file.filename(), err.to_string())
        })?;
    }

    {
        info!("Updating drivetrain ini files");
        match Drivetrain::from_car(&mut car) {
            Ok(mut drivetrain) => {
                match extract_mandatory_section::<data::drivetrain::AutoShifter>(&drivetrain) {
                    Ok(mut autoshifter) => {
                        let limiter = calculator.limiter().round() as i32;
                        autoshifter.up = (limiter / 100) * 97;
                        autoshifter.down = (limiter / 100) * 70;
                        if update_car_data(&mut drivetrain, &autoshifter).is_err() {
                            error!("Failed to update drivetrain autoshifer");
                        }
                    }
                    Err(err) => {
                        error!("Failed to update drivetrain autoshifer. {}", err.to_string());
                    }
                }

                if settings.auto_adjust_clutch {
                    match extract_mandatory_section::<data::drivetrain::Clutch>(&drivetrain) {
                        Ok(mut clutch) => {
                            let peak_torque = calculator.peak_torque();
                            if peak_torque > clutch.max_torque {
                                clutch.max_torque = round_up_to_nearest_multiple(peak_torque+30, 50)
                            }
                            if update_car_data(&mut drivetrain, &clutch).is_err() {
                                error!("Failed to update drivetrain with clutch data");
                            }
                        }
                        Err(err) => {
                            error!("Failed to update clutch MAX_TORQUE. {}", err.to_string());
                        }
                    }
                }

                info!("Writing drivetrain ini files");
                match drivetrain.write() {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Failed to write drivetrain.ini. {}", err.to_string());
                    }
                }
            }
            Err(err) => {
                error!("Failed to load drivetrain. {}", err.to_string());
            }
        };
    };

    {
        info!("Updating ai ini files");
        match Ai::from_car(&mut car) {
            Ok(ai_option) => {
                if let Some(mut ai) = ai_option {
                    match extract_mandatory_section::<data::ai::Gears>(&ai) {
                        Ok(mut gears) => {
                            let limiter = calculator.limiter().round() as i32;
                            gears.up = (limiter / 100) * 97;
                            gears.down = (limiter / 100) * 70;
                            if update_car_data(&mut ai, &gears).is_err() {
                                error!("Failed to update ai shift points");
                            }
                            match ai.write() {
                                Err(err) => {
                                    error!("Failed to write {}. {}", data::ai::INI_FILENAME, err.to_string());
                                }
                                _ => {}
                            }
                        }
                        Err(_) => {}
                    }
                } else {
                    error!("Failed to load ai data");
                }
            }
            Err(err) => {
                error!("Failed to load ai data. {}", err.to_string());
            }
        }
    }

    match DigitalInstruments::from_car(&mut car) {
        Ok(opt) => {
            if let Some(mut digital_instruments) = opt {
                info!("Updating digital instruments files");
                match ShiftLights::load_from_parent(&digital_instruments) {
                    Ok(opt) => {
                        if let Some(mut shift_lights) = opt {
                            shift_lights.update_limiter(old_limiter as u32, new_limiter as u32);
                            match update_car_data(&mut digital_instruments, &shift_lights) {
                                Err(err) => {
                                    warn!("Failed to shift lights in {}. {}",
                                          DigitalInstruments::INI_FILENAME,
                                          err.to_string())
                                }
                                _ => {}
                            }
                            match digital_instruments.write() {
                                Err(err) => {
                                    warn!("Failed to write digital_instruments.ini. {}", err.to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(err) => {
                        warn!("Failed to shift lights in {}. {}", DigitalInstruments::INI_FILENAME, err.to_string())
                    }
                }
            }
        }
        Err(err) => { warn!("Failed to update {}. {}", DigitalInstruments::INI_FILENAME, err.to_string())}
    }

    {
        info!("Updating ui components");
        let blank = String::from("---");
        match CarUiData::from_car(&mut car) {
            Ok(mut ui_data) => {
                let _ = ui_data.ui_info.update_power_curve(calculator.engine_bhp_power_curve());
                let _ = ui_data.ui_info.update_torque_curve(calculator.engine_torque_curve());
                let _ = ui_data.ui_info.update_spec("bhp", format!("{}bhp", calculator.peak_bhp()));
                let _ = ui_data.ui_info.update_spec("torque", format!("{}Nm", calculator.peak_torque()));
                if let Some(mass_val) = mass {
                    let _ = ui_data.ui_info.update_spec("weight", format!("{}kg", mass_val));
                    let _ = ui_data.ui_info.update_spec("pwratio", format!("{}kg/hp", round_float_to(mass_val as f64 / (calculator.peak_bhp() as f64), 2)));
                } else {
                    let _ = ui_data.ui_info.update_spec("weight", blank.clone());
                    let _ = ui_data.ui_info.update_spec("pwratio", blank.clone());
                }
                let _ = ui_data.ui_info.update_spec("acceleration", blank.clone());
                let _ = ui_data.ui_info.update_spec("range", blank.clone());
                let _ = ui_data.ui_info.update_spec("topspeed", blank);

                info!("Writing car ui files");
                ui_data.ui_info.write().unwrap_or_else(|e|{
                    error!("Failed to write ui files. {}", e.to_string());
                });
            }
            Err(e) => {
                error!("Failed to load ui files. {}", e.to_string());
            }
        }
    }
    Ok(())
}

pub fn swap_automation_engine_into_ac_car(beam_ng_mod_path: &Path,
                                          ac_car_path: &Path,
                                          settings: AssettoCorsaCarSettings,
                                          additional_car_data: AdditionalAcCarData) -> Result<(), FabricationError> {
    update_ac_engine_parameters(ac_car_path,
                                AcEngineParameterCalculatorV1::from_beam_ng_mod(beam_ng_mod_path)?,
                                settings, additional_car_data
    )
}

pub fn swap_crate_engine_into_ac_car(crate_engine_path: &Path,
                                     ac_car_path: &Path,
                                     settings: AssettoCorsaCarSettings,
                                     additional_car_data: AdditionalAcCarData) -> Result<(), FabricationError> {
    update_ac_engine_parameters(ac_car_path,
                                AcEngineParameterCalculatorV1::from_crate_engine(crate_engine_path)?,
                                settings, additional_car_data
    )
}

fn get_object_from_jbeam_map<'a>(map: &'a serde_hjson::Map<String, serde_hjson::Value>,
                                 key: &str,
                                 file_identifier: &str)
                                 -> Result<&'a serde_hjson::Map<String, serde_hjson::Value>, FabricationError>
{
    let section_val = map.get(key).ok_or_else(||{
        MissingDataSection(key.to_string(), file_identifier.to_string())
    })?;
    Ok(section_val.as_object().ok_or_else(||{
        InvalidData(format!("{} in {}.", key, file_identifier),
                    "expected to be an object".to_string())
    })?)
}

fn get_f64_from_jbeam_map<'a>(map: &'a serde_hjson::Map<String, serde_hjson::Value>,
                              key: &str,
                              file_identifier: &str)
                              -> Result<f64, FabricationError>
{
    Ok(map.get(key).ok_or_else(||{
        MissingDataSection(key.to_string(), file_identifier.to_string())
    })?.as_f64().ok_or_else(||{
        InvalidData(key.to_string(), "expected to be an f64".to_string())
    })?)
}

#[cfg(test)]
mod tests {
    use std::path::{PathBuf};
    
    use crate::{automation, beam_ng};
    use crate::beam_ng::get_mod_list;
    use crate::fabricator::{AcEngineParameterCalculatorV1};

    #[test]
    fn load_mods() -> Result<(), String> {
        let mods = get_mod_list();
        let calculator = AcEngineParameterCalculatorV1::from_beam_ng_mod(mods[0].as_path())?;
        std::fs::write("inertia.txt",format!("{}", calculator.inertia().unwrap()));
        std::fs::write("idle.txt",format!("{}", calculator.idle_speed().unwrap()));
        std::fs::write("limiter.txt",format!("{}", calculator.limiter()));
        std::fs::write("fuel_cons.txt",format!("{}", calculator.basic_fuel_consumption()));
        std::fs::write("torque_curve.txt",format!("{:?}", calculator.naturally_aspirated_wheel_torque_curve(0.85)));
        std::fs::write("turbo_ctrl.txt",format!("{:?}", calculator.create_turbo_controller().unwrap()));
        std::fs::write("turbo.txt",format!("{:?}", calculator.create_turbo().unwrap()));
        std::fs::write("coast.txt",format!("{:?}", calculator.coast_data().unwrap()));
        std::fs::write("metadata.txt",format!("{:?}", calculator.create_metadata()));
        std::fs::write("fuel_flow.txt", format!("{:?}", calculator.fuel_flow_consumption(0.75))).unwrap();
        std::fs::write("damage.txt", format!("{:?}", calculator.damage())).unwrap();
        Ok(())
    }

    // #[test]
    // fn clone_and_swap_test() -> Result<(), String> {
    //     let new_car_path = create_new_car_spec("zephyr_za401", "test", true).unwrap();
    //     let mods = get_mod_list();
    //     swap_automation_engine_into_ac_car(mods[0].as_path(),
    //                                        new_car_path.as_path(),
    //                                        AssettoCorsaCarSettings::default(),
    //                                        AdditionalAcCarData::default())
    // }

    #[test]
    fn dump_automation_car_file() -> Result<(), String> {
        //let path = PathBuf::from("/home/josykes/.steam/debian-installation/steamapps/compatdata/293760/pfx/drive_c/users/steamuser/AppData/Local/BeamNG.drive/mods/");
        let path = PathBuf::from("C:/Users/zephy/AppData/Local/BeamNG.drive/mods");
        // C:\Users\zephy\AppData\Local\BeamNG.drive\mods\dae1.zip
        let mod_data = beam_ng::ModData::from_path(&path.join("dawnv6.zip"))?;
        let automation_car_file = automation::car::CarFile::from_bytes( mod_data.get_automation_car_file_data().ok_or("Couldn't find car data")?.clone())?;
        println!("{:#?}", automation_car_file);
        if let Some(version) = automation_car_file.get_section("Car").unwrap().get_section("Variant").unwrap().get_attribute("GameVersion") {
            println!("{}", version);
        }
        //fs::write(Path::new("car_temp.toml"), format!("{}", automation_car_file));
        Ok(())
    }
}
