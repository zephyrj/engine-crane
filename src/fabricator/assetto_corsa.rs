/*
 * Copyright (c):
 * 2024 zephyrj
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
use std::path::{Path, PathBuf};
use itertools::Itertools;
use tracing::{debug, info};
use assetto_corsa::car::data;
use assetto_corsa::car::data::engine;
use automation::car::CarFile;
use automation::sandbox::{EngineV1, load_engine_by_uuid, SandboxVersion};
use utils::units::kw_to_bhp;
use automation::validation::AutomationSandboxCrossChecker;
use crate_engine::{CrateEngine, CrateEngineData};
use crate_engine::beam_ng_mod;
use crate::fabricator::{FabricationError};
use crate::fabricator::FabricationError::{BeamNGModDataError, FailedToLoad, InvalidData, MissingDataSection, MissingDataSource, ValidationError};
use crate::utils::numeric::{round_float_to};

pub enum EngineParameterCalculator {
    V1(EngineParameterCalculatorV1)
}

impl EngineParameterCalculator {
    pub fn from_crate_engine(crate_eng_path: &Path) -> Result<EngineParameterCalculator, FabricationError> {
        use crate::fabricator::FabricationError::*;
        info!("Creating AC parameter calculator for crate engine {}", crate_eng_path.display());
        let mut file = File::open(crate_eng_path)?;
        let crate_eng = CrateEngine::deserialize_from(&mut file).map_err(|reason|{
            FailedToLoad(crate_eng_path.display().to_string(), reason)
        })?;
        info!("Loaded {} from eng file", crate_eng.name());

        match crate_eng.data() {
            CrateEngineData::BeamNGMod(data_version) => match data_version {
                beam_ng_mod::Data::V1(data) => {
                    info!("Loading Automation car file");
                    let automation_car_file_data = data.car_file_data().clone();
                    if automation_car_file_data.is_empty() {
                        return Err(MissingDataSource("Automation car file".to_string()))
                    }
                    let automation_car_file = CarFile::from_bytes(automation_car_file_data).map_err(|reason|{
                        FailedToLoad("Automation car file".to_string(), reason)
                    })?;

                    info!("Loading main engine JBeam file");
                    let engine_jbeam_bytes = data.main_engine_jbeam_data().ok_or_else(||{
                        MissingDataSource("Main engine JBeam file".to_string())
                    })?;
                    Ok(EngineParameterCalculator::V1(EngineParameterCalculatorV1 {
                        automation_car_file,
                        engine_jbeam_data: serde_hjson::from_slice(engine_jbeam_bytes)?,
                        engine_sqlite_data: data.automation_data().clone()
                    }))
                }
            }
            _ => {
                Err(Other("Can only currently create EngineParameterCalculator from BeamNG mods".to_string()))
            }
        }
    }

    pub fn from_beam_ng_mod(beam_ng_mod_path: &Path) -> Result<EngineParameterCalculator, FabricationError> {
        use crate::fabricator::FabricationError::*;
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
        Ok(EngineParameterCalculator::V1(EngineParameterCalculatorV1 {
            automation_car_file,
            engine_jbeam_data: engine_jbeam_data.clone(),
            engine_sqlite_data
        }))
    }

    pub fn engine_weight(&self) -> u32 {
        match self {
            EngineParameterCalculator::V1(c) => c.engine_weight()
        }
    }

    pub fn inertia(&self) -> Result<f64, FabricationError> {
        match self {
            EngineParameterCalculator::V1(c) => c.inertia()
        }
    }

    pub fn idle_speed(&self) -> Option<f64> {
        match self {
            EngineParameterCalculator::V1(c) => c.idle_speed()
        }
    }

    pub fn limiter(&self) -> f64 {
        match self {
            EngineParameterCalculator::V1(c) => c.limiter()
        }
    }

    pub fn basic_fuel_consumption(&self) -> f64 {
        match self {
            EngineParameterCalculator::V1(c) => c.basic_fuel_consumption()
        }
    }

    pub fn fuel_flow_consumption(&self, mechanical_efficiency: f64) -> data::engine::FuelConsumptionFlowRate {
        match self {
            EngineParameterCalculator::V1(c) => {
                c.fuel_flow_consumption(mechanical_efficiency)
            }
        }
    }

    pub fn engine_torque_curve(&self) -> Vec<(i32, i32)> {
        match self {
            EngineParameterCalculator::V1(c) => c.engine_torque_curve()
        }
    }

    pub fn peak_torque(&self) -> i32 {
        match self {
            EngineParameterCalculator::V1(c) => c.peak_torque()
        }
    }

    pub fn engine_bhp_power_curve(&self) -> Vec<(i32, i32)> {
        match self {
            EngineParameterCalculator::V1(c) => c.engine_bhp_power_curve()
        }
    }

    pub fn peak_bhp(&self) -> i32 {
        match self {
            EngineParameterCalculator::V1(c) => c.peak_bhp()
        }
    }

    pub fn naturally_aspirated_wheel_torque_curve(&self, drivetrain_efficiency: f64) -> Vec<(i32, f64)> {
        match self {
            EngineParameterCalculator::V1(c) => {
                c.naturally_aspirated_wheel_torque_curve(drivetrain_efficiency)
            }
        }
    }

    pub fn get_max_boost_params(&self, decimal_place_precision: u32) -> (i32, f64) {
        match self {
            EngineParameterCalculator::V1(c) => {
                c.get_max_boost_params(decimal_place_precision)
            }
        }
    }

    pub fn create_turbo(&self) -> Option<engine::Turbo> {
        match self {
            EngineParameterCalculator::V1(c) => c.create_turbo()
        }
    }

    pub fn create_turbo_controller(&self) -> Option<engine::turbo_ctrl::TurboController> {
        match self {
            EngineParameterCalculator::V1(c) => c.create_turbo_controller()
        }
    }

    pub fn coast_data(&self) -> Result<engine::CoastCurve, FabricationError> {
        match self {
            EngineParameterCalculator::V1(c) => c.coast_data()
        }
    }

    pub fn damage(&self) -> engine::Damage {
        match self {
            EngineParameterCalculator::V1(c) => c.damage()
        }
    }

    pub fn create_metadata(&self) -> engine::Metadata {
        match self {
            EngineParameterCalculator::V1(c) => c.create_metadata()
        }
    }
}

#[derive(Debug)]
pub(crate) struct EngineParameterCalculatorV1 {
    automation_car_file: CarFile,
    engine_jbeam_data: serde_hjson::Map<String, serde_hjson::Value>,
    engine_sqlite_data: EngineV1
}

impl EngineParameterCalculatorV1 {
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

pub fn normalise_boost_value(boost_value: f64, decimal_places: u32) -> f64 {
    round_float_to(vec![0.0, boost_value].into_iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(), decimal_places)
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
