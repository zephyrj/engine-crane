/*
 * Copyright (c):
 * 2022 zephyrj
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

use std::fmt::{Display, Formatter};
use std::num::ParseFloatError;
use std::path::Path;
use itertools::Itertools;
use sha2::{Sha256, Digest};
use tracing::{debug, error, info, warn};
use crate::{automation, beam_ng};
use crate::assetto_corsa::Car;
use crate::assetto_corsa::car::data;
use crate::assetto_corsa::car::data::CarIniData;
use crate::assetto_corsa::car::data::car_ini_data::CarVersion;
use crate::assetto_corsa::car::data::digital_instruments::DigitalInstruments;
use crate::assetto_corsa::car::data::digital_instruments::shift_lights::ShiftLights;
use crate::assetto_corsa::car::ui::CarUiData;
use crate::assetto_corsa::car::data::Drivetrain;
use crate::assetto_corsa::car::data::Engine;
use crate::assetto_corsa::car::data::engine;
use crate::assetto_corsa::car::data::engine::turbo_ctrl::delete_all_turbo_controllers_from_car;
use crate::assetto_corsa::car::lut_utils::{InlineLut, LutType};
use crate::assetto_corsa::car::structs::LutProperty;
use crate::utils::round_float_to;

use crate::assetto_corsa::traits::{extract_mandatory_section, extract_optional_section, OptionalDataSection, update_car_data};
use crate::automation::car::{Attribute, AttributeValue, CarFile, Section};
use crate::automation::sandbox::{EngineV1, load_engine_by_uuid, SandboxVersion};

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
}

impl AssettoCorsaCarSettings {
    pub fn from_physics_level(level: AssettoCorsaPhysicsLevel) -> AssettoCorsaCarSettings {
        AssettoCorsaCarSettings {
            minimum_physics_level: level,
        }
    }
}

impl Default for AssettoCorsaCarSettings {
    fn default() -> AssettoCorsaCarSettings {
        AssettoCorsaCarSettings{
            ..Default::default()
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
struct AcEngineParameterCalculatorV1 {
    automation_car_file: CarFile,
    engine_jbeam_data: serde_hjson::Map<String, serde_hjson::Value>,
    engine_sqlite_data: EngineV1
}

impl AcEngineParameterCalculatorV1 {
    pub fn from_beam_ng_mod(beam_ng_mod_path: &Path) -> Result<AcEngineParameterCalculatorV1, String> {
        info!("Creating AC parameter calculator for {}", beam_ng_mod_path.to_path_buf().display());
        let mut mod_data = beam_ng::ModData::from_path(beam_ng_mod_path)?;
        let automation_car_file = automation::car::CarFile::from_bytes( mod_data.get_automation_car_file_data().ok_or(format!("Couldn't get .car data for {}", beam_ng_mod_path.display()))?.clone())?;
        let variant_info = automation_car_file.get_section("Car").unwrap().get_section("Variant").unwrap();
        let version_num = variant_info.get_attribute("GameVersion").unwrap().value.as_num().unwrap();

        info!("Engine version number: {}", version_num);
        let version = SandboxVersion::from_version_number(version_num as u64);
        info!("Deduced as {}", version);

        let engine_jbeam_data = mod_data.get_engine_jbeam_data().map_err(|e|{
            format!("Couldn't load engine data from {}. {}", beam_ng_mod_path.display(), &e)
        })?;
        let uid = variant_info.get_attribute("UID").unwrap().value.as_str();

        info!("Engine uuid: {}", uid);
        let engine_sqlite_data = match load_engine_by_uuid(uid, version)? {
            None => {
                return Err(format!("No engine found with uuid {}", uid));
            }
            Some(eng) => { eng }
        };
        {
            EngineDataValidator::new(&automation_car_file, &engine_sqlite_data).validate().map_err(|err|{
                format!("{}. The BeamNG mod may be out-of-date; try recreating a mod with the latest engine version", err)
            })?;
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

    pub fn inertia(&self) -> Option<f64> {
        let engine_key= self.get_engine_jbeam_key();
        info!("JBeam engine key is {}", engine_key);
        let eng_map = self.engine_jbeam_data.get(&engine_key)?.as_object()?.get("mainEngine")?.as_object()?;
        eng_map.get("inertia")?.as_f64().or_else(||{
            if let Some(str_data) = eng_map.get("inertia")?.as_str() {
                let end_trimmed_data = str_data.split("*$").collect_vec();
                debug!("End trimmed inertia is {:?}", end_trimmed_data);
                let trimmed_data = end_trimmed_data[0].rsplit("$=").collect_vec();
                debug!("Trimmed inertia is {:?}", trimmed_data);
                return match trimmed_data[0].parse::<f64>() {
                    Ok(val) => {
                        debug!("inertia is {}", val);
                        Some(val)
                    }
                    Err(_) => { None }
                }
            }
            None
        })
    }

    pub fn idle_speed(&self) -> Option<f64> {
        let values = vec![self.engine_sqlite_data.idle_speed, self.engine_sqlite_data.rpm_curve[0]];
        values.into_iter().max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    pub fn limiter(&self) -> f64 {
        self.engine_sqlite_data.max_rpm
    }

    pub fn basic_fuel_consumption(&self) -> Option<f64> {
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
        return Some((fuel_use_per_sec * 1000f64) / self.engine_sqlite_data.peak_power_rpm)
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

    pub fn naturally_aspirated_wheel_torque_curve(&self, drivetrain_efficiency: f64) -> Vec<(i32, i32)> {
        let mut out_vec = Vec::new();
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            info!("Writing torque curve for NA engine");
            for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
                let wheel_torque = self.engine_sqlite_data.torque_curve[idx] * drivetrain_efficiency;
                out_vec.push(((*rpm as i32), wheel_torque.round() as i32));
            }
        }
        else {
            info!("Writing torque curve for Turbo engine");
            for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
                let boost_pressure = vec![0.0, self.engine_sqlite_data.boost_curve[idx]].into_iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
                info!("Adjusting {}@{} for boost pressure {}", self.engine_sqlite_data.torque_curve[idx], *rpm as i32, 1f64+boost_pressure);
                let adjusted_value = ((self.engine_sqlite_data.torque_curve[idx] / (1f64+boost_pressure)) * drivetrain_efficiency).round() as i32;
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
        out_vec.push((out_vec.last().unwrap().0 + rpm_increment, out_vec.last().unwrap().1/2));
        out_vec.push((out_vec.last().unwrap().0 + rpm_increment, 0));
        out_vec
    }

    pub fn get_max_boost_params(&self, decimal_place_precision: u32) -> (i32, f64) {
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            return (0, 0.0);
        }
        let (ref_rpm_idx, max_boost) = self.engine_sqlite_data.boost_curve.iter().enumerate().fold(
            (0 as usize, normalise_boost_value(self.engine_sqlite_data.boost_curve[0], decimal_place_precision)),
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

    pub fn coast_data(&self) -> Option<engine::CoastCurve> {
        let variant_info = self.automation_car_file.get_section("Car").unwrap().get_section("Variant").unwrap();
        let version_num = variant_info.get_attribute("GameVersion").unwrap().value.as_num().unwrap() as u64;
        if version_num < 2209220000 {
            info!("Using v1 coast calculation for version {}", version_num);
            return self.coast_data_v1();
        }
        info!("Using v2 coast calculation for version {}", version_num);
        return self.coast_data_v2();
    }

    pub fn coast_data_v1(&self) -> Option<engine::CoastCurve> {
        //   The following data is available from the engine.jbeam exported file
        //   The dynamic friction torque on the engine in Nm/s.
        //   This is a friction torque which increases proportional to engine AV (rad/s).
        //   AV = (2pi * RPM) / 60
        //   friction torque = (AV * dynamicFriction) + 2*staticFriction
        //
        //   #### NOTE ####
        //   I'm assuming that all of the sources of friction are being taken into account in the BeamNG parameters used above
        //   this may not be correct.
        let eng_map = self.engine_jbeam_data.get(&self.get_engine_jbeam_key())?.as_object()?.get("mainEngine")?.as_object()?;
        let dynamic_friction = eng_map.get("dynamicFriction")?.as_f64().unwrap();
        let static_friction = eng_map.get("friction")?.as_f64().unwrap();
        let angular_velocity_at_max_rpm = (self.engine_sqlite_data.max_rpm * 2_f64 * std::f64::consts::PI) / 60_f64;
        let friction_torque = (angular_velocity_at_max_rpm * dynamic_friction) + (2_f64 * static_friction);
        Some(engine::CoastCurve::new_from_coast_ref(self.engine_sqlite_data.max_rpm.round() as i32,
                                                    friction_torque.round() as i32,
                                                    0.0))
    }

    pub fn coast_data_v2(&self) -> Option<engine::CoastCurve> {
        let eng_map = self.engine_jbeam_data.get(&self.get_engine_jbeam_key())?.as_object()?.get("mainEngine")?.as_object()?;
        let dynamic_friction = eng_map.get("dynamicFriction")?.as_f64().unwrap();
        // Not sure if this is set correctly in the outputted jbeam files but the best we can work with atm
        let static_friction = eng_map.get("engineBrakeTorque")?.as_f64().unwrap();
        let angular_velocity_at_max_rpm = (self.engine_sqlite_data.max_rpm / 60_f64) * 2_f64 * std::f64::consts::PI;
        // TODO Assuming the jbeam files are correct I think this should be:
        // friction + dynamicFriction * engineAV + engineBrakeTorque
        // however friction and engineBrakeTorque are the same in the output jbeam files which
        // would result in too high a value. Add only engineBrakeTorque for now
        let friction_torque = (angular_velocity_at_max_rpm * dynamic_friction) + static_friction;
        Some(engine::CoastCurve::new_from_coast_ref(self.engine_sqlite_data.max_rpm.round() as i32,
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

pub fn swap_automation_engine_into_ac_car(beam_ng_mod_path: &Path,
                                          ac_car_path: &Path,
                                          settings: AssettoCorsaCarSettings,
                                          additional_car_data: AdditionalAcCarData) -> Result<(), String> {
    let calculator = AcEngineParameterCalculatorV1::from_beam_ng_mod(beam_ng_mod_path)?;
    info!("Loading car {}", ac_car_path.display());
    let mut car = Car::load_from_path(ac_car_path).map_err(|err|{
        let err_str = format!("Failed to load {}. {}", ac_car_path.display(), err.to_string());
        error!("{}", &err_str);
        err_str
    })?;
    let drive_type = match Drivetrain::from_car(&mut car) {
        Ok(drivetrain) => {
            extract_mandatory_section::<data::drivetrain::Traction>(&drivetrain).map_err(|err|{
                format!("Failed to load drivetrain. {}", err.to_string())
            })?.drive_type
        },
        Err(err) => {
            return Err(format!("Failed to load drivetrain. {}", err.to_string()));
        }
    };
    info!("Existing car is {} with assumed mechanical efficiency of {}", drive_type, drive_type.mechanical_efficiency());

    let mut mass = 0;
    let mut old_limiter = 0;
    let new_limiter = calculator.limiter().round() as i32;

    {
        let mut ini_data = CarIniData::from_car(&mut car).unwrap();
        match settings.minimum_physics_level {
            AssettoCorsaPhysicsLevel::BaseGame => {
                info!("Using base game physics");
                ini_data.set_fuel_consumption(calculator.basic_fuel_consumption().unwrap());
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
                }
                let new_mass = (current_car_mass as i32 + new_engine_delta) as u32;
                info!("Updating total mass to {} based off a provided existing engine weight of {}", new_mass, current_engine_weight);
                ini_data.set_total_mass(new_mass);
            } else {
                error!("Existing car doesn't have a total mass property")
            }
        }
        info!("Writing car ini files");
        mass = ini_data.total_mass().unwrap();
        ini_data.write().unwrap();
    }

    info!("Clearing existing turbo controllers");
    let res = delete_all_turbo_controllers_from_car(&mut car);
    if let Some(err) = res.err() {
        warn!("Failed to clear fuel turbo controllers. {}", err.to_string());
    }

    {
        let mut engine = Engine::from_car(&mut car).map_err(|err| {
            format!("Failed to load engine. {}", err.to_string())
        })?;
        match settings.minimum_physics_level {
            AssettoCorsaPhysicsLevel::CspExtendedPhysics => {
                update_car_data(&mut engine,
                                &calculator.fuel_flow_consumption(drive_type.mechanical_efficiency()))
                    .map_err(|err| {
                        error!("Failed to update fuel consumption. {}", err.to_string());
                        err.to_string()
                    })?
            }
            _ => {}
        }

        let mut engine_data = extract_mandatory_section::<data::engine::EngineData>(&engine).unwrap();
        let inertia = calculator.inertia().unwrap();
        engine_data.inertia = inertia;
        old_limiter = engine_data.limiter;
        engine_data.limiter = new_limiter;
        engine_data.minimum = calculator.idle_speed().unwrap().round() as i32;
        update_car_data(&mut engine, &engine_data).unwrap();
        update_car_data(&mut engine, &calculator.damage()).unwrap();

        update_car_data(&mut engine, &calculator.coast_data().unwrap()).unwrap();

        let mut power_curve = extract_mandatory_section::<engine::PowerCurve>(&engine).unwrap();
        power_curve.update(calculator.naturally_aspirated_wheel_torque_curve(drive_type.mechanical_efficiency())).unwrap();
        update_car_data(&mut engine, &power_curve).unwrap();

        match calculator.create_turbo() {
            None => {
                info!("The new engine doesn't have a turbo");
                if let Some(mut old_turbo) = extract_optional_section::<engine::Turbo>(&engine).unwrap() {
                    info!("Removing old engine turbo parameters");
                    old_turbo.clear_sections();
                    old_turbo.clear_bov_threshold();
                    update_car_data(&mut engine,&old_turbo).unwrap();
                }
            }
            Some(new_turbo) => {
                info!("The new engine has a turbo");
                update_car_data(&mut engine,&new_turbo).unwrap();
            }
        }

        info!("Writing engine ini files");
        engine.write().map_err(|err| {
            error!("{}", err.to_string());
            format!("Swap failed. {}", err.to_string())
        })?;
    }

    if let Some(turbo_ctrl) = calculator.create_turbo_controller() {
        info!("Writing turbo controller with index 0");
        let mut controller_file = engine::TurboControllerFile::new(&mut car, 0);
        update_car_data(&mut controller_file, &turbo_ctrl).unwrap();
        controller_file.write().map_err(|err| {
            error!("{}", err.to_string());
            format!("Swap failed. {}", err.to_string())
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
        let mut ui_data = CarUiData::from_car(&mut car).unwrap();
        ui_data.ui_info.update_power_curve(calculator.engine_bhp_power_curve());
        ui_data.ui_info.update_torque_curve(calculator.engine_torque_curve());
        ui_data.ui_info.update_spec("bhp", format!("{}bhp", calculator.peak_bhp()));
        ui_data.ui_info.update_spec("torque", format!("{}Nm", calculator.peak_torque()));
        ui_data.ui_info.update_spec("weight", format!("{}kg", mass));
        ui_data.ui_info.update_spec("pwratio", format!("{}kg/hp", round_float_to(mass as f64 / (calculator.peak_bhp() as f64), 2)));

        let blank = String::from("---");
        ui_data.ui_info.update_spec("acceleration", blank.clone());
        ui_data.ui_info.update_spec("range", blank.clone());
        ui_data.ui_info.update_spec("topspeed", blank);

        info!("Writing car ui files");
        ui_data.ui_info.write().unwrap();
    }

    Ok(())
}

struct Checksummer {
    car_file_hasher: Sha256,
    sandbox_hasher: Sha256
}

impl Checksummer {
    pub fn new() -> Checksummer {
        Checksummer {
            car_file_hasher: Sha256::new(),
            sandbox_hasher: Sha256::new()
        }
    }
}

fn checksum_engine_data_v1(car_file: &CarFile, sandbox_data: &EngineV1) -> Result<(), String> {
    let mut car_file_hasher = Sha256::new();
    let get_mandatory_attribute_bytes = |section: &Section, attr: &str| {
        match section.get_attribute(attr) {
            None => { Err(format!("Car file section {} is missing attribute {}", section.name(), attr))}
            Some(attribute) => Ok(attribute.value.checksum_bytes())
        }
    };
    let get_optional_attribute_bytes = |section: &Section, attr: &str| {
        match section.get_attribute(attr) {
            None => { None }
            Some(attribute) => Some(attribute.value.checksum_bytes())
        }
    };

    let family_data = car_file.get_section("Car").unwrap().get_section("Family").unwrap();
    for key in ["GameVersion", "UID", "Name", "InternalDays", "QualityFamily", "BlockConfig", "BlockMaterial",
                      "BlockType", "Head", "HeadMaterial", "Valves", "Stroke", "Bore"] {
        car_file_hasher.update(get_mandatory_attribute_bytes(family_data, key)?);
    }

    let mut car_file_family_hash = String::new();
    for byte in car_file_hasher.finalize() {
        car_file_family_hash += &format!("{:X?}", byte);
    }

    let sandbox_family_hash = sandbox_data.family_data_checksum();
    if car_file_family_hash != sandbox_family_hash {
        return Err(format!("Family checksum mismatch.\n Sandbox: {}\n Mod: {}", sandbox_family_hash, car_file_family_hash));
    }
    info!("Family checksum match: {}", sandbox_family_hash);

    let mut car_file_hasher = Sha256::new();
    let variant_data = car_file.get_section("Car").unwrap().get_section("Variant").unwrap();
    for key in ["GameVersion", "FUID", "UID", "Name", "InternalDays", "VVL", "Crank", "Conrods", "Pistons", "VVT",
                      "AspirationType", "IntercoolerSetting", "FuelSystemType", "FuelSystem", ] {
        car_file_hasher.update(get_mandatory_attribute_bytes(variant_data, key)?);
    }
    for key in ["FuelType", "FuelLeaded"] {
        if let Some(bytes) = get_optional_attribute_bytes(variant_data, key) {
            car_file_hasher.update(bytes);
        }
    }
    for key in ["IntakeManifold", "Intake", "Headers", "ExhaustCount", "ExhaustBypassValves", "Cat", "Muffler1",
                      "Muffler2", "Bore", "Stroke", "Capacity", "Compression", "CamProfileSetting", "VVLCamProfileSetting",
                      "AFR", "AFRLean", "RPMLimit", "IgnitionTimingSetting", "ExhaustDiameter", "QualityBottomEnd",
                      "QualityTopEnd", "QualityAspiration", "QualityFuelSystem", "QualityExhaust"] {
        car_file_hasher.update(get_mandatory_attribute_bytes(variant_data, key)?);
    }
    for key in ["BalanceShaft", "SpringStiffnessSetting", "ListedOctane", "TuneOctaneOffset", "AspirationSetup",
                      "AspirationItemOption_1", "AspirationItemOption_2", "AspirationItemSubOption_1", "AspirationItemSubOption_2",
                      "AspirationBoostControl", "ChargerSize_1", "ChargerSize_2", "ChargerTune_1", "ChargerTune_2", "ChargerMaxBoost_1",
                      "ChargerMaxBoost_2", "TurbineSize_1", "TurbineSize_2"] {
        if let Some(bytes) = get_optional_attribute_bytes(variant_data, key) {
            car_file_hasher.update(bytes);
        }
    }
    let mut car_file_variant_hash = String::new();
    for byte in car_file_hasher.finalize() {
        car_file_variant_hash += &format!("{:X?}", byte);
    }
    let sandbox_variant_hash = sandbox_data.variant_data_checksum();
    if car_file_variant_hash != sandbox_variant_hash {
        return Err(format!("Variant checksum mismatch.\n Sandbox: {}\n Mod: {}", sandbox_variant_hash, car_file_variant_hash));
    }
    info!("Variant checksum match: {}", sandbox_variant_hash);
    Ok(())
}

struct EngineDataValidator<'a, 'b> {
    car_file: &'a CarFile,
    sandbox_data: &'b EngineV1,
    float_precision: u32
}

impl<'a, 'b> EngineDataValidator<'a, 'b> {
    fn new(car_file: &'a CarFile,
           sandbox_data: &'b EngineV1) -> EngineDataValidator<'a, 'b> {
        EngineDataValidator {car_file, sandbox_data, float_precision: 10}
    }

    fn validate(&self) -> Result<(), String> {
        let version = self.car_file.get_section("Car").unwrap().get_attribute("Version");
        if let Some(attribute) = version {
            if (attribute.value.as_num()?.round() as u64) < 2200000000 {
                self.validate_legacy_family()?;
                self.validate_legacy_variant()?;
                return Ok(());
            }
        }
        self.validate_family()?;
        self.validate_variant()?;
        Ok(())
    }

    fn validate_legacy_family(&self) -> Result<(), String> {
        let family_data = self.car_file.get_section("Car").unwrap().get_section("Family").unwrap();
        self.validate_u64( *&self.sandbox_data.family_version, family_data, "GameVersion")?;
        self.validate_strings(&self.sandbox_data.family_uuid, family_data,"UID")?;
        self.validate_strings(&self.sandbox_data.family_name, family_data,"Name")?;
        self.validate_i32(*&self.sandbox_data.family_game_days, family_data,"InternalDays")?;
        self.validate_strings(&self.sandbox_data.block_config, family_data,"BlockConfig")?;
        self.validate_strings(&self.sandbox_data.block_material, family_data,"BlockMaterial")?;
        self.validate_strings(&self.sandbox_data.block_type, family_data,"BlockType")?;
        self.validate_strings(&self.sandbox_data.head_type, family_data,"Head")?;
        self.validate_strings(&self.sandbox_data.head_material, family_data,"HeadMaterial")?;
        self.validate_strings(&self.sandbox_data.vvl, family_data,"VVL")?;
        self.validate_strings(&self.sandbox_data.valves, family_data,"Valves")?;
        self.validate_floats(*&self.sandbox_data.max_stroke, family_data,"Stroke")?;
        self.validate_floats(*&self.sandbox_data.max_bore, family_data,"Bore")?;
        Ok(())
    }

    fn validate_family(&self) -> Result<(), String> {
        let family_data = self.car_file.get_section("Car").unwrap().get_section("Family").unwrap();
        self.validate_u64( *&self.sandbox_data.family_version, family_data, "GameVersion")?;
        self.validate_strings(&self.sandbox_data.family_uuid, family_data,"UID")?;
        self.validate_strings(&self.sandbox_data.family_name, family_data,"Name")?;
        self.validate_i32(*&self.sandbox_data.family_game_days, family_data,"InternalDays")?;
        self.validate_i32(*&self.sandbox_data.family_quality, family_data,"QualityFamily")?;
        self.validate_strings(&self.sandbox_data.block_config, family_data,"BlockConfig")?;
        self.validate_strings(&self.sandbox_data.block_material, family_data,"BlockMaterial")?;
        self.validate_strings(&self.sandbox_data.block_type, family_data,"BlockType")?;
        self.validate_strings(&self.sandbox_data.head_type, family_data,"Head")?;
        self.validate_strings(&self.sandbox_data.head_material, family_data,"HeadMaterial")?;
        self.validate_strings(&self.sandbox_data.valves, family_data,"Valves")?;
        self.validate_floats(*&self.sandbox_data.max_stroke, family_data,"Stroke")?;
        self.validate_floats(*&self.sandbox_data.max_bore, family_data,"Bore")?;
        Ok(())
    }

    fn validate_legacy_variant(&self) -> Result<(), String> {
        let variant_data = self.car_file.get_section("Car").unwrap().get_section("Variant").unwrap();
        self.validate_u64( *&self.sandbox_data.variant_version, variant_data, "GameVersion")?;
        self.validate_strings(&self.sandbox_data.family_uuid, variant_data,"FUID")?;
        self.validate_strings(&self.sandbox_data.uuid, variant_data,"UID")?;
        self.validate_strings(&self.sandbox_data.variant_name, variant_data,"Name")?;
        self.validate_i32(*&self.sandbox_data.variant_game_days, variant_data,"InternalDays")?;
        self.validate_strings(&self.sandbox_data.crank, variant_data,"Crank")?;
        self.validate_strings(&self.sandbox_data.conrods, variant_data,"Conrods")?;
        self.validate_strings(&self.sandbox_data.pistons, variant_data,"Pistons")?;
        self.validate_strings(&self.sandbox_data.vvt, variant_data,"VVT")?;
        self.validate_strings(&self.sandbox_data.aspiration, variant_data,"AspirationType")?;
        self.validate_floats(*&self.sandbox_data.intercooler_setting, variant_data,"IntercoolerSetting")?;
        self.validate_strings(&self.sandbox_data.fuel_system_type, variant_data,"FuelSystemType")?;
        self.validate_strings(&self.sandbox_data.fuel_system, variant_data,"FuelSystem")?;
        if let Some(fuel_type) = &self.sandbox_data.fuel_type { self.validate_strings(fuel_type, variant_data,"FuelType")?; }
        self.validate_strings(&self.sandbox_data.intake_manifold, variant_data,"IntakeManifold")?;
        self.validate_strings(&self.sandbox_data.intake, variant_data,"Intake")?;
        self.validate_strings(&self.sandbox_data.headers, variant_data,"Headers")?;
        self.validate_strings(&self.sandbox_data.exhaust_count, variant_data,"ExhaustCount")?;
        self.validate_strings(&self.sandbox_data.exhaust_bypass_valves, variant_data,"ExhaustBypassValves")?;
        self.validate_strings(&self.sandbox_data.cat, variant_data,"Cat")?;
        self.validate_strings(&self.sandbox_data.muffler_1, variant_data,"Muffler1")?;
        self.validate_strings(&self.sandbox_data.muffler_2, variant_data,"Muffler2")?;
        self.validate_floats(*&self.sandbox_data.bore, variant_data,"Bore")?;
        self.validate_floats(*&self.sandbox_data.stroke, variant_data,"Stroke")?;
        self.validate_floats(*&self.sandbox_data.capacity, variant_data,"Capacity")?;
        self.validate_floats(*&self.sandbox_data.compression, variant_data,"Compression")?;
        self.validate_floats(*&self.sandbox_data.cam_profile_setting, variant_data,"CamProfileSetting")?;
        self.validate_floats(*&self.sandbox_data.vvl_cam_profile_setting, variant_data,"VVLCamProfileSetting")?;
        if let Some(afr) = &self.sandbox_data.afr { self.validate_floats(*afr, variant_data,"AFR")?; }
        if let Some(afr_lean) = &self.sandbox_data.afr_lean {self.validate_floats(*afr_lean, variant_data,"AFRLean")?; }
        self.validate_floats(*&self.sandbox_data.rpm_limit, variant_data,"RPMLimit")?;
        self.validate_floats(*&self.sandbox_data.ignition_timing_setting, variant_data,"IgnitionTimingSetting")?;
        self.validate_floats(*&self.sandbox_data.exhaust_diameter, variant_data,"ExhaustDiameter")?;
        self.validate_i32(*&self.sandbox_data.quality_bottom_end, variant_data,"QualityBottomEnd")?;
        self.validate_i32(*&self.sandbox_data.quality_top_end, variant_data,"QualityTopEnd")?;
        self.validate_i32(*&self.sandbox_data.quality_aspiration, variant_data,"QualityAspiration")?;
        self.validate_i32(*&self.sandbox_data.quality_fuel_system, variant_data,"QualityFuelSystem")?;
        self.validate_i32(*&self.sandbox_data.quality_exhaust, variant_data,"QualityExhaust")?;
        Ok(())
    }

    fn validate_variant(&self) -> Result<(), String> {
        let variant_data = self.car_file.get_section("Car").unwrap().get_section("Variant").unwrap();
        self.validate_u64( *&self.sandbox_data.variant_version, variant_data, "GameVersion")?;
        self.validate_strings(&self.sandbox_data.family_uuid, variant_data,"FUID")?;
        self.validate_strings(&self.sandbox_data.uuid, variant_data,"UID")?;
        self.validate_strings(&self.sandbox_data.variant_name, variant_data,"Name")?;
        self.validate_i32(*&self.sandbox_data.variant_game_days, variant_data,"InternalDays")?;
        self.validate_strings(&self.sandbox_data.vvl, variant_data,"VVL")?;
        self.validate_strings(&self.sandbox_data.crank, variant_data,"Crank")?;
        self.validate_strings(&self.sandbox_data.conrods, variant_data,"Conrods")?;
        self.validate_strings(&self.sandbox_data.pistons, variant_data,"Pistons")?;
        self.validate_strings(&self.sandbox_data.vvt, variant_data,"VVT")?;
        self.validate_strings(&self.sandbox_data.aspiration, variant_data,"AspirationType")?;
        self.validate_floats(*&self.sandbox_data.intercooler_setting, variant_data,"IntercoolerSetting")?;
        self.validate_strings(&self.sandbox_data.fuel_system_type, variant_data,"FuelSystemType")?;
        self.validate_strings(&self.sandbox_data.fuel_system, variant_data,"FuelSystem")?;
        if let Some(fuel_type) = &self.sandbox_data.fuel_type { self.validate_strings(fuel_type, variant_data,"FuelType")?; }
        if let Some(fuel_leaded) = &self.sandbox_data.fuel_leaded { self.validate_i32(*fuel_leaded, variant_data,"FuelLeaded")?; }
        self.validate_strings(&self.sandbox_data.intake_manifold, variant_data,"IntakeManifold")?;
        self.validate_strings(&self.sandbox_data.intake, variant_data,"Intake")?;
        self.validate_strings(&self.sandbox_data.headers, variant_data,"Headers")?;
        self.validate_strings(&self.sandbox_data.exhaust_count, variant_data,"ExhaustCount")?;
        self.validate_strings(&self.sandbox_data.exhaust_bypass_valves, variant_data,"ExhaustBypassValves")?;
        self.validate_strings(&self.sandbox_data.cat, variant_data,"Cat")?;
        self.validate_strings(&self.sandbox_data.muffler_1, variant_data,"Muffler1")?;
        self.validate_strings(&self.sandbox_data.muffler_2, variant_data,"Muffler2")?;
        self.validate_floats(*&self.sandbox_data.bore, variant_data,"Bore")?;
        self.validate_floats(*&self.sandbox_data.stroke, variant_data,"Stroke")?;
        self.validate_floats(*&self.sandbox_data.capacity, variant_data,"Capacity")?;
        self.validate_floats(*&self.sandbox_data.compression, variant_data,"Compression")?;
        self.validate_floats(*&self.sandbox_data.cam_profile_setting, variant_data,"CamProfileSetting")?;
        self.validate_floats(*&self.sandbox_data.vvl_cam_profile_setting, variant_data,"VVLCamProfileSetting")?;
        if let Some(afr) = &self.sandbox_data.afr { self.validate_floats(*afr, variant_data,"AFR")?; }
        if let Some(afr_lean) = &self.sandbox_data.afr_lean {self.validate_floats(*afr_lean, variant_data,"AFRLean")?; }
        self.validate_floats(*&self.sandbox_data.rpm_limit, variant_data,"RPMLimit")?;
        self.validate_floats(*&self.sandbox_data.ignition_timing_setting, variant_data,"IgnitionTimingSetting")?;
        self.validate_floats(*&self.sandbox_data.exhaust_diameter, variant_data,"ExhaustDiameter")?;
        self.validate_i32(*&self.sandbox_data.quality_bottom_end, variant_data,"QualityBottomEnd")?;
        self.validate_i32(*&self.sandbox_data.quality_top_end, variant_data,"QualityTopEnd")?;
        self.validate_i32(*&self.sandbox_data.quality_aspiration, variant_data,"QualityAspiration")?;
        self.validate_i32(*&self.sandbox_data.quality_fuel_system, variant_data,"QualityFuelSystem")?;
        self.validate_i32(*&self.sandbox_data.quality_exhaust, variant_data,"QualityExhaust")?;
        if let Some(val) = &self.sandbox_data.balance_shaft { self.validate_strings(val, variant_data, "BalanceShaft")?; }
        if let Some(val) = &self.sandbox_data.spring_stiffness { self.validate_floats(*val, variant_data, "SpringStiffnessSetting")?; }
        if let Some(val) = &self.sandbox_data.listed_octane { self.validate_i32(*val, variant_data, "ListedOctane")?; }
        if let Some(val) = &self.sandbox_data.tune_octane_offset { self.validate_i32(*val, variant_data, "TuneOctaneOffset")?; }
        if let Some(val) = &self.sandbox_data.aspiration_setup { self.validate_strings(val, variant_data, "AspirationSetup")?; }
        if let Some(val) = &self.sandbox_data.aspiration_item_1 { self.validate_strings(val, variant_data, "AspirationItemOption_1")?; }
        if let Some(val) = &self.sandbox_data.aspiration_item_2 { self.validate_strings(val, variant_data, "AspirationItemOption_2")?; }
        if let Some(val) = &self.sandbox_data.aspiration_item_suboption_1 { self.validate_strings(val, variant_data, "AspirationItemSubOption_1")?; }
        if let Some(val) = &self.sandbox_data.aspiration_item_suboption_2 { self.validate_strings(val, variant_data, "AspirationItemSubOption_2")?; }
        if let Some(val) = &self.sandbox_data.aspiration_boost_control { self.validate_strings(val, variant_data, "AspirationBoostControl")?; }
        if let Some(val) = &self.sandbox_data.charger_size_1 { self.validate_floats(*val, variant_data, "ChargerSize_1")?; }
        if let Some(val) = &self.sandbox_data.charger_size_2 { self.validate_floats(*val, variant_data, "ChargerSize_2")?; }
        if let Some(val) = &self.sandbox_data.charger_tune_1 { self.validate_floats(*val, variant_data, "ChargerTune_1")?; }
        if let Some(val) = &self.sandbox_data.charger_tune_2 { self.validate_floats(*val, variant_data, "ChargerTune_2")?; }
        if let Some(val) = &self.sandbox_data.charger_max_boost_1 { self.validate_floats(*val, variant_data, "ChargerMaxBoost_1")?; }
        if let Some(val) = &self.sandbox_data.charger_max_boost_2 { self.validate_floats(*val, variant_data, "ChargerMaxBoost_2")?; }
        if let Some(val) = &self.sandbox_data.turbine_size_1 { self.validate_floats(*val, variant_data, "TurbineSize_1")?; }
        if let Some(val) = &self.sandbox_data.turbine_size_2 { self.validate_floats(*val, variant_data, "TurbineSize_2")?; }
        Ok(())
    }

    fn validate_strings(&self,
                        sandbox_value: &str,
                        car_file_section: &Section,
                        car_file_key: &str) -> Result<(), String> {
        let attr = match car_file_section.get_attribute(car_file_key) {
            None => { Err(format!("Car file section {} is missing attribute {}", car_file_section.name(), car_file_key))}
            Some(attribute) => Ok(attribute.value.as_str())
        }?;
        info!("Checking {}", car_file_key);
        if sandbox_value != attr {
            return Err(format!("{}: {} != {}", car_file_key, sandbox_value, attr))
        }
        Ok(())
    }

    fn validate_u64(&self,
                    sandbox_value: u64,
                    car_file_section: &Section,
                    car_file_key: &str) -> Result<(), String> {
        let attr = match car_file_section.get_attribute(car_file_key) {
            None => { Err(format!("Car file section {} is missing attribute {}", car_file_section.name(), car_file_key))}
            Some(attribute) => Ok(attribute.value.as_num()?.round() as u64)
        }?;
        info!("Checking {}", car_file_key);
        if sandbox_value != attr {
            return Err(format!("{}: {} != {}", car_file_key, sandbox_value, attr))
        }
        Ok(())
    }

    fn validate_i32(&self,
                     sandbox_value: i32,
                     car_file_section: &Section,
                     car_file_key: &str) -> Result<(), String> {
        let attr = match car_file_section.get_attribute(car_file_key) {
            None => { Err(format!("Car file section {} is missing attribute {}", car_file_section.name(), car_file_key))}
            Some(attribute) => Ok(attribute.value.as_num()?.round() as i32)
        }?;
        info!("Checking {}", car_file_key);
        if sandbox_value != attr {
            return Err(format!("{}: {} != {}", car_file_key, sandbox_value, attr))
        }
        Ok(())
    }

    fn validate_floats(&self,
                       sandbox_value: f64,
                       car_file_section: &Section,
                       car_file_key: &str) -> Result<(), String> {
        let attr = match car_file_section.get_attribute(car_file_key) {
            None => { Err(format!("Car file section {} is missing attribute {}", car_file_section.name(), car_file_key))}
            Some(attribute) => Ok(attribute.value.as_num()?)
        }?;
        info!("Checking {}", car_file_key);
        if round_float_to(sandbox_value, self.float_precision) !=
           round_float_to(attr, self.float_precision) {
            return Err(format!("{}: {} != {}", car_file_key, sandbox_value, attr))
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::fs::File;
    use std::path::{Path, PathBuf};
    use crate::assetto_corsa::car::create_new_car_spec;
    use crate::{automation, beam_ng};
    use crate::beam_ng::get_mod_list;
    use crate::fabricator::{AcEngineParameterCalculatorV1, AdditionalAcCarData, AssettoCorsaCarSettings, swap_automation_engine_into_ac_car};

    #[test]
    fn load_mods() -> Result<(), String> {
        let mods = get_mod_list();
        let calculator = AcEngineParameterCalculatorV1::from_beam_ng_mod(mods[0].as_path())?;
        std::fs::write("inertia.txt",format!("{}", calculator.inertia().unwrap()));
        std::fs::write("idle.txt",format!("{}", calculator.idle_speed().unwrap()));
        std::fs::write("limiter.txt",format!("{}", calculator.limiter()));
        std::fs::write("fuel_cons.txt",format!("{}", calculator.basic_fuel_consumption().unwrap()));
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
        let mod_data = beam_ng::ModData::from_path(&path.join("dae1.zip"))?;
        let automation_car_file = automation::car::CarFile::from_bytes( mod_data.get_automation_car_file_data().ok_or("Couldn't find car data")?.clone())?;
        //println!("{:#?}", automation_car_file);
        for section in automation_car_file.get_section("Car").unwrap().get_section("Variant").unwrap().attribute_keys() {
            println!("{}", section);
        }
        //fs::write(Path::new("car_temp.toml"), format!("{}", automation_car_file));
        Ok(())
    }
}
