use std::cmp::max;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use iced::keyboard::KeyCode::P;
use serde_hjson::Map;
use toml::Value;
use crate::{assetto_corsa, automation, beam_ng};
use crate::assetto_corsa::car::Car;
use crate::assetto_corsa::drivetrain::DriveType;
use crate::assetto_corsa::engine::{ControllerCombinator, ControllerInput, TurboController, TurboControllers, TurboSection};
use crate::automation::car::CarFile;
use crate::automation::sandbox::{EngineV1, load_engine_by_uuid};
use crate::beam_ng::ModData;

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

#[derive(Debug)]
struct AcEngineParameterCalculatorV1 {
    automation_car_file: CarFile,
    engine_jbeam_data: serde_hjson::Map<String, serde_hjson::Value>,
    engine_sqlite_data: EngineV1
}

impl AcEngineParameterCalculatorV1 {
    pub fn from_beam_ng_mod(beam_ng_mod_path: &Path) -> Result<AcEngineParameterCalculatorV1, String> {
        let mod_data = beam_ng::extract_mod_data(beam_ng_mod_path)?;
        let automation_car_file = automation::car::CarFile::from_bytes( mod_data.car_file_data)?;
        let engine_jbeam_data = mod_data.engine_jbeam_data;
        let uid = automation_car_file.get_section("Car").unwrap().get_section("Variant").unwrap().get_attribute("UID").unwrap().value.as_str().unwrap();
        let engine_sqlite_data = match load_engine_by_uuid(uid)? {
            None => { return Err(String::from("No engine found")); }
            Some(eng) => { eng }
        };
        Ok(AcEngineParameterCalculatorV1 {
            automation_car_file,
            engine_jbeam_data,
            engine_sqlite_data
        })
    }

    pub fn inertia(&self) -> Option<f64> {
        let eng_map = self.engine_jbeam_data.get("Camso_Engine")?.as_object()?.get("mainEngine")?.as_object()?;
        eng_map.get("inertia")?.as_f64()
    }

    pub fn idle_speed(&self) -> Option<f64> {
        let values = vec![self.engine_sqlite_data.idle_speed, self.engine_sqlite_data.rpm_curve[0]];
        values.into_iter().max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    pub fn limiter(&self) -> Option<f64> {
        Some(self.engine_sqlite_data.max_rpm)
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

    pub fn naturally_aspirated_torque_curve(&self) -> Vec<(i32, f64)> {
        let mut out_vec = Vec::new();
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
                out_vec.push(((*rpm as i32), (self.engine_sqlite_data.torque_curve[idx])) );
            }
        }
        else {
            for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
                let boost_pressure = vec![0.0, self.engine_sqlite_data.boost_curve[idx]].into_iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
                out_vec.push(((*rpm as i32), (self.engine_sqlite_data.torque_curve[idx] / 1f64+boost_pressure)) );
            }
        }
        out_vec
    }

    pub fn create_turbo_section(&self, out_path: &Path) -> Option<Vec<assetto_corsa::engine::Turbo>> {
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            return None;
        }

        let section = TurboSection::new(
            out_path,
            0,
            // TODO work out how to better approximate these
            0.99,
            0.965,
            self.engine_sqlite_data.peak_boost,
            self.engine_sqlite_data.peak_boost,
            (self.engine_sqlite_data.peak_boost * 10_f64).ceil() / 10_f64,
            self.engine_sqlite_data.peak_boost_rpm.round() as i32,
            2.5,
            0
        );
        None
    }

    pub fn create_turbo_controllers(&self, out_path: &Path) -> Option<assetto_corsa::engine::TurboControllers> {
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            return None;
        }

        let mut lut: Vec<(f64, f64)> = Vec::new();
        for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
            lut.push((*rpm, (self.engine_sqlite_data.boost_curve[idx].r)));
        }
        let controller = TurboController::new(
            out_path,
            0,
            ControllerInput::Rpms,
            ControllerCombinator::Add,
            lut,
            0.95,
            10000_f64,
            0_f64
        );
        let mut controllers = TurboControllers::new(out_path, 0);
        controllers.add_controller(controller).unwrap();
        Some(controllers)
    }
}

pub fn build_ac_engine_from_beam_ng_mod(beam_ng_mod_path: &Path) -> Result<(), String>{
    let calculator = AcEngineParameterCalculatorV1::from_beam_ng_mod(beam_ng_mod_path)?;
    println!("inertia = {}", calculator.inertia().unwrap());
    println!("idle speed = {}", calculator.idle_speed().unwrap());
    println!("limiter = {}", calculator.limiter().unwrap());
    println!("basic fuel consumption = {}", calculator.basic_fuel_consumption().unwrap());
    println!("na torque curve = {:?}", calculator.naturally_aspirated_torque_curve());
    println!("turbo controllers = {}", calculator.create_turbo_controllers(PathBuf::from("").as_path()).unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::beam_ng::get_mod_list;
    use crate::fabricator::build_ac_engine_from_beam_ng_mod;

    #[test]
    fn load_mods() -> Result<(), String> {
        let mods = get_mod_list().unwrap();
        build_ac_engine_from_beam_ng_mod(PathBuf::from(&mods[0]).as_path());
        Ok(())
    }
}
