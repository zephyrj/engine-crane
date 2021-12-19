mod error;
pub mod car;

use std::collections::HashMap;
use std::fs;
use std::default::Default;
use std::ffi::OsString;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use configparser::ini::Ini;
use serde_json::Value;


use crate::steam;

pub const STEAM_GAME_NAME: &str = "assettocorsa";
pub const STEAM_GAME_ID: i64 = 244210;

pub fn is_installed() -> bool {
    if let Some(install_path) = steam::get_install_path(STEAM_GAME_NAME) {
        install_path.is_dir()
    } else {
        false
    }
}

pub fn get_installed_cars_path() -> Option<PathBuf> {
    if let Some(mut install_path) = steam::get_install_path(STEAM_GAME_NAME) {
        for path in ["content", "cars"] {
            install_path.push(path)
        }
        Some(install_path)
    } else {
        None
    }
}

#[derive(Debug)]
pub struct Cars {
    unpacked_cars: Vec<car::Car>,
    packed_car_dirs: Vec<OsString>
}

impl Cars {
    pub fn load() {

    }
}
