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

pub mod jbeam;

use std::collections::hash_map::Keys;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use serde_hjson::{Map, Value};
use steam;
use tracing::{debug, info, warn};

#[cfg(target_os = "windows")]
use {
    directories::BaseDirs,
    parselnk::Lnk
};


pub const STEAM_GAME_NAME: &str = "BeamNG.drive";
pub const STEAM_GAME_ID: i64 = 284160;
pub const AUTOMATION_STEAM_GAME_ID: i64 = 293760;

#[cfg(target_os = "windows")]
pub fn get_default_mod_path() -> PathBuf {
    let mut mod_path_buf : PathBuf = match BaseDirs::new() {
        None => {
            let username = whoami::username();
            PathBuf::from_iter(["C:", "Users", &username, "AppData", "Local"])
        }
        Some(basedirs) => { basedirs.cache_dir().to_path_buf() }
    };
    mod_path_buf.push(STEAM_GAME_NAME);
    let beamng_installed = steam::get_game_install_path(STEAM_GAME_NAME).is_dir();
    match beamng_installed {
        true => {
            let mut link_path = mod_path_buf.clone();
            link_path.push("latest.lnk");
            match Lnk::try_from(link_path.as_path()) {
                Ok(lnk) => {
                    if let Some(target_path) = lnk.link_info.local_base_path {
                        mod_path_buf = PathBuf::from(target_path);
                    }
                }
                Err(_) => {}
            }
        }
        false => {}
    }
    mod_path_buf.push("mods");
    mod_path_buf
}

#[cfg(target_os = "linux")]
pub fn get_default_mod_path() -> PathBuf {
    let mut mod_path_buf: PathBuf = steam::get_wine_prefix_dir(AUTOMATION_STEAM_GAME_ID);
    for path in ["users", "steamuser", "AppData", "Local", "BeamNG.drive", "mods"] {
        mod_path_buf.push(path);
    }
    mod_path_buf
}

pub fn get_mod_list_in(path: &PathBuf) -> Vec<PathBuf> {
    info!("Looking for BeamNG mods in {}", path.display());
    read_mods_in_path(&path)
}

pub fn get_mod_list() -> Vec<PathBuf> {
    let mod_dir = get_default_mod_path();
    return match mod_dir.is_dir() {
        true => {
            info!("Looking for BeamNG mods in {}", mod_dir.display());
            read_mods_in_path(&mod_dir)
        }
        false => {
            warn!("The provided BeamNG mod path {} does not exist", mod_dir.display());
            Vec::new()
        }
    }
}

fn read_mods_in_path(path: &PathBuf) -> Vec<PathBuf> {
    let dir_entries = match fs::read_dir(path) {
        Ok(entry_list) => entry_list,
        Err(_e) => { return Vec::new(); }
    };

    dir_entries.filter_map(|e| {
        match e {
            Ok(dir_entry) => {
                if dir_entry.path().is_file() {
                    match dir_entry.path().extension() {
                        Some(ext) => {
                            if ext.ne("zip") {
                                return None
                            }
                        },
                        None => return None
                    }
                    Some(dir_entry.path())
                } else {
                    None
                }
            },
            _ => None
        }
    }).collect()
}

#[derive(Debug)]
pub struct ModData {
    info_json: serde_json::Map<String, serde_json::Value>,
    jbeam_file_data: HashMap<String, Vec<u8>>,
    car_file_data: Option<Vec<u8>>,
    license_data: Option<Vec<u8>>,
    archive_data: zip::ZipArchive<File>
}

impl ModData {
    pub fn from_path(mod_path: &Path) -> Result<ModData, String> {
        info!("Opening {}", mod_path.display());
        let zipfile = fs::File::open(mod_path).map_err(|err| {
            format!("Failed to open {}. {}", mod_path.display(), err.to_string())
        })?;

        info!("Extracting {}", mod_path.display());
        let mut archive = zip::ZipArchive::new(zipfile).map_err(|err| {
            format!("Failed to read archive {}. {}", mod_path.display(), err.to_string())
        })?;

        let mut info_json_path = String::new();
        let mut license_data_path = None;
        let mut car_data_path: Option<String> = None;
        let mut jbeam_file_list = Vec::new();
        for file_path in archive.file_names() {
            if file_path.ends_with(".jbeam") {
                jbeam_file_list.push(String::from(file_path));
            }
            else if file_path.ends_with(".car") {
                car_data_path = Some(String::from(file_path));
            }
            else  if file_path.ends_with("info.json") {
                info_json_path = String::from(file_path);
            }
            else if file_path.ends_with("license.txt") {
                license_data_path = Some(String::from(file_path));
            }
        }

        let info_json = _extract_json_data_from_archive(&mut archive, &info_json_path)?;

        let mut jbeam_file_data = HashMap::new();
        for file_path in jbeam_file_list {
            match _extract_file_data_from_archive(&mut archive, &file_path) {
                Ok(data) => {
                    let filename = match PathBuf::from(&file_path).file_name() {
                        None => {
                            warn!("Couldn't get filename of {}", file_path);
                            file_path
                        },
                        Some(p) => p.to_string_lossy().to_string()
                    };
                    jbeam_file_data.insert(filename, data);
                }
                Err(e) => {
                    warn!("Couldn't extract {} from {}. {}", file_path, mod_path.display(), e.to_string());
                }
            }
        }

        let mut car_file_data = None;
        if let Some(path) = &car_data_path {
            car_file_data = match _extract_file_data_from_archive(&mut archive, path) {
                Ok(car_data) => { Some(car_data) }
                Err(_) => {
                    info!("No .car file found in {}", mod_path.display());
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
                    warn!("Couldn't extract license data from {}. {}",mod_path.display(), &err_str);
                    None
                }
            };
        }

        Ok(ModData{
            info_json,
            car_file_data,
            jbeam_file_data,
            license_data,
            archive_data: archive
        })
    }

