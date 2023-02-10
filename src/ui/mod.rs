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

mod swap;
mod edit;
mod settings;

use std::ffi::OsStr;
use std::fs;
use swap::{EngineSwapMessage, EngineSwapTab};
use edit::{EditMessage, EditTab};
use settings::{SettingsMessage, SettingsTab};

use std::path::{Path, PathBuf};
use config::{Config, ConfigError};
use iced::{Element, Length, Sandbox, Error, Settings, Background, Color};
use iced::widget::{Column, Text, Container};
use iced_aw::{TabLabel, Tabs};
use iced::alignment::{Horizontal, Vertical};
//use iced_aw::tab_bar::{StyleSheet};
use iced::Theme;
use iced_aw::style::tab_bar::Appearance;
use iced_aw::style::TabBarStyles;
use iced_aw::tab_bar::StyleSheet;
use crate::{assetto_corsa, beam_ng};
use tracing::{span, Level, info, error, warn};
use rfd::FileDialog;
use serde::{Serialize, Deserialize};

const HEADER_SIZE: u16 = 32;
const TAB_PADDING: u16 = 16;

pub fn launch() -> Result<(), Error> {
    UIMain::run(Settings::default())
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(usize),
    AcPathSelectPressed,
    BeamNGModPathSelectPressed,
    EngineSwap(EngineSwapMessage),
    Edit(EditMessage),
    Settings(SettingsMessage)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalSettings {
    ac_install_path: String,
    beamng_mod_path: String,
}


//.map_err(|e|{ format!("Failed to set config. {}", e.to_string()) })?

impl GlobalSettings {
    const AC_INSTALL_PATH: &'static str = "ac_install_path";
    const BEAMNG_MOD_PATH: &'static str = "beamng_mod_path";
    const CONFIG_FILENAME: &'static str = "engine-crane-conf";

    fn default() -> Self {
        GlobalSettings {
            ac_install_path: assetto_corsa::get_default_install_path().to_string_lossy().into_owned(),
            beamng_mod_path: beam_ng::get_default_mod_path().to_string_lossy().into_owned()
        }
    }

    fn load() -> Result<Self, ConfigError> {
        let builder = Config::builder();
        return match builder
            .set_default(GlobalSettings::AC_INSTALL_PATH, assetto_corsa::get_default_install_path().to_string_lossy().into_owned())?
            .set_default(GlobalSettings::BEAMNG_MOD_PATH, beam_ng::get_default_mod_path().to_string_lossy().into_owned())?
            .add_source(config::File::with_name(GlobalSettings::CONFIG_FILENAME))
            .add_source(config::Environment::with_prefix("APP"))
            .build() {
            Ok(settings) => {
                settings.try_deserialize()
            }
            Err(e) => {
                warn!("Failed to load settings. {}", e.to_string());
                let builder = Config::builder();
                let settings = builder
                    .set_default(GlobalSettings::AC_INSTALL_PATH, assetto_corsa::get_default_install_path().to_string_lossy().into_owned())?
                    .set_default(GlobalSettings::BEAMNG_MOD_PATH, beam_ng::get_default_mod_path().to_string_lossy().into_owned())?
                    .build()?;
                let ret: GlobalSettings = settings.try_deserialize()?;
                ret.write().unwrap_or_else(|e| { error!("Failed to write settings. {}", e.to_string())});
                Ok(ret)
            }
        }
    }

    fn ac_install_path(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.ac_install_path);
        if path.is_dir() {
            return Some(path);
        }
        None
    }

    fn set_ac_install_path(&mut self, new_path: &Path) {
        self.ac_install_path = new_path.to_string_lossy().into_owned();
    }

    fn beamng_mod_path(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.beamng_mod_path);
        if path.is_dir() {
            return Some(path);
        }
        None
    }

    fn set_beamng_mod_path(&mut self, new_path: &Path) {
        self.beamng_mod_path = new_path.to_string_lossy().into_owned();
    }

    fn write(&self) -> std::io::Result<()> {
        fs::write(format!("{}.toml", GlobalSettings::CONFIG_FILENAME), toml::to_string(&self).map_err(|e|{
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to encode settings to toml")
        })?)
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ListPath {
    full_path: PathBuf,
}

impl ListPath {
    fn from_path(path: PathBuf) -> ListPath {
        ListPath {full_path: path}
    }

    fn convert_path_vec(mut path_vec: Vec<PathBuf>) -> Vec<ListPath> {
        path_vec.into_iter().fuse().map(|path|{
            ListPath::from_path(path)
        }).collect()
    }
}

impl std::fmt::Display for ListPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match &self.full_path.file_name() {
            None => { "".to_string() }
            Some(filename) => { filename.to_string_lossy().into_owned() }
        };
        write!(f, "{}", out)
    }
}

pub struct AssettoCorsaData {
    available_cars: Vec<ListPath>,
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

