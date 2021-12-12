/*
Copyright (c):
2021 zephyrj
zephyrj@protonmail.com

This file is part of engine-crane.

engine-crane is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

engine-crane is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with sim-racing-tools. If not, see <https://www.gnu.org/licenses/>.
*/

fn main() {
    if assetto_corsa::is_installed() {
        println!("Assetto Corsa is installed");
        println!("Installed cars can be found at {}",
                 assetto_corsa::get_installed_cars_path().unwrap().display())
    } else {
        println!("Assetto Corsa is not installed");
        return;
    }

    if automation::is_installed() {
        println!("Automation is installed");
    } else {
        println!("Automation is not installed");
        return;
    }

    println!("BeamNG mod folder resolved to {}", beam_ng::get_mod_path().unwrap().display());
}

mod assetto_corsa {
    use std::path::PathBuf;
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
}

mod automation {
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
}

mod beam_ng {
    use std::path::PathBuf;
    use directories::BaseDirs;
    use parselnk::Lnk;
    use crate::steam;

    pub const STEAM_GAME_NAME: &str = "BeamNG.drive";
    pub const STEAM_GAME_ID: i64 = 284160;

    pub fn get_mod_path() -> Option<PathBuf> {
        let dirs = BaseDirs::new().unwrap();
        let mut mod_path_buf: PathBuf = dirs.cache_dir().to_path_buf();
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
}

pub mod steam {
    use std::path::PathBuf;
    #[cfg(target_os = "linux")]
    use directories::UserDirs;

    #[cfg(target_os = "linux")]
    pub fn get_install_path(&str: game_name) -> Option<PathBuf> {
        if let Some(user_dirs) = UserDirs::new() {
            let mut install_path = PathBuf::from(user_dirs.home_dir());
            for path in [".steam", "debian-installation", "steamapps", "common", game_name] {
                install_path.push(path);
            }
            Some(install_path)
        } else {
            None
        }
    }

    #[cfg(target_os = "windows")]
    pub fn get_install_path(game_name: &str) -> Option<PathBuf> {
        let path = PathBuf::from(format!("C:\\Program Files (x86)\\Steam\\steamapps\\common\\{}",
                                     game_name));
        if path.is_dir() {
            Some(path)
        } else {
            None
        }
    }
}