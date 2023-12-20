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
mod image_data;
mod button;
mod data;

use std::fs;
use std::fs::create_dir;
use swap::{EngineSwapMessage, EngineSwapTab};
use edit::{EditMessage, EditTab};
use settings::{SettingsMessage, SettingsTab};

use std::path::{Path, PathBuf};
use config::{Config, ConfigError};
use iced::{Element, Sandbox, Error, Settings, Background, Color, Padding};
use iced::widget::{Column, Text, Container};
use iced_aw::{TabLabel, Tabs};
use iced::alignment::{Horizontal, Vertical};
use iced::Theme;
use iced_aw::style::tab_bar::Appearance;
use iced_aw::style::TabBarStyles;
use iced_aw::tab_bar::StyleSheet;
use crate::{assetto_corsa, beam_ng, fabricator};
use tracing::{span, Level, info, error, warn};
use rfd::FileDialog;
use serde::{Serialize, Deserialize};
use crate::data::{get_default_crate_engine_path, get_local_app_data_path};
use crate::fabricator::{AdditionalAcCarData, AssettoCorsaCarSettings};
use crate::ui::data::{ApplicationData, AssettoCorsaData, BeamNGData};

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
    CrateEnginePathSelectPressed,
    EngineSwap(EngineSwapMessage),
    EngineSwapRequested,
    Edit(EditMessage),
    Settings(SettingsMessage),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalSettings {
    ac_install_path: String,
    beamng_mod_path: String,
    crate_engine_path: String
}

impl GlobalSettings {
    const AC_INSTALL_PATH: &'static str = "ac_install_path";
    const BEAMNG_MOD_PATH: &'static str = "beamng_mod_path";
    const CRATE_ENGINE_PATH: &'static str = "crate_engine_path";
    const CONFIG_FILENAME: &'static str = "engine-crane-conf";

    fn default() -> Self {
        GlobalSettings {
            ac_install_path: assetto_corsa::get_default_install_path().to_string_lossy().into_owned(),
            beamng_mod_path: beam_ng::get_default_mod_path().to_string_lossy().into_owned(),
            crate_engine_path: crate::data::get_default_crate_engine_path().to_string_lossy().into_owned()
        }
    }

    fn load() -> Result<Self, ConfigError> {
        let builder = Config::builder();
        return match builder
            .set_default(GlobalSettings::AC_INSTALL_PATH, assetto_corsa::get_default_install_path().to_string_lossy().into_owned())?
            .set_default(GlobalSettings::BEAMNG_MOD_PATH, beam_ng::get_default_mod_path().to_string_lossy().into_owned())?
            .set_default(GlobalSettings::CRATE_ENGINE_PATH, crate::data::get_default_crate_engine_path().to_string_lossy().into_owned())?
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
                    .set_default(GlobalSettings::CRATE_ENGINE_PATH, crate::data::get_default_crate_engine_path().to_string_lossy().into_owned())?
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

    fn crate_engine_path(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.crate_engine_path);
        if path.is_dir() {
            return Some(path);
        }
        None
    }

    fn set_crate_engine_pahth(&mut self, new_path: &Path) {
        self.crate_engine_path = new_path.to_string_lossy().into_owned();
    }

