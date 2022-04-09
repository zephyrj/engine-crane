use std::fmt::{Display, format, Formatter};
use std::num;
use std::path::{Path};
use iced::keyboard::KeyCode::N;
use tracing::{debug, error, info};
use crate::{assetto_corsa, automation, beam_ng};
use crate::assetto_corsa::car::{Car, CarVersion, SpecValue};
use crate::assetto_corsa::drivetrain::DriveType;
use crate::assetto_corsa::engine::{CoastCurve, ControllerCombinator, ControllerInput, Damage, Metadata, Turbo, TurboController, TurboControllers, TurboSection};
use crate::assetto_corsa::traits::{CarIniData, extract_mandatory_section, extract_optional_section};
use crate::automation::car::{Attribute, CarFile};
use crate::automation::sandbox::{EngineV1, load_engine_by_uuid, SandboxVersion};
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

pub fn round_float_to(float: f64, decimal_places: u32) -> f64 {
    let precision_base: u64 = 10;
    let precision_factor = precision_base.pow(decimal_places) as f64;
    (float * precision_factor).round() / precision_factor
}

pub fn normalise_boost_value(boost_value: f64, decimal_places: u32) -> f64 {
    round_float_to(vec![0.0, boost_value].into_iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(), decimal_places)
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
        let mod_data = beam_ng::extract_mod_data(beam_ng_mod_path)?;
        let automation_car_file = automation::car::CarFile::from_bytes( mod_data.car_file_data)?;
        let engine_jbeam_data = mod_data.engine_jbeam_data;
        let variant_info = automation_car_file.get_section("Car").unwrap().get_section("Variant").unwrap();
        let uid = variant_info.get_attribute("UID").unwrap().value.as_str();
        let version_num = variant_info.get_attribute("GameVersion").unwrap().value.as_num().unwrap();
        info!("Engine uuid: {}", uid);
        info!("Engine version number: {}", version_num);
        let version = SandboxVersion::from_version_number(version_num as i32);
        info!("Deduced as {}", version);
        let engine_sqlite_data = match load_engine_by_uuid(uid, version)? {
            None => {
                return Err(format!("No engine found with uuid {}", uid));
            }
            Some(eng) => { eng }
        };
        Ok(AcEngineParameterCalculatorV1 {
            automation_car_file,
            engine_jbeam_data,
            engine_sqlite_data
        })
    }

    pub fn engine_weight(&self) -> u32 {
        self.engine_sqlite_data.weight.round() as u32
    }

    pub fn inertia(&self) -> Option<f64> {
        let eng_map = self.engine_jbeam_data.get("Camso_Engine")?.as_object()?.get("mainEngine")?.as_object()?;
        eng_map.get("inertia")?.as_f64()
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

    pub fn fuel_flow_consumption(&self, mechanical_efficiency: f64) -> assetto_corsa::engine::FuelConsumptionFlowRate {
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
        assetto_corsa::engine::FuelConsumptionFlowRate::new(
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
    pub fn engine_power_curve(&self) -> Vec<(i32, i32)> {
        let mut out_vec = Vec::new();
        let kw_to_bhp = |power_kw: f64| { power_kw * 1.341 };
        for (idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
            out_vec.push(((*rpm as i32), kw_to_bhp(self.engine_sqlite_data.power_curve[idx]).round() as i32));
        }
        out_vec
    }

    pub fn peak_power(&self) -> i32 {
        self.engine_sqlite_data.peak_power.round() as i32
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

    pub fn create_turbo(&self) -> Option<assetto_corsa::engine::Turbo> {
        if self.engine_sqlite_data.aspiration.starts_with("Aspiration_Natural") {
            return None;
        }
        // todo update this to take into account the boost amount set and ignore any overboost that may skew the turbo section calculation
        let (ref_rpm, max_boost) = self.get_max_boost_params(3);
        let mut t = Turbo::new();
        t.add_section(TurboSection::new(
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

    pub fn create_turbo_controllers(&self, out_path: &Path) -> Option<assetto_corsa::engine::TurboControllers> {
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

    pub fn coast_data(&self) -> Option<assetto_corsa::engine::CoastCurve> {
        //   The following data is available from the engine.jbeam exported file
        //   The dynamic friction torque on the engine in Nm/s.
        //   This is a friction torque which increases proportional to engine AV (rad/s).
        //   AV = (2pi * RPM) / 60
        //   friction torque = (AV * dynamicFriction) + 2*staticFriction
        //
        //   #### NOTE ####
        //   I'm assuming that all of the sources of friction are being taken into account in the BeamNG parameters used above
        //   this may not be correct.
        let eng_map = self.engine_jbeam_data.get("Camso_Engine")?.as_object()?.get("mainEngine")?.as_object()?;
        let dynamic_friction = eng_map.get("dynamicFriction")?.as_f64().unwrap();
        let static_friction = eng_map.get("friction")?.as_f64().unwrap();
        let angular_velocity_at_max_rpm = (self.engine_sqlite_data.max_rpm * 2_f64 * std::f64::consts::PI) / 60_f64;
        let friction_torque = (angular_velocity_at_max_rpm * dynamic_friction) + (2_f64 * static_friction);
        Some(CoastCurve::new_from_coast_ref(self.engine_sqlite_data.max_rpm.round() as i32,
                                            friction_torque.round() as i32,
                                            0.0))
    }

    pub fn damage(&self) -> assetto_corsa::engine::Damage {
        let (_, max_boost) = self.get_max_boost_params(2);
        Damage::new(
            (self.limiter()+200_f64).round() as i32,
            1,
            Some(max_boost.ceil()),
            match self.engine_sqlite_data.aspiration.as_str() {
                "Aspiration_Natural" => { Some(0) },
                _ => { Some(4) }
            }
        )
    }

    pub fn create_metadata(&self) -> assetto_corsa::engine::Metadata {
        let mut m = Metadata::new();
        m.set_version(2);
        m.set_source(assetto_corsa::engine::Source::Automation);
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
    let mut ac_car = Car::load_from_path(ac_car_path).unwrap();
    let drive_type = &extract_mandatory_section::<assetto_corsa::drivetrain::Traction>(ac_car.drivetrain()).unwrap().drive_type;
    info!("Existing car is {} with assumed mechanical efficiency of {}", drive_type, drive_type.mechanical_efficiency());
    match settings.minimum_physics_level {
        AssettoCorsaPhysicsLevel::BaseGame => {
            info!("Using base game physics");
            ac_car.set_fuel_consumption(calculator.basic_fuel_consumption().unwrap());
        }
        AssettoCorsaPhysicsLevel::CspExtendedPhysics => {
            info!("Using CSP extended physics");
            ac_car.set_version(CarVersion::CspExtendedPhysics);
            ac_car.clear_fuel_consumption();
            ac_car.mut_engine().update_component(&calculator.fuel_flow_consumption(drive_type.mechanical_efficiency())).map_err(|err| {
                error!("Failed to update fuel consumption. {}", err.to_string());
                err.to_string()
            })?;
        }
    }

    if let Some(current_engine_weight) = additional_car_data.engine_weight() {
        if let Some(current_car_mass) = ac_car.total_mass() {
            let new_engine_delta: i32 = calculator.engine_weight() as i32 - current_engine_weight as i32;
            if new_engine_delta < 0 && new_engine_delta.abs() as u32 >= current_car_mass {
                error!("Invalid existing engine weight ({}). Would result in negative total mass", current_engine_weight);
            }
            let new_mass = (current_car_mass as i32 + new_engine_delta) as u32;
            info!("Updating total mass to {} based off a provided existing engine weight of {}", new_mass, current_engine_weight);
            ac_car.set_total_mass(new_mass);
        } else {
            error!("Existing car doesn't have a total mass property")
        }
    }

    {
        let engine = ac_car.mut_engine();
        info!("Clearing existing turbo controllers");
        engine.clear_turbo_controllers().unwrap();

        let mut engine_data = extract_mandatory_section::<assetto_corsa::engine::EngineData>(engine).unwrap();
        let inertia = calculator.inertia().unwrap();
        engine_data.inertia = inertia;
        let limiter = calculator.limiter().round() as i32;
        engine_data.limiter = limiter;
        engine_data.minimum = calculator.idle_speed().unwrap().round() as i32;
        engine.update_component(&engine_data).unwrap();
        engine.update_component(&calculator.damage()).unwrap();
        engine.update_component(&calculator.coast_data().unwrap()).unwrap();
        engine.update_power_curve(calculator.naturally_aspirated_wheel_torque_curve(drive_type.mechanical_efficiency())).unwrap();

        match calculator.create_turbo() {
            None => {
                info!("The new engine doesn't have a turbo");
                if let Some(mut old_turbo) = extract_optional_section::<Turbo>(engine).unwrap() {
                    info!("Removing old engine turbo parameters");
                    old_turbo.clear_sections();
                    old_turbo.bov_pressure_threshold = None;
                    engine.update_component(&old_turbo).unwrap();
                }
            }
            Some(new_turbo) => {
                info!("The new engine has a turbo");
                engine.update_component(&new_turbo).unwrap();
                if let Some(turbo_ctrl) = calculator.create_turbo_controllers(engine.data_dir()) {
                    info!("Adding turbo controller with index 0");
                    engine.add_turbo_controllers(0, turbo_ctrl);
                }
            }
        }

        info!("Writing engine ini files");
        engine.write().unwrap();
    }

    info!("Updating ui components");
    let mass = ac_car.total_mass().unwrap();
    ac_car.ui_info.update_power_curve(calculator.engine_power_curve());
    ac_car.ui_info.update_torque_curve(calculator.engine_torque_curve());
    ac_car.ui_info.update_spec("bhp", format!("{}bhp", calculator.peak_power()));
    ac_car.ui_info.update_spec("torque", format!("{}Nm", calculator.peak_torque()));
    ac_car.ui_info.update_spec("weight", format!("{}kg", mass));
    ac_car.ui_info.update_spec("pwratio", format!("{}kg/hp", round_float_to(mass as f64 / (calculator.peak_power() as f64), 2)));

    let blank = String::from("---");
    ac_car.ui_info.update_spec("acceleration", blank.clone());
    ac_car.ui_info.update_spec("range", blank.clone());
    ac_car.ui_info.update_spec("topspeed", blank);

    info!("Writing car ui files");
    ac_car.ui_info.write().unwrap();

    info!("Writing car ini files");
    ac_car.write().unwrap();

    Ok(())
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
        std::fs::write("turbo_ctrl.txt",format!("{}", calculator.create_turbo_controllers(PathBuf::from("").as_path()).unwrap()));
        std::fs::write("turbo.txt",format!("{:?}", calculator.create_turbo().unwrap()));
        std::fs::write("coast.txt",format!("{:?}", calculator.coast_data().unwrap()));
        std::fs::write("metadata.txt",format!("{:?}", calculator.create_metadata()));
        std::fs::write("fuel_flow.txt", format!("{:?}", calculator.fuel_flow_consumption(0.75))).unwrap();
        std::fs::write("damage.txt", format!("{:?}", calculator.damage())).unwrap();
        Ok(())
    }

    #[test]
    fn clone_and_swap_test() -> Result<(), String> {
        let new_car_path = create_new_car_spec("zephyr_za401", "test").unwrap();
        let mods = get_mod_list();
        swap_automation_engine_into_ac_car(mods[0].as_path(),
                                           new_car_path.as_path(),
                                           AssettoCorsaCarSettings::default(),
                                           AdditionalAcCarData::default())
    }

    #[test]
    fn dump_automation_car_file() -> Result<(), String> {
        let mods = get_mod_list();
        let mod_data = beam_ng::extract_mod_data(mods[0].as_path())?;
        let automation_car_file = automation::car::CarFile::from_bytes( mod_data.car_file_data)?;
        fs::write(Path::new("car_temp.toml"), format!("{}", automation_car_file));
        Ok(())
    }
}
