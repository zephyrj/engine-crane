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

mod assetto_corsa;

use std::fmt::{Display, Formatter};
use std::io;
use std::path::Path;
use itertools::Itertools;
use serde_hjson;
use sha2::{Digest};
use tracing::{error, info, warn};
use utils::numeric::{round_float_to, round_up_to_nearest_multiple};

use crate::assetto_corsa::car::data::engine::{CoastCurve, Damage, EngineData, PowerCurve};

use crate::beam_ng;
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

use crate::assetto_corsa::traits::{extract_mandatory_section, extract_optional_section, OptionalDataSection, update_car_data};
use crate::fabricator::FabricationError::MissingDataSection;


#[derive(thiserror::Error, Debug)]
pub enum FabricationError {
    #[error("io error")]
    IoError(#[from] io::Error),
    #[error("assetto corsa data error")]
    ACDataError(#[from] crate::assetto_corsa::error::Error),
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

pub fn swap_automation_engine_into_ac_car(beam_ng_mod_path: &Path,
                                          ac_car_path: &Path,
                                          settings: AssettoCorsaCarSettings,
                                          additional_car_data: AdditionalAcCarData) -> Result<(), FabricationError> {
    update_ac_engine_parameters(ac_car_path,
                                assetto_corsa::EngineParameterCalculator::from_beam_ng_mod(beam_ng_mod_path)?,
                                settings, additional_car_data
    )
}

pub fn swap_crate_engine_into_ac_car(crate_engine_path: &Path,
                                     ac_car_path: &Path,
                                     settings: AssettoCorsaCarSettings,
                                     additional_car_data: AdditionalAcCarData) -> Result<(), FabricationError> {
    update_ac_engine_parameters(ac_car_path,
                                assetto_corsa::EngineParameterCalculator::from_crate_engine(crate_engine_path)?,
                                settings, additional_car_data
    )
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
        AdditionalAcCarData { engine_weight }
    }

    pub fn default() -> AdditionalAcCarData {
        AdditionalAcCarData { engine_weight: None }
    }

    pub fn engine_weight(&self) -> Option<u32> {
        self.engine_weight
    }
}

pub fn update_ac_engine_parameters(ac_car_path: &Path,
                                   calculator: assetto_corsa::EngineParameterCalculator,
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

#[cfg(test)]
mod tests {
    use std::path::{PathBuf};

    use crate::{automation, beam_ng};
    use crate::beam_ng::get_mod_list;
    use crate::fabricator::assetto_corsa::{EngineParameterCalculator};

    #[test]
    fn load_mods() -> Result<(), String> {
        let mods = get_mod_list();
        let calculator = EngineParameterCalculator::from_beam_ng_mod(mods[0].as_path())?;
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
