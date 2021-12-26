use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use directories::BaseDirs;
use parselnk::Lnk;
use crate::steam;

pub const STEAM_GAME_NAME: &str = "BeamNG.drive";
pub const STEAM_GAME_ID: i64 = 284160;

pub fn get_mod_path() -> Option<PathBuf> {
    let mut mod_path_buf: PathBuf = BaseDirs::new().unwrap().cache_dir().to_path_buf();
    mod_path_buf.push(STEAM_GAME_NAME);
    match steam::get_install_path(STEAM_GAME_NAME) {
        Some(_) => {
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
        None => {}
    }
    mod_path_buf.push("mods");
    Some(mod_path_buf)
}

pub fn get_mod_list() -> Option<Vec<OsString>> {
    let mod_dir = get_mod_path()?;
    let dir_entries = match fs::read_dir(mod_dir) {
        Ok(entry_list) => entry_list,
        Err(e) => return None
    };

    let mods: Vec<OsString> = dir_entries.filter_map(|e| {
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
                    Some(dir_entry.path().into_os_string())
                } else {
                    None
                }
            },
            _ => None
        }
    }).collect();
    Some(mods)
}