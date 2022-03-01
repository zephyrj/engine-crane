pub mod jbeam;

use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use serde_hjson::Value;
use crate::steam;

pub const STEAM_GAME_NAME: &str = "BeamNG.drive";
pub const STEAM_GAME_ID: i64 = 284160;

#[cfg(target_os = "windows")]
pub fn get_mod_path() -> Option<PathBuf> {
    let mut mod_path_buf: PathBuf = BaseDirs::new().unwrap().cache_dir().to_path_buf();
    mod_path_buf.push(STEAM_GAME_NAME);
    match steam::get_game_install_path(STEAM_GAME_NAME) {
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

#[cfg(target_os = "linux")]
pub fn get_mod_path() -> Option<PathBuf> {
    use crate::automation;
    let mut mod_path_buf: PathBuf = steam::get_wine_prefix_dir(automation::STEAM_GAME_ID)?;
    for path in ["users", "steamuser", "AppData", "Local", "BeamNG.drive", "mods"] {
        mod_path_buf.push(path);
    }
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

#[derive(Debug)]
pub struct ModData {
    pub car_file_data: Vec<u8>,
    pub engine_jbeam_data: serde_hjson::Map<String, serde_hjson::Value>
}

pub fn load_mod_data(mod_name: &str) -> Result<ModData, String> {
    let mod_path = &get_mod_path();
    let mod_path = match mod_path {
        None => { return Err(String::from("Cannot find Beam.NG mods path")); }
        Some(mod_path) => { mod_path.join(mod_name) }
    };
    extract_mod_data(mod_path.as_path())
}

pub fn extract_mod_data(mod_path: &Path) -> Result<ModData, String> {
    let zipfile = std::fs::File::open(mod_path).map_err(|err| {
        format!("Failed to open {}. {}", mod_path.display(), err.to_string())
    })?;
    let mut archive = zip::ZipArchive::new(zipfile).map_err(|err| {
        format!("Failed to read archive {}. {}", mod_path.display(), err.to_string())
    })?;
    let filenames: Vec<String> = archive.file_names().map(|filename| {
        String::from(filename)
    }).collect();

    let mut car_data: Vec<u8> = Vec::new();
    let mut jbeam_data: Vec<u8> = Vec::new();
    for file_path in &filenames {
        if file_path.ends_with(".car") {
            match archive.by_name(file_path) {
                Ok(mut file) => {
                    file.read_to_end(&mut car_data).unwrap();
                },
                Err(err) => {
                    return Err(format!("Failed to read {}. {}", file_path, err.to_string()));
                }
            }
        } else if file_path.ends_with("camso_engine.jbeam") {
            match archive.by_name(file_path) {
                Ok(mut file) => {
                    file.read_to_end(&mut jbeam_data).unwrap();
                },
                Err(err) => {
                    return Err(format!("Failed to read {}. {}", file_path, err.to_string()));
                }
            }
        }
    }
    Ok(ModData{
        car_file_data: car_data,
        engine_jbeam_data: jbeam::from_slice(&*jbeam_data).unwrap()
    })
}
