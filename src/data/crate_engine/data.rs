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

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use bincode::{Options, serialize_into};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use automation::sandbox::{EngineV1, SandboxVersion};
use crate::data::{AutomationSandboxCrossChecker, crate_engine, CreationOptions};

#[derive(Debug, Deserialize, Serialize)]
pub enum DataVersion {
    V1
}

impl DataVersion {
    pub const VERSION_1_STRING: &'static str = "v1";

    pub fn from_u16(val: u16) -> Result<DataVersion, String> {
        match val {
            1 => Ok(DataVersion::V1),
            _ => Err(format!("Unknown data version {}", val))
        }
    }

    pub fn as_u16(&self) -> u16 {
        match self {
            DataVersion::V1 => 1
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DataVersion::V1 => DataVersion::VERSION_1_STRING
        }
    }
}

impl Display for DataVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.as_str())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataV1 {
    mod_info_json_data: Option<Vec<u8>>,
    main_engine_jbeam_filename: String,
    jbeam_file_data: HashMap<String, Vec<u8>>,
    _car_file_data: Vec<u8>,
    automation_variant_data: EngineV1,
    license_data: Option<Vec<u8>>
}

impl DataV1 {
    pub const VERSION: DataVersion = DataVersion::V1;

    pub fn version(&self) -> DataVersion {
        Self::VERSION
    }

    pub fn from_beamng_mod_zip(mod_path: &Path, options: CreationOptions) -> Result<DataV1, String> {
        let mut mod_data = beam_ng::ModData::from_path(mod_path)?;
        let car_file_data = match mod_data.get_automation_car_file_data() {
            None => return Err("Failed to load .car file from mod. File is missing".to_string()),
            Some(data) => data.clone()
        };
        let automation_car_file = automation::car::CarFile::from_bytes(car_file_data.clone())?;
        let uid = _get_engine_uuid_from_car_file(&automation_car_file)?;
        if uid.len() < 5 {
            return Err(format!("Invalid engine uuid found {}", uid));
        }
        info!("Engine uuid: {}", &uid);
        let expected_engine_data_filename = format!("camso_engine_{}.jbeam", &uid[0..5]);
        info!("Expect to find engine data in {}", &expected_engine_data_filename);

        let mut main_engine_data_file = match mod_data.contains_jbeam_file(&expected_engine_data_filename) {
            false => {
                let legacy_engine_filename = "camso_engine.jbeam".to_string();
                match mod_data.contains_jbeam_file(&legacy_engine_filename) {
                    true => {
                        Some(legacy_engine_filename)
                    },
                    false => {
                        None
                    }
                }
            }
            true => {
                info!("Found main engine data file: {}", &expected_engine_data_filename);
                Some(expected_engine_data_filename)
            }
        };

        if main_engine_data_file.is_none() {
            for name in mod_data.jbeam_filenames() {
                if name.contains("camso_engine_")
                {
                    if !name.contains("structure") &&
                        !name.contains("internals") &&
                        !name.contains("balancing")
                    {
                        main_engine_data_file = Some(name.clone());
                        break;
                    }
                }
            }
        }

        let main_engine_jbeam_filename =
            main_engine_data_file.ok_or("Failed to find the main engine data".to_string())?;
        info!("Found main engine data file: {}", main_engine_jbeam_filename);

        let version = _get_engine_version_from_car_file(&automation_car_file)?;
        info!("Engine version number: {}", version);
        let version = SandboxVersion::from_version_number(version);
        info!("Deduced as {}", version);
        let automation_variant_data = match automation::sandbox::load_engine_by_uuid(&uid, version)? {
            None => {
                return Err(format!("No engine found with uuid {}", uid));
            }
            Some(eng) => { eng }
        };

        if options.xref_mod_with_sandbox {
            AutomationSandboxCrossChecker::new(&automation_car_file, &automation_variant_data).validate().map_err(|err|{
                format!("{}. The BeamNG mod may be out-of-date; try recreating a mod with the latest engine version", err)
            })?;
        }

        let mut mod_info_json_data = match mod_data.get_info_json() {
            Ok(data_str) => {
                Some(data_str.into_bytes())
            }
            Err(err_str) => {
                warn!("Couldn't read info.json from {}. {}", mod_path.display(), &err_str);
                None
            }
        };

        Ok(DataV1 {
            mod_info_json_data,
            main_engine_jbeam_filename,
            jbeam_file_data: mod_data.take_jbeam_file_data(),
            _car_file_data: car_file_data,
            automation_variant_data,
            license_data: mod_data.take_license_data()
        })
    }

