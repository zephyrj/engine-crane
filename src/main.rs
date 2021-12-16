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
use std::path::Path;

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

    let car_path = Path::new("C:\\Program Files (x86)\\Steam\\steamapps\\common\\assettocorsa\\content\\cars\\abarth500");
    let car = assetto_corsa::Car::load_from_path(car_path).unwrap();
    println!("{:?}", car);
}

mod assetto_corsa {
    use std::fmt;
    use std::default::Default;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use configparser::ini::Ini;
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

    #[derive(Debug, Clone)]
    pub struct InvalidCarError {
        reason: String
    }

    impl fmt::Display for InvalidCarError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Invalid car config: {}", self.reason)
        }
    }

    #[derive(Debug)]
    pub enum CarVersion {
        Default,
        CspExtendedPhysics
    }

    impl Default for CarVersion {
        fn default() -> Self {
            CarVersion::Default
        }
    }

    impl CarVersion {
        pub const VERSION_1 :&'static str = "1";
        pub const CSP_EXTENDED_2 : &'static str = "extended-2";

        fn from_string(s: &str) -> CarVersion {
            match s {
                CarVersion::VERSION_1 => CarVersion::Default,
                CarVersion::CSP_EXTENDED_2 => CarVersion::CspExtendedPhysics,
                _ => CarVersion::Default
            }
        }

        fn to_string(&self) -> &str {
            match self {
                CarVersion::Default => CarVersion::VERSION_1,
                CarVersion::CspExtendedPhysics => CarVersion::CSP_EXTENDED_2
            }
        }
    }

    #[derive(Debug)]
    #[derive(Default)]
    pub struct Car {
        root_path: OsString,
        pub(crate) ini_config: Ini,
        version: CarVersion,
        screen_name: String,
        total_mass: u32,
        fuel_consumption: Option<f64>,
        default_fuel: u32,
        max_fuel: u32
    }

    impl Car {
        pub fn load_from_path(car_folder_path: &Path) -> Result<Car, InvalidCarError> {
            let mut car = Car {
                root_path: OsString::from(car_folder_path),
                ini_config: Ini::new(),
                ..Default::default()
            };
            let mut ini_path = PathBuf::from(car_folder_path);
            ini_path.push("data");
            ini_path.push("car.ini");
            match car.ini_config.load(ini_path.as_path()) {
                Err(err_str) =>  {
                    return Err(InvalidCarError {
                        reason: String::from(format!("Failed to decode {}: {}",
                                                        ini_path.display(),
                                                        err_str)) })
                },
                Ok(_) => {}
            }
            car.version = CarVersion::from_string(&car.get_ini_string("header", "version")?);
            car.screen_name = car.get_ini_string("info", "screen_name")?;
            car.total_mass = car.get_ini_int("basic", "totalmass")? as u32;
            car.default_fuel = car.get_ini_int("fuel", "fuel")? as u32;
            car.max_fuel = car.get_ini_int("fuel", "max_fuel")? as u32;
            match car.get_ini_float("fuel", "consumption") {
                Ok(fuel_consumption) => car.fuel_consumption = Some(fuel_consumption),
                Err(_) => {}
            }
            Ok(car)
        }

        fn get_ini_string(&self, section: &str, key: &str) -> Result<String, InvalidCarError> {
            if let Some(var) = self.ini_config.get(section, key) {
                Ok(var)
            } else {
                Err(InvalidCarError { reason: String::from(format!("Missing field {} -> {}",
                                                                      section, key)) })
            }
        }

        fn get_ini_int(&self, section: &str, key: &str) -> Result<i64, InvalidCarError> {
            Car::handle_number_result(self.ini_config.getint(section, key), section, key)
        }

        fn get_ini_float(&self, section: &str, key: &str) -> Result<f64, InvalidCarError> {
            Car::handle_number_result(self.ini_config.getfloat(section, key), section, key)
        }

        fn handle_number_result<T>(result: Result<Option<T>, String>, section: &str, key: &str) -> Result<T, InvalidCarError> {
            match result {
                Ok(var) => {
                    if let Some(ret_var) = var {
                        Ok(ret_var)
                    } else {
                        Err(InvalidCarError {
                            reason: String::from(format!("Missing field {} -> {}",
                                                         section, key)) })
                    }
                },
                Err(str) => Err(InvalidCarError {
                    reason: String::from(format!("Failed to parse field {} -> {}: {}",
                                                 section, key, str)) })
            }
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