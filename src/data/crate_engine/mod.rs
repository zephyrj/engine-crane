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
use bincode::{deserialize_from, Options, serialize_into};
use serde::{Deserialize, Serialize};
use tracing::{warn};
use automation::sandbox::{EngineV1};
use beam_ng::jbeam;
use metadata::MetadataV1;
use crate::data::crate_engine::data::{DataV1, DataVersion};

pub use metadata::{CrateEngineMetadata};


pub(crate) const CRATE_ENGINE_FILE_SUFFIX: &'static str = "eng";

type CurrentMetadataType = MetadataV1;
type CurrentDataType = DataV1;


pub struct CrateEngine {
    metadata: CrateEngineMetadata,
    data: CrateEngineData
}

impl CrateEngine {
    pub fn from_beamng_mod_zip(mod_path: &Path, options: CreationOptions) -> Result<CrateEngine, String> {
        let data = DataV1::from_beamng_mod_zip(mod_path, options)?;
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
        let metadata = CurrentMetadataType {
            data_version: CurrentDataType::VERSION.as_u16(),
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
            data: CrateEngineData::from_current_version(data)
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

    pub fn version(&self) -> DataVersion {
        self.metadata.data_version().unwrap()
    }

    pub fn get_automation_car_file_data(&self) -> &Vec<u8> {
        self.data.get_automation_car_file_data()
    }

    pub fn get_automation_engine_data(&self) -> &EngineV1 {
        self.data.get_automation_engine_data()
    }

    pub fn get_engine_jbeam_data(&self) -> Option<&Vec<u8>> {
        self.data.get_engine_jbeam_data()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreationOptions {
    pub xref_mod_with_sandbox: bool
}

impl CreationOptions {
    pub fn default() -> CreationOptions {
        CreationOptions { xref_mod_with_sandbox: true }
    }
}

pub enum CrateEngineData {
    DataV1(DataV1)
}

impl CrateEngineData {
    fn from_current_version(inner_type: CurrentDataType) -> CrateEngineData {
        return CrateEngineData::DataV1(inner_type)
    }

    pub fn from_reader(metadata: &CrateEngineMetadata, reader: &mut impl Read) -> Result<CrateEngineData, String> {
        match metadata.data_version()? {
            DataVersion::V1 => {
                let internal_data =
                    deserialize_from(reader).map_err(|e| {
                        format!("Failed to deserialise {} crate engine. {}", DataVersion::V1, e.to_string())
                    })?;
                Ok(CrateEngineData::DataV1(internal_data))
            }
        }
    }

    pub fn serialize_into(&self, writer: &mut impl Write) -> bincode::Result<()> {
        match self {
            CrateEngineData::DataV1(d) => {
                serialize_into(writer, d)
            }
        }
    }

    pub fn get_automation_car_file_data(&self) -> &Vec<u8> {
        match self {
            CrateEngineData::DataV1(d) => {
                &d.car_file_data()
            }
        }
    }

    pub fn get_automation_engine_data(&self) -> &EngineV1 {
        match self {
            CrateEngineData::DataV1(d) => {
                &d.automation_data()
            }
        }
    }

    pub fn get_engine_jbeam_data(&self) -> Option<&Vec<u8>> {
        match self {
            CrateEngineData::DataV1(d) => {
                d.main_engine_jbeam_data()
            }
        }
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
    let eng = CrateEngine::from_beamng_mod_zip(&path, CreationOptions::default())?;
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

    {
        let calc = crate::fabricator::AcEngineParameterCalculatorV1::from_crate_engine(&crate_path)?;
        println!("Limiter {}", calc.limiter());
        println!("Torque {}", calc.peak_torque());
        println!("BHP {}", calc.peak_bhp());
    }
    Ok(())
}

