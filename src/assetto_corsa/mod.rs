mod error;
pub mod car;
mod engine;
mod file_utils;
mod lut_utils;
mod ini_utils;

use std::collections::HashMap;
use std::fs;
use std::ffi::OsString;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use configparser::ini::Ini;
use serde_json::Value;
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
