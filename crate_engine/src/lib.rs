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

pub mod metadata;
pub mod source;
mod data;

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::{warn};
use unwrap_infallible::UnwrapInfallible;
use beam_ng::jbeam;

pub use metadata::CrateEngineMetadata;
pub use data::CrateEngineData;
pub use data::beam_ng_mod;
pub use data::direct_export;

pub type FromBeamNGModOptions = beam_ng_mod::CreationOptions;


pub const CRATE_ENGINE_FILE_SUFFIX: &'static str = "eng";


pub struct CrateEngine {
    metadata: CrateEngineMetadata,
    data: CrateEngineData
}

impl CrateEngine {
    pub fn from_beamng_mod_zip(mod_path: &Path, options: FromBeamNGModOptions) -> Result<CrateEngine, String> {
        let crate_data = CrateEngineData::from_beamng_mod_zip(mod_path, options)?;
        let data = match crate_data {
            CrateEngineData::BeamNGMod(ref d) => match d {
                beam_ng_mod::Data::V1(d) => d
            }
            _ => return Err("Should have created crate engine from beamng data".to_string())
        };
        let engine_data = match data.main_engine_jbeam_data() {
            None => return Err("Missing main engine jbeam data".to_string()),
            Some(data) => data
        };
        let name =
            _get_name_from_jbeam_data(engine_data).unwrap_or_else(
                || {
                    let m = mod_path.file_name().unwrap_or("unknown".as_ref()).to_str().unwrap_or("unknown");
                    m.strip_suffix(".zip").unwrap_or(m).to_string()
                });

        let automation_data_hash = data.automation_data_hash();
        if automation_data_hash.is_none() {
            warn!("Failed to calculate automation data hash");
        }

        let engine_jbeam_hash = data.jbeam_data_hash();
        if engine_jbeam_hash.is_none() {
            warn!("Failed to calculate engine jbeam data hash");
        }

        let fuel = match data.automation_data().fuel_type.as_ref() {
            None => "Unknown".to_string(),
            Some(f) => f.clone()
        };
        let metadata = metadata::CurrentMetadataType {
            data_version: crate_data.version_int(),
            automation_version: data.automation_data().variant_version,
            name,
            automation_data_hash,
            engine_jbeam_hash,
            build_year: data.automation_data().get_variant_build_year(),
            block_config: data.automation_data().get_block_config(),
            head_config: data.automation_data().get_head_config(),
            valves: data.automation_data().get_valve_type(),
            capacity: data.automation_data().get_capacity_cc(),
            aspiration: data.automation_data().get_aspiration(),
            fuel,
            peak_power: data.automation_data().peak_power.round() as u32,
            peak_power_rpm: data.automation_data().peak_power_rpm.round() as u32,
            peak_torque: data.automation_data().peak_torque.round() as u32,
            peak_torque_rpm: data.automation_data().peak_torque_rpm.round() as u32,
            max_rpm: data.automation_data().max_rpm.round() as u32
        };

        Ok(CrateEngine{
            metadata: CrateEngineMetadata::from_current_version(metadata),
            data: crate_data
        })
    }

