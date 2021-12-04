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
    println!("The game is referred to as {} and its game id is {}",
             assetto_corsa::steam::GAME_NAME,
             assetto_corsa::steam::GAME_ID);

    if assetto_corsa::is_installed() {
        println!("It is installed");
        println!("Installed cars can be found at {}",
                 assetto_corsa::get_installed_cars_path().unwrap().display())
    } else {
        println!("It is not installed");
        return;
    }

}

mod assetto_corsa {
    use std::path::PathBuf;

    pub fn is_installed() -> bool {
        if let Some(install_path) = steam::get_install_path() {
            install_path.is_dir()
        } else {
            false
        }
    }

    pub fn get_installed_cars_path() -> Option<PathBuf> {
        if let Some(mut install_path) = steam::get_install_path() {
            for path in ["content", "cars"] {
                install_path.push(path)
            }
            Some(install_path)
        } else {
            None
        }
    }

    pub mod steam {
        use std::path::PathBuf;
        use directories::UserDirs;

        pub const GAME_NAME: &str = "assettocorsa";
        pub const GAME_ID: i64 = 244210;

        #[cfg(target_os = "linux")]
        pub fn get_install_path() -> Option<PathBuf> {
            if let Some(user_dirs) = UserDirs::new() {
                let mut install_path = PathBuf::from(user_dirs.home_dir());
                for path in [".steam", "debian-installation", "steamapps", "common", GAME_NAME] {
                    install_path.push(path);
                }
                Some(install_path)
            } else {
                None
            }
        }

        #[cfg(target_os = "windows")]
        pub fn get_install_path() -> Option<PathBuf> {
            Some(PathBuf::from(format!("C:\\Program Files (x86)\\Steam\\{}", GAME_NAME)))
        }
    }
}

mod automation {

}