    pub fn get_automation_car_file_data(&self) -> Option<&Vec<u8>> {
        match &self.car_file_data {
            None => { None }
            Some(data) => { Some(data) }
        }
    }

    pub fn get_info_json_map(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.info_json
    }

    pub fn get_info_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.info_json)
    }

    pub fn take_license_data(&mut self) -> Option<Vec<u8>>{
        std::mem::take(&mut self.license_data)
    }

    pub fn jbeam_filenames(&self) -> Keys<'_, String, Vec<u8>> {
        self.jbeam_file_data.keys()
    }

    pub fn get_jbeam_file_data(&self, filename: &str) -> Option<&Vec<u8>> {
        self.jbeam_file_data.get(filename)
    }

    pub fn contains_jbeam_file(&self, filename: &str) -> bool {
        self.jbeam_file_data.contains_key(filename)
    }

    pub fn take_jbeam_file_data(&mut self) -> HashMap<String, Vec<u8>> {
        std::mem::take(&mut self.jbeam_file_data)
    }

    pub fn get_engine_jbeam_data(&mut self, expected_eng_key: Option<&str>) -> Result<Map<String, Value>, String> {
        let mut expected_filename: Option<String> = None;
        let mut found_filename : Option<String> = None;
        if let Some(key) = expected_eng_key {
            expected_filename = Some(format!("camso_engine_{}.jbeam", &key));
            info!("Expect to find engine data in {}", expected_filename.as_ref().unwrap());
        }

        for filename in self.archive_data.file_names() {
            if let Some(expected_name) = &expected_filename {
                if filename.ends_with(expected_name) {
                    found_filename = Some(filename.to_string());
                    info!("Found expected engine.jbeam file at {}", filename);
                    break;
                }
            }
            if filename.ends_with("camso_engine.jbeam") {
                found_filename = Some(filename.to_string());
                info!("Found legacy engine.jbeam file at {}", filename);
                break;
            }
        }

        if found_filename.is_none() {
            for filename in self.archive_data.file_names() {
                if filename.contains("camso_engine_") {
                    if !filename.contains("structure") &&
                        !filename.contains("internals") &&
                        !filename.contains("balancing") {
                        found_filename = Some(filename.to_string());
                        info!("Found engine.jbeam file at {}", filename);
                        break;
                    }
                }
            }
        }

        if let Some(name) = found_filename {
            return match _extract_jbeam_data_from_archive(&mut self.archive_data, &name) {
                Ok(jbeam_map) => {
                    Ok(jbeam_map)
                }
                Err(e) => {
                    Err(format!("Failed to read {}. {}", &name, &e))
                }
            }
        }
        Err("Couldn't find engine file".to_string())
    }
}

pub fn load_mod_data(mod_name: &str) -> Result<ModData, String> {
    let mod_path = get_default_mod_path();
    let mod_path = match mod_path.is_dir() {
        false => { return Err(String::from("Cannot find Beam.NG mods path")); }
        true => { mod_path.join(mod_name) }
    };
    ModData::from_path(mod_path.as_path())
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

fn _extract_json_data_from_archive(archive: &mut zip::ZipArchive<fs::File>,
                                   file_path: &str) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let file_data: Vec<u8> = _extract_file_data_from_archive(archive, file_path)?;
    serde_json::from_slice(&*file_data).map_err(|e| {
        return e.to_string();
    })
}


mod tests {
    // use std::path::PathBuf;
    // use crate::{get_default_mod_path, get_mod_list, load_mod_data};

    #[test]
    fn get_beam_ng_mod_path() -> Result<(), String> {
        let path = PathBuf::from(get_default_mod_path());
        println!("BeamNG mod path is {}", path.display());
        Ok(())
    }

    #[test]
    fn get_beam_ng_mod_list() -> Result<(), String> {
        let path = PathBuf::from(get_default_mod_path());
        let mods = get_mod_list();
        if mods.len() == 0 {
            println!("No mods found in {}", path.display());
        } else {
            for p in mods {
                println!("{}", p.display())
            }
        }
        Ok(())
    }

    #[test]
    fn load_beam_ng_mod() -> Result<(), String> {
        let _mod_data = load_mod_data("turbo_boy_modifed.zip")?;
        Ok(())
    }
}
