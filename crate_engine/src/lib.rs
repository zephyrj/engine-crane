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

mod metadata;
mod source;
mod data;

use std::fmt::{Display};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use bincode::{Options};
use serde::{Deserialize, Serialize};
use tracing::{warn};
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
        let engine_data = data.main_engine_jbeam_data().unwrap();
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

