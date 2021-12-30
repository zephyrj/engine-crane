use std::ffi::OsString;
use std::path::PathBuf;
use directories::UserDirs;
use rusqlite::{Connection, Result};

pub fn get_db_path() -> Option<OsString> {
    let sandbox_path = UserDirs::new()?.document_dir()?.join(PathBuf::from_iter(["My Games", "Automation", "Sandbox_openbeta.db"]));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

pub fn get_engine_names() -> Option<Vec<String>>
{
    let conn = Connection::open(get_db_path()?).unwrap();

}