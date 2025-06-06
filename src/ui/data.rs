/*
 * Copyright (c):
 * 2025 zephyrj
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

use zephyrj_ac_tools as assetto_corsa;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};
use std::fs::create_dir;
use std::io;
use std::path::PathBuf;
use tracing::{error, info, Level, span, warn};
use crate_engine::{CrateEngineMetadata};
use engine_crane_lib::data::{find_crate_engines_in_path, get_default_crate_engine_path, get_local_app_data_path};
use crate::settings::{AcInstallPath, AutomationUserdataPath, BeamNGModPath, CrateEnginePath, LegacyAutomationUserdataPath, PathSetting};
use crate::ui::{GlobalSettings, ListPath, settings};
use crate::ui::settings::Setting;
use crate::utils::filesystem;

fn create_local_data_dirs_if_missing() {
    let local_data_path = get_local_app_data_path();
    if !local_data_path.is_dir() {
        match create_dir(&local_data_path) {
            Ok(_) => {
                info!("Created local data dir {}", local_data_path.display());
                let crate_eng_dir = get_default_crate_engine_path();
                match create_dir(&crate_eng_dir) {
                    Ok(_) => {
                        info!("Created default crate engine dir {}", crate_eng_dir.display());
                    }
                    Err(e) => {
                        warn!("Failed to create default crate engine dir. {}", e.to_string())
                    }
                }
            }
            Err(e) => {
                warn!("Failed to create local data dir. {}", e.to_string())
            }
        }
    } else {
        info!("Local app data dir found at {}", local_data_path.display());
    }
}

pub struct ApplicationData {
    pub(crate) settings: GlobalSettings,
    pub(crate) assetto_corsa_data: AssettoCorsaData,
    pub(crate) beam_ng_data: BeamNGData,
    pub(crate) crate_engine_data: CrateEngineData,
    pub(crate) permissions: HashMap<&'static str, (PathState, PathState)>
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PathState {
    Ok,
    NotOk,
    DoesntExist,
    Invalid
}

impl PathState {
    fn as_str(&self) -> &'static str {
        match self {
            PathState::Ok => "Ok",
            PathState::NotOk => "Not Ok",
            PathState::DoesntExist => "Doesn't exist",
            PathState::Invalid => "Invalid"
        }
    }
}

impl Display for PathState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

macro_rules! resolve_path {
    ($path_string:expr) => {
        {
            let path = PathBuf::from($path_string);
            match path.is_dir() {
                true => Some(path),
                false => None
            }
        }
    }
}

impl ApplicationData {
    pub(crate) fn new() -> ApplicationData {
        create_local_data_dirs_if_missing();
        let settings = GlobalSettings::load().unwrap_or_else(|e| {
            warn!("Failed to load settings. {}", e.to_string());
            GlobalSettings::default()
        });
        let mut data = ApplicationData {
            settings,
            assetto_corsa_data: AssettoCorsaData::new(),
            beam_ng_data: BeamNGData::new(),
            crate_engine_data: CrateEngineData::new(),
            permissions: HashMap::new()
        };
        data.set_path_permission_data::<AcInstallPath>();
        data.assetto_corsa_data.property_update(&data.settings);
        data.set_path_permission_data::<BeamNGModPath>();
        data.beam_ng_data.property_update(&data.settings);
        data.set_path_permission_data::<CrateEnginePath>();
        data.crate_engine_data.property_update(&data.settings);
        data.set_path_permission_data::<LegacyAutomationUserdataPath>();
        data.set_path_permission_data::<AutomationUserdataPath>();
        data
    }

    pub(crate) fn revert_to_default(&mut self, setting: settings::Setting) {
        match setting {
            Setting::AcPath => {
                self.revert_to_default_path::<AcInstallPath>();
                self.assetto_corsa_data.property_update(&self.settings);
            }
            Setting::BeamNGModPath => {
                self.revert_to_default_path::<BeamNGModPath>();
                self.beam_ng_data.property_update(&self.settings);
            }
            Setting::CrateEnginePath => {
                self.revert_to_default_path::<CrateEnginePath>();
                self.crate_engine_data.property_update(&self.settings);
            }
            Setting::LegacyAutomationUserdataPath => {
                self.revert_to_default_path::<LegacyAutomationUserdataPath>();
            }
            Setting::AutomationUserdataPath => {
                self.revert_to_default_path::<AutomationUserdataPath>();
            }
        }
    }

    fn revert_to_default_path<T: PathSetting>(&mut self) {
        self.set_path::<T>(T::default());
    }

    pub(crate) fn get_path<T: PathSetting>(&self) -> Option<PathBuf> {
        T::resolve_path(&self.settings)
    }

    pub(crate) fn set_path<T: PathSetting>(&mut self, val: T::ValueType) {
        T::set(&mut self.settings, val);
        self.set_path_permission_data::<T>();
    }

    pub(crate) fn get_ac_install_path(&self) -> Option<PathBuf> {
        self.get_path::<AcInstallPath>()
    }

    pub(crate) fn update_ac_install_path(&mut self, new_path: PathBuf) {
        self.set_path::<AcInstallPath>(new_path.to_string_lossy().into_owned());
        self.assetto_corsa_data.refresh_available_cars(resolve_path!(self.settings.get::<AcInstallPath>()));
    }

    pub(crate) fn get_beam_ng_mod_path(&self) -> Option<PathBuf> {
        self.get_path::<BeamNGModPath>()
    }

    pub(crate) fn update_beamng_mod_path(&mut self, new_path: PathBuf) {
        self.set_path::<BeamNGModPath>(new_path.to_string_lossy().into_owned());
        self.beam_ng_data.property_update(&self.settings);
    }

    pub(crate) fn get_crate_engine_path(&self) -> Option<PathBuf> {
        self.get_path::<CrateEnginePath>()
    }

    pub(crate) fn update_crate_engine_path(&mut self, new_path: PathBuf) {
        self.set_path::<CrateEnginePath>(new_path.to_string_lossy().into_owned());
        self.crate_engine_data.property_update(&self.settings)
    }

    pub(crate) fn get_legacy_automation_userdata_path(&self) -> Option<PathBuf> {
        self.get_path::<LegacyAutomationUserdataPath>()
    }

    pub(crate) fn update_legacy_automation_userdata_path(&mut self, new_path: PathBuf) {
        self.set_path::<LegacyAutomationUserdataPath>(new_path.to_string_lossy().into_owned());
    }

    pub(crate) fn get_automation_userdata_path(&self) -> Option<PathBuf> {
        self.get_path::<AutomationUserdataPath>()
    }

    pub(crate) fn update_automation_userdata_path(&mut self, new_path: PathBuf) {
        self.set_path::<AutomationUserdataPath>(new_path.to_string_lossy().into_owned());
    }

    pub(crate) fn refresh_available_cars(&mut self) {
        self.assetto_corsa_data.refresh_available_cars(self.get_path::<AcInstallPath>())
    }

    pub(crate) fn refresh_crate_engines(&mut self) {
        self.crate_engine_data.refresh_available_engines(self.get_path::<CrateEnginePath>())
    }

    pub fn get_permission_data<T: PathSetting>(&self) -> (PathState, PathState) {
        match self.permissions.get(T::param_name()) {
            None => (PathState::Invalid, PathState::Invalid),
            Some((readable, writable)) => (*readable, *writable)
        }
    }

    fn update_permission_data<T: PathSetting>(&mut self,
                                                                  read_state: PathState,
                                                                  write_state: PathState)
    {
        self.permissions.entry(T::param_name())
            .and_modify(|(readable, writable)|{
                (*readable, *writable) = (read_state, write_state);
            })
            .or_insert_with(||{
                (read_state, write_state)
            });
    }

    fn set_path_permission_data<T: PathSetting>(&mut self) {
        match self.get_path::<T>() {
            None => {
                info!("{} not set", T::friendly_name());
                self.update_permission_data::<T>(PathState::DoesntExist, PathState::DoesntExist);
            }
            Some(path) => {
                info!("{} set to {}", T::friendly_name(), path.display());
                match filesystem::is_directory_read_writable(&path) {
                    Ok((readable, writable)) => {
                        let read_state = match readable {
                            true => PathState::Ok,
                            false => PathState::NotOk
                        };
                        let write_state = match writable {
                            true => PathState::Ok,
                            false => PathState::NotOk
                        };
                        info!("{} readable state {}", path.display(), read_state);
                        info!("{} writable state {}", path.display(), write_state);
                        self.update_permission_data::<T>(read_state, write_state);
                    }
                    Err(e) => {
                        let new_state = match e.kind() {
                            io::ErrorKind::NotFound => {
                                warn!("{} cannot be found", path.display());
                                PathState::DoesntExist
                            },
                            _ => {
                                warn!("{} is an invalid path. {}", path.display(), e.to_string());
                                PathState::Invalid
                            }
                        };
                        self.update_permission_data::<T>(new_state, new_state);
                    }
                }
            }
        }
    }
}

pub struct AssettoCorsaData {
    pub(crate) available_cars: Vec<ListPath>,
}

impl AssettoCorsaData {
    fn new() -> AssettoCorsaData {
        AssettoCorsaData {
            available_cars: Vec::new()
        }
    }

    fn from_settings(settings: &GlobalSettings) -> AssettoCorsaData {
        let mut ac_data = AssettoCorsaData::new();
        ac_data.property_update(settings);
        ac_data
    }

    pub fn property_update(&mut self, settings: &GlobalSettings) {
        self.refresh_available_cars(resolve_path!(settings.get::<AcInstallPath>()))
    }

    pub fn refresh_available_cars(&mut self, ac_install_path: Option<PathBuf>) {
        self.available_cars.clear();
        match ac_install_path {
            None => warn!("No AC install path set when refreshing car list"),
            Some(path) => {
                if path.is_dir() {
                    self.available_cars = Self::load_available_cars(&path);
                    self.available_cars.sort();
                } else {
                    warn!("Invalid AC install path set when refreshing car list. {}", path.display())
                }
            }
        }
    }

    fn load_available_cars(ac_install_path: &PathBuf) -> Vec<ListPath> {
        let span = span!(Level::INFO, "Loading Assetto Corsa cars");
        let _enter = span.enter();
        return match assetto_corsa::Installation::from_path(ac_install_path.clone()).get_list_of_installed_cars() {
            Ok(vec) => {
                info!("Found {} cars", vec.len());
                ListPath::convert_path_vec(vec)
            }
            Err(err) => {
                error!("{}", err.to_string());
                Vec::new()
            }
        }
    }
}

pub struct BeamNGData {
    pub(crate) available_mods: Vec<ListPath>
}

impl BeamNGData {
    fn new() -> BeamNGData {
        BeamNGData {
            available_mods: Vec::new()
        }
    }

    fn from_settings(settings: &GlobalSettings) -> BeamNGData {
        let mut beam_data = BeamNGData::new();
        beam_data.property_update(settings);
        beam_data
    }

    pub fn property_update(&mut self, settings: &GlobalSettings) {
        if let Some(path) = resolve_path!(settings.get::<BeamNGModPath>()) {
            self.refresh_available_mods(&path);
        } else {
            info!("Update to GlobalSettings contains no BeamNG data path");
            self.available_mods.clear();
        }
    }

    fn refresh_available_mods(&mut self, beam_install_path: &PathBuf) {
        self.available_mods.clear();
        if beam_install_path.is_dir() {
            self.available_mods = Self::load_available_mods(beam_install_path);
            self.available_mods.sort();
        }
    }

    fn load_available_mods(beamng_mod_path: &PathBuf) -> Vec<ListPath> {
        let span = span!(Level::INFO, "Loading beamNG mods");
        let _enter = span.enter();
        let mods = ListPath::convert_path_vec(zephyrj_beamng_tools::get_mod_list_in(beamng_mod_path));
        info!("Found {} mods", mods.len());
        mods
    }
}

pub struct CrateEngineData {
    pub(crate) available_engines: Vec<String>,
    metadata: BTreeMap<String, CrateEngineMetadata>,
    locations: BTreeMap<String, PathBuf>
}

impl CrateEngineData {
    fn new() -> CrateEngineData {
        CrateEngineData {
            available_engines: Vec::new(),
            metadata: BTreeMap::new(),
            locations: BTreeMap::new()
        }
    }

    fn from_settings(settings: &GlobalSettings) -> CrateEngineData {
        let mut data = CrateEngineData::new();
        data.property_update(settings);
        data
    }

    pub fn property_update(&mut self, settings: &GlobalSettings) {
        self.refresh_available_engines(resolve_path!(settings.get::<CrateEnginePath>()));
    }

    pub fn get_path_for(&self, name: &str) -> Option<&PathBuf> {
        self.locations.get(name)
    }

    pub fn get_metadata_for(&self, name: &str) -> Option<&CrateEngineMetadata> {
        self.metadata.get(name)
    }

    pub fn get_location_for(&self, name: &str) -> Option<&PathBuf> {
        self.locations.get(name)
    }

    fn clear_data(&mut self) {
        self.available_engines.clear();
        self.locations.clear();
        self.metadata.clear();
    }

    fn refresh_available_engines(&mut self, crate_engine_path: Option<PathBuf>) {
        self.clear_data();
        match crate_engine_path {
            None => warn!("No crate engine path set when refreshing engines"),
            Some(path) => {
                if path.is_dir() {
                    self.load_available_engines(&path);
                } else {
                    warn!("Invalid crate engine path set when refreshing engines. {}", path.display())
                }
            }
        }
    }

    fn load_available_engines(&mut self, crate_eng_path: &PathBuf) {
        let span = span!(Level::INFO, "Loading crate engines");
        let _enter = span.enter();
        let found_engs = find_crate_engines_in_path(crate_eng_path, None).unwrap_or_else(|e| {
            warn!("Failed to read {}. {}", crate_eng_path.display(), e.to_string());
            BTreeMap::new()
        });
        info!("Found {} crate engines", found_engs.len());
        for (path, metadatum) in found_engs.into_iter() {
            let x = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy();
            let id_name =
                format!("{} ({})",
                        metadatum.name().to_string(),
                        x);
            self.add_engine(id_name, path, metadatum);
        }
    }

    fn add_engine(&mut self, id_name: String, path: PathBuf, metadatum: CrateEngineMetadata) {
        self.available_engines.push(id_name.clone());
        self.locations.insert(id_name.clone(), path);
        self.metadata.insert(id_name, metadatum);
    }
}
