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

pub mod error;
pub mod traits;
pub mod file_utils;
pub mod ini_utils;
pub mod car;
pub use car::Car;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{PathBuf};
use tracing::info;
use crate::error::{Error, ErrorKind, Result};
use steam;

pub const STEAM_GAME_NAME: &str = "assettocorsa";
pub const STEAM_GAME_ID: i64 = 244210;

pub fn get_default_install_path() -> PathBuf {
    steam::get_game_install_path(STEAM_GAME_NAME)
}

pub fn get_default_installed_cars_path() -> PathBuf {
    let mut install_path = steam::get_game_install_path(STEAM_GAME_NAME);
    for path in ["content", "cars"] {
        install_path.push(path)
    }
    install_path
}

pub struct Installation {
    base_path: PathBuf
}

impl Installation {
    pub fn new() -> Installation {
        Installation{base_path: get_default_install_path()}
    }

    pub fn from_path(path: PathBuf) -> Installation {
        Installation{base_path: path}
    }

    pub fn is_installed(&self) -> bool {
        self.base_path.is_dir()
    }

    pub fn get_installed_car_path(&self) -> PathBuf {
        (&self.base_path).join(PathBuf::from_iter(["content", "cars"]))
    }

    pub fn get_list_of_installed_cars(&self) -> Result<Vec<PathBuf>> {
        let car_path = self.get_installed_car_path();
        return match car_path.is_dir() {
            true => {
                info!("AC cars directory is {}", car_path.display());
                read_cars_in_path(&car_path)
            }
            false => {
                Err(Error::new(ErrorKind::NotInstalled,
                               String::from("Assetto Corsa isn't installed")))
            }
        }
    }

    pub fn get_root_sfx_path(&self) -> Result<PathBuf> {
        let mut path = self.base_path.clone();
        for dir in ["content", "sfx"] {
            path.push(dir)
        }
        return match path.is_dir() {
            true => { Ok(path) }
            false => {
                Err(Error::new(ErrorKind::NotInstalled,
                               String::from(
                                   format!("Assetto Corsa doesn't appear to be installed"))))
            }
        }
    }

    pub fn load_sfx_data(&self) -> Result<SfxData> {
        let sfx_guid_file_path = self.get_root_sfx_path()?.join("GUIDs.txt");
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
                        let data_vec = sfx_data.sfx_by_folder_map.entry(folder_name.to_string()).or_insert_with(|| Vec::new());
                        data_vec.push(line);
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
}

pub fn get_list_of_installed_cars_in(ac_install_path: &PathBuf) -> Result<Vec<PathBuf>> {
    let car_path = ac_install_path.join(PathBuf::from_iter(["content", "cars"]));
    read_cars_in_path(&car_path)
}

fn read_cars_in_path(car_path: &PathBuf) -> Result<Vec<PathBuf>> {
    let dir_entries = match fs::read_dir(car_path) {
        Ok(entry_list) => entry_list,
        Err(e) => return Err(Error::new(ErrorKind::NotInstalled,
                                        String::from(
                                            format!("Assetto Corsa doesn't appear to be installed: {}",
                                                    e.to_string()))))
    };

    let cars = dir_entries.filter_map(|e| {
        match e {
            Ok(dir_entry) => {
                let path = dir_entry.path();
                if path.is_dir() && (path.join("data").is_dir() || path.join("data.acd").is_file()) {
                    Some(dir_entry.path())
                } else {
                    None
                }
            },
            _ => None
        }
    }).collect();
    Ok(cars)
}


#[derive(Debug)]
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


#[cfg(test)]
mod tests {
    use crate::{Installation};

    #[test]
    fn sfx_test() -> Result<(), String> {
        let install = Installation::new();
        println!("{:?}", install.load_sfx_data());
        Ok(())
    }
}
