pub mod metadata;
pub mod engine_data;
pub mod power_curve;
pub mod fuel_consumption;
pub mod damage;
pub mod coast;
pub mod turbo_ctrl;
pub mod turbo;

use std::collections::HashMap;
use std::path::Path;
use crate::assetto_corsa::car::Car;
use crate::assetto_corsa::error::{Result, Error, ErrorKind};
use crate::assetto_corsa::ini_utils::Ini;
use crate::assetto_corsa::traits::{CarDataFile, DataInterface};

pub use metadata::Metadata;
pub use engine_data::EngineData;
pub use power_curve::PowerCurve;
pub use fuel_consumption::FuelConsumptionFlowRate;
pub use damage::Damage;
pub use coast::CoastCurve;
pub use turbo::Turbo;
pub use turbo_ctrl::TurboControllerFile;


#[derive(Debug)]
pub struct Engine<'a> {
    car: &'a mut Car,
    ini_data: Ini,
}

impl<'a> Engine<'a> {
    const INI_FILENAME: &'static str = "engine.ini";

    // pub fn load_from_ini_string(ini_data: String) -> Engine<'a> {
    //     let ini_data = Ini::load_from_string(ini_data);
    //     let power_curve= LutProperty::path_only(String::from("HEADER"), String::from("POWER_CURVE"), &ini_data).unwrap();
    //     Engine {
    //         ini_data,
    //         power_curve,
    //         turbo_controllers: HashMap::new()
    //     }
    // }

    pub fn from_car(car: & mut Car) -> Result<Engine> {
        let data_interface = car.mut_data_interface();
        let file_data = data_interface.get_file_data(Engine::INI_FILENAME)?;
        let ini_data = Ini::load_from_string(String::from_utf8_lossy(file_data.as_slice()).into_owned());
        Ok(Engine {
            car,
            ini_data,
        })
    }

    pub fn to_bytes_map(&self) -> HashMap<String, Vec<u8>> {
        let mut map = HashMap::new();
        map.insert(Engine::INI_FILENAME.to_owned(), self.ini_data.to_bytes());
        map
    }

    pub fn write(&mut self) -> Result<()> {
        self.car.mut_data_interface().write_file_data(Engine::INI_FILENAME, self.ini_data.to_bytes())?;
        Ok(())
    }

    pub fn write_to_dir(&mut self, dir: &Path) -> Result<()> {
        self.ini_data.write_to_file(&dir.join(Engine::INI_FILENAME))?;
        Ok(())
    }
}