    // TODO!
    pub fn from_exporter_data(data_type: direct_export::Data) -> Result<CrateEngine, String> {
        let metadata = match &data_type {
            direct_export::Data::V1(data) => {
                let automation_version = data.float_data["Info"]["GameVersion"].round() as u64;
                let name = format!("{} {}", data.string_data["Info"]["FamilyName"], data.string_data["Info"]["VariantName"]);
                let build_year = data.float_data["Info"]["VariantYear"].round() as u16;
                let block_config = automation::BlockConfig::from_str(&data.string_data["Parts"]["BlockType"]).unwrap_infallible();
                let head_config = automation::HeadConfig::from_str(&data.string_data["Parts"]["HeadType"]).unwrap_infallible();
                let valves = automation::Valves::from_int(
                    (data.float_data["Parts"]["IntakeValves"].round() + data.float_data["Parts"]["ExhaustValves"].round()) as u16
                ).unwrap_infallible();
                let capacity = (data.float_data["Tune"]["Displacement"] * 1000.0).round() as u32;
                let aspiration = automation::AspirationType::from_str(&data.string_data["Parts"]["Aspiration"]).unwrap_infallible();

                metadata::MetadataV2 {
                    source: source::DataSource::from_direct_export(),
                    data_version: data.version_int(),
                    automation_version,
                    name,
                    build_year,
                    block_config,
                    head_config,
                    valves,
                    capacity,
                    aspiration,
                    fuel: data.string_data["Fuel"]["Type"].clone(),
                    peak_power: data.float_data["Results"]["PeakPower"].round() as u32,
                    peak_power_rpm: data.float_data["Results"]["PeakPowerRPM"].round() as u32,
                    peak_torque: data.float_data["Results"]["PeakTorque"].round() as u32,
                    peak_torque_rpm: data.float_data["Results"]["PeakTorqueRPM"].round() as u32,
                    max_rpm: data.float_data["Results"]["MaxRPM"].round() as u32
                }
            }
        };
        Ok(CrateEngine{
            metadata: CrateEngineMetadata::MetadataV2(metadata),
            data: CrateEngineData::DirectExport(data_type)
        })
    }

    pub fn deserialize_from(reader: &mut impl Read) -> Result<CrateEngine, String> {
        let metadata = CrateEngineMetadata::from_reader(reader)?;
        let data = CrateEngineData::from_reader(&metadata, reader)?;
        Ok(CrateEngine { metadata, data })
    }

    pub fn serialize_to(&self, writer: &mut impl Write) -> bincode::Result<()> {
        self.metadata.serialize_into(writer)?;
        self.data.serialize_into(writer)
    }

    pub fn name(&self) -> &str {
        self.metadata.name()
    }

    pub fn version(&self) -> u16 {
        self.metadata.data_version()
    }

    pub fn data(&self) -> &CrateEngineData {
        &self.data
    }

    pub fn write_to_path(&self, path: PathBuf) -> bincode::Result<PathBuf> {
        if !path.is_dir() {
            return Err(bincode::Error::from(
                bincode::ErrorKind::Custom(format!("Output path {} not found", path.display()))
            ))
        }
        let mut sanitized_name = sanitize_filename::sanitize(self.name());
        sanitized_name = sanitized_name.replace(" ", "_");
        let mut crate_path = path.join(format!("{}.eng", sanitized_name));
        let mut extra_num = 2;
        while crate_path.is_file() {
            crate_path = path.join(format!("{}{}.eng", sanitized_name, extra_num));
            extra_num += 1;
        }
        let mut f = File::create(&crate_path)?;
        self.serialize_to(&mut f)?;
        Ok(crate_path)
    }
}

fn _get_name_from_jbeam_data(engine_data: &Vec<u8>) -> Option<String> {
    let data_map = match jbeam::from_slice(&*engine_data) {
        Ok(d) => d,
        Err(_) => { return None; }
    };

    let mut engine_key = String::from("Camso_Engine");
    let test_key = String::from(engine_key.clone() + "_");
    for key in data_map.keys() {
        if key.starts_with(&test_key) {
            engine_key = String::from(key);
            break;
        }
    }
    let eng_info = data_map.get(&engine_key)?.as_object()?.get("information")?.as_object()?;
    Some(eng_info.get("name")?.as_str()?.to_string())
}


#[test]
fn create_crate_engine() -> Result<(), String> {
    let path = PathBuf::from("C:/Users/zephy/AppData/Local/BeamNG.drive/mods/dawnv6.zip");
    let eng = CrateEngine::from_beamng_mod_zip(&path, data::beam_ng_mod::CreationOptions::default())?;
    println!("Loaded {} from mod", eng.name());
    let crate_path = PathBuf::from(format!("{}.eng", eng.name()));
    {
        let mut file = File::create(format!("{}.eng", eng.name())).expect("Failed to open file");
        match eng.serialize_to(&mut file) {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(e.to_string())
            }
        }?
    }
    Ok(())
}

