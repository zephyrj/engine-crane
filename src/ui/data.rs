/*
 * Copyright (c):
 * 2023 zephyrj
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

use std::fs::create_dir;
use std::path::PathBuf;
use tracing::{error, info, Level, span, warn};
use crate::data::{get_default_crate_engine_path, get_local_app_data_path};
use crate::ui::{GlobalSettings, ListPath};

pub struct ApplicationData {
    pub(crate) settings: GlobalSettings,
    pub(crate) assetto_corsa_data: AssettoCorsaData,
    pub(crate) beam_ng_data: BeamNGData
}

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

impl ApplicationData {
    pub(crate) fn new() -> ApplicationData {
        create_local_data_dirs_if_missing();
        let settings = GlobalSettings::load().unwrap_or_else(|e| {
            warn!("Failed to load settings. {}", e.to_string());
            GlobalSettings::default()
        });
        match settings.ac_install_path() {
            None => { info!("Assetto Corsa path not set") }
            Some(path) => { info!("Assetto Corsa path set to {}", path.display()) }
        }
        match settings.beamng_mod_path() {
            None => { info!("BeamNG mod path not set") }
            Some(path) => { info!("BeamNG mod path set to {}", path.display()) }
        }
        match settings.crate_engine_path() {
            None => { info!("Crate engine path not set") }
            Some(path) => { info!("Crate engine path set to {}", path.display()) }
        }

        let assetto_corsa_data = AssettoCorsaData::from_settings(&settings);
        let beam_ng_data = BeamNGData::from_settings(&settings);
        ApplicationData {
            settings,
            assetto_corsa_data,
            beam_ng_data
        }
    }

    pub(crate) fn get_ac_install_path(&self) -> Option<PathBuf> {
        self.settings.ac_install_path()
    }

    pub(crate) fn update_ac_install_path(&mut self, new_path: PathBuf) {
        self.settings.set_ac_install_path(&new_path);
        self.assetto_corsa_data.property_update(&self.settings);
    }

    pub(crate) fn get_beam_ng_mod_path(&self) -> Option<PathBuf> {
        self.settings.beamng_mod_path()
    }

    pub(crate) fn update_beamng_mod_path(&mut self, new_path: PathBuf) {
        self.settings.set_beamng_mod_path(&new_path);
        self.beam_ng_data.property_update(&self.settings);
    }

    pub(crate) fn get_crate_engine_path(&self) -> Option<PathBuf> {
        self.settings.crate_engine_path()
    }

    pub(crate) fn update_crate_engine_path(&mut self, new_path: PathBuf) {
        self.settings.set_crate_engine_pahth(&new_path);
    }

    pub(crate) fn refresh_available_cars(&mut self) {
        self.assetto_corsa_data.refresh_available_cars(&PathBuf::from(&self.settings.ac_install_path))
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
        if let Some(path) = &settings.ac_install_path() {
            self.refresh_available_cars(path);
        } else {
            info!("Update to GlobalSettings contains no AC install path");
            self.available_cars.clear();
        }
    }

    pub fn refresh_available_cars(&mut self, ac_install_path: &PathBuf) {
        self.available_cars.clear();
        if ac_install_path.is_dir() {
            self.available_cars = Self::load_available_cars(ac_install_path);
            self.available_cars.sort();
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
        if let Some(path) = &settings.beamng_mod_path() {
            self.refresh_available_mods(path);
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
        let mods = ListPath::convert_path_vec(beam_ng::get_mod_list_in(beamng_mod_path));
        info!("Found {} mods", mods.len());
        mods
    }
}