impl<'a> CarDataFile for Engine<'a> {
    fn ini_data(&self) -> &Ini {
        &self.ini_data
    }
    fn mut_ini_data(&mut self) -> &mut Ini {
        &mut self.ini_data
    }
    fn data_interface(&self) -> &dyn DataInterface {
        self.car.data_interface()
    }
    fn mut_data_interface(&mut self) -> &mut dyn DataInterface {
        self.car.mut_data_interface()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::assetto_corsa::car::data::engine::{CoastCurve, Damage, Engine, EngineData, ExtendedFuelConsumptionBaseData, FuelConsumptionFlowRate, Turbo};
    use crate::assetto_corsa::ini_utils::IniUpdater;
    use crate::assetto_corsa::car::lut_utils::{InlineLut, LutType};
    use crate::assetto_corsa::car::structs::LutProperty;
    use crate::assetto_corsa::traits::{extract_mandatory_section, extract_optional_section, MandatoryDataSection};

    const TURBO_NO_CTRL_DATA: &'static str = r#"
[HEADER]
VERSION=1
POWER_CURVE=power.lut			; power curve file
COAST_CURVE=FROM_COAST_REF 		; coast curve. can define 3 different options (coast reference, coast values for mathematical curve, coast curve file)

[ENGINE_DATA]
ALTITUDE_SENSITIVITY=0.1	; sensitivity to altitude
INERTIA=0.120					; engine inertia
LIMITER=6500					; engine rev limiter. 0 no limiter
LIMITER_HZ=30
MINIMUM=1250

[COAST_REF]
RPM=7000						; rev number reference
TORQUE=60						; engine braking torque value in Nm at rev number reference
NON_LINEARITY=0					; coast engine brake from ZERO to TORQUE value at rpm with linear (0) to fully exponential (1)


[TURBO_0]
LAG_DN=0.99				; Interpolation lag used slowing down the turbo
LAG_UP=0.99				; Interpolation lag used to spin up the turbo
MAX_BOOST=1.00				; Maximum boost generated. This value is never exceeded and multiply the torque like T=T*(1.0 + boost), so a boost of 2 will give you 3 times the torque at a given rpm.
WASTEGATE=0			; Max level of boost before the wastegate does its things. 0 = no wastegate
DISPLAY_MAX_BOOST=1.00	; Value used by display apps
REFERENCE_RPM=5000			; The reference rpm where the turbo reaches maximum boost (at max gas pedal).
GAMMA=1
COCKPIT_ADJUSTABLE=0

[BOV]
PRESSURE_THRESHOLD=0.5 ; the pressure on the air intake that the valve can take before opening, the pressure on the intake depends on throttle, this is mostly used for fmod audio

[DAMAGE]
TURBO_BOOST_THRESHOLD=1.5  ; level of TOTAL boost before the engine starts to take damage
TURBO_DAMAGE_K=5			; amount of damage per second per (boost - threshold)
RPM_THRESHOLD=6700			; RPM at which the engine starts to take damage
RPM_DAMAGE_K=1
    "#;

    #[test]
    fn load_engine() -> Result<(), String> {
        let this_file = Path::new(file!());
        let this_dir = this_file.parent().unwrap();
        let path = this_dir.join("test-data/car-with-turbo-with-ctrls/data");
        match Engine::load_from_dir(&path) {
            Ok(engine) => {
                let metadata = engine.metadata().map_err(|err|{
                    err.to_string()
                })?;
                let coast_curve = extract_mandatory_section::<CoastCurve>(&engine).map_err(|err|{
                    err.to_string()
                })?;
                let engine_data = extract_mandatory_section::<EngineData>(&engine).map_err(|err|{
                    err.to_string()
                })?;
                let damage = extract_mandatory_section::<Damage>(&engine).map_err(|err|{
                    err.to_string()
                })?;
                let turbo = extract_optional_section::<Turbo>(&engine).map_err(|err|{
                    err.to_string()
                })?;
                assert!(turbo.is_some());
                Ok(())
            }
            Err(e) => { Err(e.to_string()) }
        }
    }

    #[test]
    fn update_engine_data() -> Result<(), String> {
        let new_altitude_sensitivity = 0.15;
        let new_inertia = 0.140;
        let new_limiter = 7000;
        let new_limiter_hz = 40;
        let new_minimum = 900;

        let output_ini_string = component_update_test(|engine_data: &mut EngineData| {
            engine_data.altitude_sensitivity = new_altitude_sensitivity;
            engine_data.inertia = new_inertia;
            engine_data.limiter = new_limiter;
            engine_data.limiter_hz = new_limiter_hz;
            engine_data.minimum = new_minimum;
        })?;
        validate_component(output_ini_string, |engine_data: &EngineData| {
            assert_eq!(engine_data.altitude_sensitivity, new_altitude_sensitivity, "altitude_sensitivity is correct");
            assert_eq!(engine_data.inertia, new_inertia, "inertia is correct");
            assert_eq!(engine_data.limiter, new_limiter, "limiter is correct");
            assert_eq!(engine_data.limiter_hz, new_limiter_hz, "limiter_hz is correct");
            assert_eq!(engine_data.minimum, new_minimum, "minimum is correct");
        })
    }

    #[test]
    fn update_coast_curve() -> Result<(), String> {
        let new_rpm = 9000;
        let new_torque = 80;
        let new_non_linearity = 0.5;

        let output_ini_string = component_update_test(|coast_curve: &mut CoastCurve| {
            coast_curve.reference_rpm = new_rpm;
            coast_curve.torque = new_torque;
            coast_curve.non_linearity = new_non_linearity;
        })?;
        validate_component(output_ini_string, |coast_curve: &CoastCurve| {
            assert_eq!(coast_curve.reference_rpm, new_rpm, "Reference rpm is correct");
            assert_eq!(coast_curve.torque, new_torque, "torque is correct");
            assert_eq!(coast_curve.non_linearity, new_non_linearity, "non-linearity is correct");
        })
    }

    #[test]
    fn update_damage() -> Result<(), String> {
        let new_turbo_boost_threshold = Some(1.9);
        let new_turbo_damage_k = Some(10);
        let new_rpm_threshold = 7000;
        let rpm_damage_k = 2;

        let output_ini_string = component_update_test(|damage: &mut Damage| {
            damage.turbo_boost_threshold = new_turbo_boost_threshold;
            damage.turbo_damage_k = new_turbo_damage_k;
            damage.rpm_threshold = new_rpm_threshold;
            damage.rpm_damage_k = rpm_damage_k;
        })?;
        validate_component(output_ini_string, |damage: &Damage| {
            assert_eq!(damage.turbo_boost_threshold, new_turbo_boost_threshold, "turbo_boost_threshold is correct");
            assert_eq!(damage.turbo_damage_k, new_turbo_damage_k, "turbo_damage_k is correct");
            assert_eq!(damage.rpm_threshold, new_rpm_threshold, "rpm_threshold is correct");
            assert_eq!(damage.rpm_damage_k, rpm_damage_k, "rpm_damage_k is correct");
        })
    }

    #[test]
    fn update_fuel_flow_rate() -> Result<(), String> {
        let new_mechanical_efficiency = 0.85;
        let new_idle_throttle = 0.04;
        let new_idle_cutoff = 1100;
        let new_max_fuel_flow = 100;
        let fuel_flow_vec = vec![(1000, 10), (2000, 20), (3000, 30), (4000, 40), (5000, 50), (6000, 60)];

        let mut engine = Engine::load_from_ini_string(String::from(TURBO_NO_CTRL_DATA));
        engine.update_component(&FuelConsumptionFlowRate::new(
            new_idle_throttle,
            new_idle_cutoff,
            new_mechanical_efficiency,
            Some(fuel_flow_vec.clone()),
            new_max_fuel_flow
        )).map_err(|err| format!("{}", err.to_string()))?;

        let ini_string = engine.ini_data.to_string();
        let engine = Engine::load_from_ini_string(ini_string);
        let component = extract_optional_section::<FuelConsumptionFlowRate>(&engine).map_err(|err| format!("{}", err.to_string()))?.unwrap();
        assert_eq!(component.base_data.mechanical_efficiency, Some(new_mechanical_efficiency), "mechanical_efficiency is correct");
        assert_eq!(component.base_data.idle_cutoff, Some(new_idle_cutoff), "idle_cutoff is correct");
        assert_eq!(component.base_data.idle_throttle, Some(new_idle_throttle), "idle_throttle is correct");
        assert_eq!(component.max_fuel_flow, new_max_fuel_flow, "max_fuel_flow is correct");
        assert!(component.max_fuel_flow_lut.is_some());
        let lut = component.max_fuel_flow_lut.unwrap();
        assert_eq!(fuel_flow_vec, lut.to_vec(), "max_fuel_flow_lut is correct");
        Ok(())
    }


    fn component_update_test<T: IniUpdater + MandatoryDataSection, F: FnOnce(&mut T)>(component_update_fn: F) -> Result<String, String> {
        let mut engine = Engine::load_from_ini_string(String::from(TURBO_NO_CTRL_DATA));
        let mut component = extract_mandatory_section::<T>(&engine).unwrap();
        component_update_fn(&mut component);
        engine.update_component(&component).map_err(|err| format!("{}", err.to_string()))?;
        Ok(engine.ini_data.to_string())
    }

    fn validate_component<T, F>(ini_string: String, component_validation_fn: F) -> Result<(), String>
        where T: MandatoryDataSection,
              F: FnOnce(&T)
    {
        let engine = Engine::load_from_ini_string(ini_string);
        let component = extract_mandatory_section::<T>(&engine).map_err(|err| format!("{}", err.to_string()))?;
        component_validation_fn(&component);
        Ok(())
    }
}