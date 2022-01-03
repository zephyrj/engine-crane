use std::ffi::OsString;
use std::path::PathBuf;
use directories::UserDirs;
use rusqlite::{Connection, Result};

use crate::steam;
use crate::automation::{STEAM_GAME_ID};


#[cfg(target_os = "windows")]
pub fn get_db_path() -> Option<OsString> {
    let sandbox_path = UserDirs::new()?.document_dir()?.join(PathBuf::from_iter(legacy_sandbox_path()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

#[cfg(target_os = "linux")]
pub fn get_db_path() -> Option<OsString> {
    let sandbox_path = steam::get_wine_documents_dir(STEAM_GAME_ID)?.join(PathBuf::from_iter(legacy_sandbox_path()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

pub fn get_engine_names() -> Option<Vec<String>>
{
    let conn = Connection::open(get_db_path()?).unwrap();
    None
}

fn legacy_sandbox_path() -> Vec<&'static str> {
    vec!["My Games", "Automation", "Sandbox_openbeta.db"]
}