    fn write(&self) -> std::io::Result<()> {
        fs::write(format!("{}.toml", GlobalSettings::CONFIG_FILENAME), toml::to_string(&self).map_err(|_e|{
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

    fn convert_path_vec(path_vec: Vec<PathBuf>) -> Vec<ListPath> {
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

/// The default appearance of a [`TabBar`](crate::native::TabBar).
#[derive(Clone, Copy, Debug)]
pub struct CustomStyleSheet;

impl StyleSheet for CustomStyleSheet {
    type Style = Theme;

    fn active(&self, _style: &Self::Style, is_active: bool) -> Appearance {
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

    pub fn notify_app_data_update(&mut self, update_event: &Message) {
        match self.app_data.settings.write() {
            Ok(_) => { info!("Wrote settings successfully"); }
            Err(e) => { error!("Failed to write settings. {}", e.to_string()); }
        }
        self.settings_tab.app_data_update(&self.app_data, update_event);
        self.engine_swap_tab.app_data_update(&self.app_data, update_event);
        self.edit_tab.app_data_update(&self.app_data, update_event);
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
        let edit_tab = EditTab::new(&app_data);
        UIMain {
            app_data,
            active_tab: 0,
            engine_swap_tab,
            edit_tab,
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
                let install_path = open_dir_select_dialog(self.app_data.get_ac_install_path().as_ref());
                if let Some(path) = install_path {
                    self.app_data.update_ac_install_path(path);
                    self.notify_app_data_update(&message);
                }
            }
            Message::BeamNGModPathSelectPressed => {
                let mod_path = open_dir_select_dialog(self.app_data.get_beam_ng_mod_path().as_ref());
                if let Some(path) = mod_path {
                    self.app_data.update_beamng_mod_path(path);
                    self.notify_app_data_update(&message);
                }
            }
            Message::CrateEnginePathSelectPressed => {
                let eng_path = open_dir_select_dialog(self.app_data.get_crate_engine_path().as_ref());
                if let Some(path) = eng_path {
                    self.app_data.update_crate_engine_path(path);
                    self.notify_app_data_update(&message);
                }
            }
            Message::EngineSwapRequested => {
                if self.app_data.get_ac_install_path().is_none() {
                    self.engine_swap_tab.update_status(String::from("Please set the Assetto Corsa install path in the settings tab"));
                    return;
                }
                if self.engine_swap_tab.current_car.is_none() {
                    self.engine_swap_tab.update_status(String::from("Please select an Assetto Corsa car"));
                    return;
                } else if self.engine_swap_tab.current_mod.is_none() {
                    self.engine_swap_tab.update_status(String::from("Please select an BeamNG mod"));
                    return;
                }

                let new_spec_name = self.engine_swap_tab.current_new_spec_name.as_str();
                let new_car_path = {
                    let span = span!(Level::INFO, "Creating new car spec");
                    let _enter = span.enter();
                    let ac_install = assetto_corsa::Installation::from_path(
                        self.app_data.get_ac_install_path().as_ref().unwrap().clone()
                    );
                    match assetto_corsa::car::create_new_car_spec(&ac_install,
                                                                  self.engine_swap_tab.current_car.as_ref().unwrap(),
                                                                  new_spec_name,
                                                                  self.engine_swap_tab.unpack_physics_data)
                    {
                        Ok(path) => { path }
                        Err(e) => {
                            error!("Swap failed: {}", e.to_string());
                            self.engine_swap_tab.update_status(format!("Swap failed: {}", e.to_string()));
                            return;
                        }
                    }
                };

                if let Some(mod_path) = self.engine_swap_tab.current_mod.as_ref() {
                    let span = span!(Level::INFO, "Updating car physics");
                    let _enter = span.enter();
                    let current_engine_weight =
                        if let Some(weight_string) = &self.engine_swap_tab.current_engine_weight {
                            match weight_string.parse::<u32>() {
                                Ok(val) => {
                                    Some(val)
                                }
                                Err(_) => {
                                    None
                                }
                            }
                        } else {
                            None
                        };
                    let mut car_settings = AssettoCorsaCarSettings::default();
                    car_settings.minimum_physics_level = self.engine_swap_tab.current_minimum_physics;
                    match fabricator::swap_automation_engine_into_ac_car(mod_path.as_path(),
                                                                         new_car_path.as_path(),
                                                                         car_settings,
                                                                         AdditionalAcCarData::new(current_engine_weight)) {
                        Ok(_) => {
                            self.engine_swap_tab.update_status(format!("Created {} successfully", new_car_path.display()));
                            self.app_data.refresh_available_cars();
                            self.notify_app_data_update(&message);
                        }
                        Err(err_str) => {
                            error!("{}", &err_str);
                            self.engine_swap_tab.update_status(err_str)
                        }
                    }
                } else {
                    let err_str = "Swap failed: Couldn't get ref to current mod";
                    error!(err_str);
                    self.engine_swap_tab.update_status(format!("{}", err_str));
                    return;
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
            .push(
                self.edit_tab.tab_label(),
                self.edit_tab.view(&self.app_data)
            )
            .push(
                self.settings_tab.tab_label(),
                self.settings_tab.view(&self.app_data)
            )
            .tab_bar_style(TabBarStyles::Custom(Box::new(CustomStyleSheet)))
            .tab_bar_position(iced_aw::TabBarPosition::Top)
            .into()
    }
}

fn open_dir_select_dialog(starting_path: Option<&PathBuf>) -> Option<PathBuf> {
    let root_dir = PathBuf::from("/");
    let path = starting_path.unwrap_or(&root_dir);
    FileDialog::new()
        .set_directory(path)
        .pick_folder()
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
            .align_x(Horizontal::Left)
            .align_y(Vertical::Top)
            .padding(Padding::from([TAB_PADDING*2, TAB_PADDING, TAB_PADDING, TAB_PADDING]))
            .into()
    }

    fn content<'a, 'b>(
        &'a self,
        ac_data: &'b ApplicationData
    ) -> Element<'_, Self::Message>
    where 'b: 'a;
}
