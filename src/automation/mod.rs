pub mod car;
pub mod sandbox;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use directories::UserDirs;
use crate::steam;

pub const STEAM_GAME_NAME: &str = "Automation";
pub const STEAM_GAME_ID: i64 = 293760;

pub fn is_installed() -> bool {
    if let Some(install_path) = steam::get_install_path(STEAM_GAME_NAME) {
        install_path.is_dir()
    } else {
        false
    }
}




