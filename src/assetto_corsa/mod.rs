mod error;
pub mod car;
mod engine;
mod file_utils;
mod lut_utils;
mod ini_utils;
mod drivetrain;
mod traits;

use std::collections::HashMap;
use std::fs;
use std::ffi::OsString;
use std::fmt::format;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::PathBuf;
use iced::keyboard::KeyCode::V;
use crate::assetto_corsa::error::{Result, Error, ErrorKind};

use crate::steam;

pub const STEAM_GAME_NAME: &str = "assettocorsa";
pub const STEAM_GAME_ID: i64 = 244210;

pub fn is_installed() -> bool {
    if let Some(install_path) = steam::get_game_install_path(STEAM_GAME_NAME) {
        install_path.is_dir()
    } else {
        false
    }
}

pub fn get_installed_cars_path() -> Option<PathBuf> {
    if let Some(mut install_path) = steam::get_game_install_path(STEAM_GAME_NAME) {
        for path in ["content", "cars"] {
            install_path.push(path)
        }
        Some(install_path)
    } else {
        None
    }
}

pub fn get_root_sfx_path() -> Result<PathBuf> {
    return match steam::get_game_install_path(STEAM_GAME_NAME) {
        Some(mut path) => {
            for dir in ["content", "sfx"] {
                path.push(dir)
            }
            return Ok(path);
        }
        None => {
            Err(Error::new(ErrorKind::NotInstalled,
                           String::from(
                               format!("Assetto Corsa doesn't appear to be installed"))))
        }
    }
}

pub fn get_list_of_installed_cars() -> Result<Vec<OsString>> {
    let car_dir = match get_installed_cars_path() {
        Some(path) => path,
        None => return Err(Error::new(ErrorKind::NotInstalled,
                                      String::from("Assetto Corsa isn't installed")))
    };
    let dir_entries = match fs::read_dir(car_dir) {
        Ok(entry_list) => entry_list,
        Err(e) => return Err(Error::new(ErrorKind::NotInstalled,
                                        String::from(
                                            format!("Assetto Corsa doesn't appear to be installed: {}",
                                                    e.to_string()))))
    };

    let cars: Vec<OsString> = dir_entries.filter_map(|e| {
        match e {
            Ok(dir_entry) => {
                if dir_entry.path().is_dir() {
                    Some(dir_entry.path().into_os_string())
                } else {
                    None
                }
            },
            _ => None
        }
    }).collect();
    Ok(cars)
}

pub struct SfxData {
    sfx_by_folder_map: HashMap<String, Vec<String>>,
    sfx_bank_map: HashMap<String, String>
}

impl SfxData {
    pub fn generate_clone_guid_info(&self, existing_car_name: &str, new_car_name: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if self.sfx_bank_map.contains_key(existing_car_name) {
            out.push(format!("{} bank:/{}",
                                   self.sfx_bank_map.get(existing_car_name).unwrap(),
                                   new_car_name));
            for entry in self.sfx_by_folder_map.get(existing_car_name).unwrap() {
                out.push(entry.replace(existing_car_name, new_car_name));
            }
        }
        out
    }
}

pub fn load_sfx_data() -> Result<SfxData> {
    let sfx_guid_file_path = get_root_sfx_path()?.join("GUIDs.txt");
    let file = File::open(&sfx_guid_file_path).map_err(|err|{
        Error::new(ErrorKind::NotInstalled,
                   String::from(
                       format!("Couldn't open {}. {}", sfx_guid_file_path.display(), err.to_string())))
    })?;

    let mut sfx_data = SfxData {
        sfx_by_folder_map: HashMap::new(),
        sfx_bank_map: HashMap::new()
    };
    for line_res in BufReader::new(file).lines() {
        match line_res {
            Ok(line) => {
                let line_data: Vec<_> = line.split_whitespace().collect();
                let guid = line_data[0];
                let sfx_line = line_data[1];
                if sfx_line.starts_with("event") {
                    let temp: Vec<_> = sfx_line.split(":").collect::<Vec<_>>()[1].split("/").collect();
                    let folder_name = temp[2];
                    if !sfx_data.sfx_by_folder_map.contains_key(folder_name) {
                        sfx_data.sfx_by_folder_map.insert(String::from(folder_name), Vec::new());
                    }
                    sfx_data.sfx_by_folder_map.get_mut(folder_name).unwrap().push(line)
                } else if sfx_line.starts_with("bank") {
                    sfx_data.sfx_bank_map.insert(String::from(sfx_line.split("/").collect::<Vec<_>>()[1]),
                                                 String::from(guid));
                }
            }
            Err(_) => { continue }
        }
    }
    Ok(sfx_data)
}