    fn refresh_available_cars(&mut self, ac_install_path: &PathBuf) {
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
    available_mods: Vec<ListPath>
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

pub struct ApplicationData {
    settings: GlobalSettings,
    assetto_corsa_data: AssettoCorsaData,
    beam_ng_data: BeamNGData
}

impl ApplicationData {
    fn new() -> ApplicationData {
        let settings = GlobalSettings::load().unwrap_or_else(|e| {
            error!("Failed to load settings. {}", e.to_string());
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

        let assetto_corsa_data = AssettoCorsaData::from_settings(&settings);
        let beam_ng_data = BeamNGData::from_settings(&settings);
        ApplicationData {
            settings,
            assetto_corsa_data,
            beam_ng_data
        }
    }

    fn get_ac_install_path(&self) -> Option<PathBuf> {
        self.settings.ac_install_path()
    }

    fn update_ac_install_path(&mut self, new_path: PathBuf) {
        self.settings.set_ac_install_path(&new_path);
        self.assetto_corsa_data.property_update(&self.settings);
    }

    fn get_beam_ng_mod_path(&self) -> Option<PathBuf> {
        self.settings.beamng_mod_path()
    }

    fn update_beamng_mod_path(&mut self, new_path: PathBuf) {
        self.settings.set_beamng_mod_path(&new_path);
        self.beam_ng_data.property_update(&self.settings);
    }
}

/// The default appearance of a [`TabBar`](crate::native::TabBar).
#[derive(Clone, Copy, Debug)]
pub struct CustomStyleSheet;

impl StyleSheet for CustomStyleSheet {
    type Style = Theme;

    fn active(&self, style: &Self::Style, is_active: bool) -> Appearance {
        Appearance {
            background: None,
            border_color: None,
            border_width: 0.0,
            tab_label_background: if is_active {
                Background::Color([0.9, 0.9, 0.9].into())
            } else {
                Background::Color([0.67, 0.67, 0.67].into())
            },
            tab_label_border_color: [0.7, 0.7, 0.7].into(),
            tab_label_border_width: 1.0,
            icon_color: if is_active {
                Color::BLACK
            } else {
                Color::from_rgb(0.5, 0.5, 0.5)
            },
            text_color: if is_active {
                Color::BLACK
            } else {
                Color::from_rgb(0.5, 0.5, 0.5)
            },
        }
    }

    fn hovered(&self, style: &Self::Style, is_active: bool) -> Appearance {
        Appearance {
            tab_label_background: Background::Color([0.3, 0.3, 0.3].into()),
            text_color: Color::WHITE,
            ..self.active(style, is_active)
        }
    }
}

pub struct UIMain {
    app_data: ApplicationData,
    active_tab: usize,
    engine_swap_tab: EngineSwapTab,
    edit_tab: EditTab,
    settings_tab: SettingsTab
}

impl UIMain {
    pub fn get_settings(&self) -> &GlobalSettings {
        &self.app_data.settings
    }

    pub fn get_ac_data(&self) -> &AssettoCorsaData {
        &self.app_data.assetto_corsa_data
    }

    pub fn get_beam_ng_data(&self) -> &BeamNGData {
        &self.app_data.beam_ng_data
    }

    pub fn notify_app_data_update(&mut self) {
        match self.app_data.settings.write() {
            Ok(_) => { info!("Wrote settings successfully"); }
            Err(e) => { error!("Failed to write settings. {}", e.to_string()); }
        }
        self.settings_tab.app_data_update(&self.app_data);
        self.engine_swap_tab.app_data_update(&self.app_data);
        self.edit_tab.app_data_update(&self.app_data);
    }
}

impl Sandbox for UIMain {
    type Message = Message;

    fn new() -> Self {
        span!(Level::INFO, "Creating UIMain");
        let app_data = ApplicationData::new();
        info!("Initialised settings successfully");
        let settings_tab = SettingsTab::new();
        info!("Created settings tab");
        let engine_swap_tab = EngineSwapTab::new();
        info!("Created engine-swap tab");
        UIMain {
            app_data,
            active_tab: 0,
            engine_swap_tab,
            edit_tab: EditTab::new(),
            settings_tab
        }
    }

    fn title(&self) -> String {
        String::from("Engine Crane")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::TabSelected(selected) => self.active_tab = selected,
            Message::EngineSwap(message) => self.engine_swap_tab.update(message, &self.app_data),
            Message::Edit(message) => self.edit_tab.update(message, &self.app_data),
            Message::Settings(message) => self.settings_tab.update(message, &self.app_data),
            Message::AcPathSelectPressed => {
                let install_path = FileDialog::new()
                    .set_directory(match self.app_data.get_ac_install_path() {
                        Some(str) => str,
                        None => PathBuf::from("/")
                    })
                    .pick_folder();
                if let Some(path) = install_path {
                    self.app_data.update_ac_install_path(path);
                    self.notify_app_data_update();
                }
            }
            Message::BeamNGModPathSelectPressed => {
                let mod_path = FileDialog::new()
                    .set_directory(match self.app_data.get_beam_ng_mod_path() {
                        Some(str) => str,
                        None => PathBuf::from("/")
                    })
                    .pick_folder();
                if let Some(path) = mod_path {
                    self.app_data.update_beamng_mod_path(path);
                    self.notify_app_data_update();
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        Tabs::new(self.active_tab, Message::TabSelected)
            .push(
                self.engine_swap_tab.tab_label(),
                self.engine_swap_tab.view(&self.app_data)
            )
            // .push(
            //     self.edit_tab.tab_label(),
            //     self.edit_tab.view(&self.app_data)
            // )
            .push(
                self.settings_tab.tab_label(),
                self.settings_tab.view(&self.app_data)
            )
            .tab_bar_style(TabBarStyles::Custom(Box::new(CustomStyleSheet)))
            .tab_bar_position(iced_aw::TabBarPosition::Top)
            .into()
    }
}


trait Tab {
    type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    fn view<'a, 'b>(
        &'a self,
        app_data: &'b ApplicationData
    ) -> Element<'_, Self::Message>
    where 'b: 'a
    {
        let column = Column::new()
            .spacing(5)
            .push(Text::new(self.title()).size(HEADER_SIZE))
            .push(self.content(app_data));

        Container::new(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(TAB_PADDING)
            .into()
    }

    fn content<'a, 'b>(
        &'a self,
        ac_data: &'b ApplicationData
    ) -> Element<'_, Self::Message>
    where 'b: 'a;
}
