pub mod car;
pub mod sandbox;

use std::path::PathBuf;
use crate::steam;

pub const STEAM_GAME_NAME: &str = "Automation";
pub const STEAM_GAME_ID: i64 = 293760;

pub fn is_installed() -> bool {
    if let Some(install_path) = get_install_path() {
        install_path.is_dir()
    } else {
        false
    }
}

pub fn get_install_path() -> Option<PathBuf> {
    steam::get_game_install_path(STEAM_GAME_NAME)
}