    pub fn jbeam_data(&self) -> &HashMap<String, Vec<u8>> {
        &self.jbeam_file_data
    }

    pub fn main_engine_jbeam_data(&self) -> Option<&Vec<u8>> {
        self.jbeam_data().get(&self.main_engine_jbeam_filename)
    }

    pub fn automation_data(&self) -> &EngineV1 {
        &self.automation_variant_data
    }

    pub fn automation_data_hash(&self) -> Option<[u8; 32]> {
        let mut auto_hasher = Sha256::new();
        auto_hasher.update(&self.automation_variant_data.family_data_checksum_data());
        auto_hasher.update(&self.automation_variant_data.variant_data_checksum_data());
        auto_hasher.update(&self.automation_variant_data.result_data_checksum_data());
        create_sha256_hash_array(auto_hasher)
    }

    pub fn jbeam_data_hash(&self) -> Option<[u8; 32]> {
        let mut jbeam_hasher = Sha256::new();
        jbeam_hasher.update(&self.main_engine_jbeam_data()?);
        create_sha256_hash_array(jbeam_hasher)
    }

    pub fn car_file_data(&self) -> &Vec<u8> {
        &self._car_file_data
    }

    pub fn from_eng_file(file_path: &Path) -> bincode::Result<DataV1> {
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let config = bincode::options().with_limit(104857600);
        config.deserialize(&buffer)
    }

    pub fn write_to_file(&self, file: &mut File) -> bincode::Result<()> {
        serialize_into(file, &self)
    }
}

fn _is_legacy_main_engine_data_file(filename: &str) -> bool {
    if filename.ends_with("camso_engine.jbeam") {
        return true;
    }
    if filename.contains("camso_engine_")
    {
        if !filename.contains("structure") &&
            !filename.contains("internals") &&
            !filename.contains("balancing")
        {
            return true;
        }
    }
    false
}

fn _get_variant_section_from_car_file(automation_car_file: &automation::car::CarFile)
                                      -> Result<&automation::car::Section, String>
{
    Ok(automation_car_file
        .get_section("Car").ok_or("Failed to find Car section in .car file".to_string())?
        .get_section("Variant").ok_or("Failed to find Car.Variant section in .car file".to_string())?)
}

fn _get_engine_version_from_car_file(automation_car_file: &automation::car::CarFile) -> Result<u64, String> {
    let variant_info = _get_variant_section_from_car_file(automation_car_file)?;
    let version_num = variant_info.get_attribute("GameVersion").unwrap().value.as_num().unwrap();
    Ok(version_num as u64)
}

fn _get_engine_uuid_from_car_file(automation_car_file: &automation::car::CarFile) -> Result<String, String> {
    let variant_info = _get_variant_section_from_car_file(automation_car_file)?;
    let uid =
        variant_info.get_attribute("UID").ok_or("No UID in Car.Variant section".to_string())?.value.as_str();
    Ok(uid.to_string())
}

fn create_sha256_hash_array(hasher: impl Digest) -> Option<[u8; 32]> {
    let hash: Vec<u8> = hasher.finalize().iter().map(|b| *b).collect();
    match <[u8; 32]>::try_from(hash) {
        Ok(hash_array) => Some(hash_array),
        Err(_) => {
            None
        }
    }
}
