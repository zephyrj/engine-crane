/*
 * Copyright (c):
 * 2023 zephyrj
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
mod validation;

use std::collections::HashMap;
use std::{fs, io};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use automation::sandbox::{EngineV1, load_engine_by_uuid, SandboxVersion};

use bincode::{deserialize, serialize, serialize_into};
use serde::{Deserialize, Serialize};
use serde_hjson::{Map, Value};
use tracing::{debug, error, info, warn};
use tracing_subscriber::fmt::format;
use beam_ng::jbeam;
pub use crate::data::validation::AutomationSandboxCrossChecker;


#[derive(Debug, Deserialize, Serialize)]
pub enum CrateEngineVersion {
    V1
}

impl CrateEngineVersion {
    pub const VERSION_1_STRING: &'static str = "v1";

    pub fn as_str(&self) -> &'static str {
        match self {
            CrateEngineVersion::V1 => CrateEngineVersion::VERSION_1_STRING
        }
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


#[derive(Debug, Deserialize, Serialize)]
pub struct CrateEngine {
    version: CrateEngineVersion,
    name: String,
    mod_info_json_data: Option<Vec<u8>>,
    main_engine_jbeam_filename: String,
    jbeam_file_data: HashMap<String, Vec<u8>>,
    car_file_data: Vec<u8>,
    automation_variant_data: EngineV1,
    license_data: Option<Vec<u8>>
}

impl CrateEngine {
    pub fn from_beamng_mod_zip(mod_path: &Path, options: CreationOptions) -> Result<CrateEngine, String> {
        info!("Opening {}", mod_path.display());
        let zipfile = fs::File::open(mod_path).map_err(|err| {
            format!("Failed to open {}. {}", mod_path.display(), err.to_string())
        })?;

        info!("Extracting {}", mod_path.display());
        let mut archive = zip::ZipArchive::new(zipfile).map_err(|err| {
            format!("Failed to read archive {}. {}", mod_path.display(), err.to_string())
        })?;

        let mut info_json_path = None;
        let mut license_data_path = None;
        let mut car_data_path = String::new();
        let mut jbeam_file_list = Vec::new();
        for file_path in archive.file_names() {
            if file_path.ends_with("info.json") {
                info_json_path = Some(String::from(file_path));
            }
            else if file_path.ends_with(".car") {
                car_data_path = String::from(file_path);
            }
            else if file_path.ends_with(".jbeam") {
                jbeam_file_list.push(String::from(file_path));
            }
            else if file_path.ends_with("license.txt") {
                license_data_path = Some(String::from(file_path));
            }
        }

        let car_file_data = match _extract_file_data_from_archive(&mut archive, &car_data_path) {
            Ok(car_data) => { car_data }
            Err(err_str) => {
                error!("Couldn't extract {} from {}. {}",
                            &car_data_path,
                            mod_path.display(),
                            &err_str);
                return Err(format!("Failed to load .car file from mod. {}", err_str));
            }
        };
        let automation_car_file = automation::car::CarFile::from_bytes(car_file_data.clone())?;
        let uid = _get_engine_uuid_from_car_file(&automation_car_file)?;
        if uid.len() < 5 {
            return Err(format!("Invalid engine uuid found {}", uid));
        }
        info!("Engine uuid: {}", &uid);
        let expected_engine_data_filename = format!("camso_engine_{}.jbeam", &uid[0..5]);
        info!("Expect to find engine data in {}", &expected_engine_data_filename);

        let mut jbeam_file_data: HashMap<String, Vec<u8>> = HashMap::new();
        let mut main_engine_data_file = None;
        for name in jbeam_file_list {
            if main_engine_data_file.is_none() {
                if name.ends_with(&expected_engine_data_filename) {
                    main_engine_data_file = Some(name.clone());
                } else if _is_legacy_main_engine_data_file(&name) {
                    main_engine_data_file = Some(name.clone());
                }
            }
            match _extract_file_data_from_archive(&mut archive, &name) {
                Ok(data) => {
                    jbeam_file_data.insert(name, data);
                }
                Err(err_str) => {
                    if let Some(main_file) = &main_engine_data_file {
                        let err_str =
                            format!("Failed to main engine data from {}. {}", main_file, err_str);
                        error!("{}", &err_str);
                        return Err(err_str);
                    } else {
                        warn!("Failed to load data from {}. {}", name, err_str);
                    }
                }
            }
        }
        let main_engine_jbeam_filename =
            main_engine_data_file.ok_or("Failed to find the main engine data".to_string())?;
        let engine_data = jbeam_file_data.get(&main_engine_jbeam_filename).unwrap();
        let name = _get_name_from_jbeam_data(engine_data).unwrap_or_else(|| {
            let m = mod_path.file_name().unwrap_or("unknown".as_ref()).to_str().unwrap_or("unknown");
            m.strip_suffix(".zip").unwrap_or(m).to_string()
        });

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

        let mut mod_info_json_data = None;
        if let Some(name) = info_json_path {
            mod_info_json_data = match _extract_file_data_from_archive(&mut archive, &name) {
                Ok(data) => {
                    Some(data)
                }
                Err(err_str) => {
                    warn!("Couldn't extract {} from {}. {}", &name,mod_path.display(), &err_str);
                    None
                }
            };
        }
        let mut license_data = None;
        if let Some(name) = license_data_path {
            license_data = match _extract_file_data_from_archive(&mut archive, &name) {
                Ok(data) => {
                    Some(data)
                }
                Err(err_str) => {
                    warn!("Couldn't extract {} from {}. {}", &name,mod_path.display(), &err_str);
                    None
                }
            };
        }

        Ok(CrateEngine {
            version: CrateEngineVersion::V1,
            name,
            mod_info_json_data,
            main_engine_jbeam_filename,
            jbeam_file_data,
            car_file_data,
            automation_variant_data,
            license_data
        })
    }

    pub fn from_eng_file(file_path: &Path) -> bincode::Result<CrateEngine> {
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        deserialize(&buffer)
    }

    pub fn name(&self) -> &str {
        &self.name
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

fn _get_engine_uuid_from_car_file(automation_car_file: &automation::car::CarFile) -> Result<String, String> {
    let variant_info = _get_variant_section_from_car_file(automation_car_file)?;
    let uid =
        variant_info.get_attribute("UID").ok_or("No UID in Car.Variant section".to_string())?.value.as_str();
    Ok(uid.to_string())
}

fn _get_engine_version_from_car_file(automation_car_file: &automation::car::CarFile) -> Result<u64, String> {
    let variant_info = _get_variant_section_from_car_file(automation_car_file)?;
    let version_num = variant_info.get_attribute("GameVersion").unwrap().value.as_num().unwrap();
    Ok(version_num as u64)
}

fn _extract_file_data_from_archive(archive: &mut zip::ZipArchive<fs::File>,
                                   file_path: &str)
                                   -> Result<Vec<u8>, String>
{
    let mut data: Vec<u8> = Vec::new();
    match archive.by_name(file_path) {
        Ok(mut file) => {
            debug!("Found engine data at {}", file_path);
            file.read_to_end(&mut data).map_err(|e|{
                format!("Read to end of {} failed. {}", file_path, e.to_string())
            })?;
            Ok(data)
        },
        Err(err) => {
            return Err(format!("Failed to read {}. {}", file_path, err.to_string()));
        }
    }
}

fn _extract_jbeam_data_from_archive(archive: &mut zip::ZipArchive<fs::File>,
                                    file_path: &str) -> Result<Map<String, Value>, String> {
    let jbeam_data: Vec<u8> = _extract_file_data_from_archive(archive, file_path)?;
    jbeam::from_slice(&*jbeam_data).map_err(|e| {
        return e.to_string();
    })
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
    println!("Loaded {}", eng.name());
    let mut file = File::create(format!("{}.eng", eng.name())).expect("Failed to open file");
    match eng.write_to_file(&mut file) {
        Ok(_) => Ok(()),
        Err(e) => {
            Err(e.to_string())
        }
    }
}