/*
 * Copyright (c):
 * 2022 zephyrj
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

use std::path::PathBuf;

#[cfg(target_os = "linux")]
use directories::UserDirs;

#[cfg(target_os = "linux")]
pub fn get_install_dir() -> PathBuf {
    let user_dirs = UserDirs::new().unwrap();
    let mut install_path = PathBuf::from(user_dirs.home_dir());
    for path in [".steam", "debian-installation"] {
        install_path.push(path);
    }
    install_path
}

#[cfg(target_os = "windows")]
pub fn get_install_dir() -> PathBuf {
    PathBuf::from("C:\\Program Files (x86)\\Steam")
}

pub fn get_game_install_path(game_name: &str) -> PathBuf {
    let mut install_path = get_install_dir();
    for path in ["steamapps", "common", game_name] {
        install_path.push(path);
    }
    install_path
}

#[cfg(target_os = "linux")]
pub fn get_wine_prefix_dir(game_id: i64) -> PathBuf {
    let mut install_path = get_install_dir();
    for path in ["steamapps", "compatdata", &game_id.to_string(), "pfx", "drive_c"] {
        install_path.push(path);
    }
    install_path
}

#[cfg(target_os = "linux")]
pub fn get_wine_documents_dir(game_id: i64) -> PathBuf {
    let mut install_path = get_wine_prefix_dir(game_id);
    for path in ["users", "steamuser", "My Documents"] {
        install_path.push(path);
    }
    install_path
}

#[cfg(target_os = "linux")]
pub fn get_wine_appdata_local_dir(game_id: i64) -> PathBuf {
    let mut install_path = get_wine_prefix_dir(game_id);
    for path in ["users", "steamuser", "AppData", "Local"] {
        install_path.push(path);
    }
    install_path
}

