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
mod crate_engines;
mod elements;

use swap::{EngineSwapMessage, EngineSwapTab};
use edit::{EditMessage, EditTab};
use settings::{SettingsMessage, SettingsTab};

use std::path::{Path, PathBuf};
use iced::{Element, Sandbox, Error, Settings, Background, Color, Padding};
use iced::widget::{Column, Text, Container};
use iced_aw::{TabLabel, Tabs};
use iced::alignment::{Horizontal, Vertical};
use iced::Theme;
use iced_aw::style::tab_bar::Appearance;
use iced_aw::style::TabBarStyles;
use iced_aw::tab_bar::StyleSheet;
use crate::{assetto_corsa, fabricator};
use tracing::{span, Level, info, error};
use rfd::FileDialog;
use assetto_corsa::car::delete_car;
use automation::sandbox::SandboxFinder;

use crate::fabricator::{AdditionalAcCarData, AssettoCorsaCarSettings};
use crate::settings::GlobalSettings;
use crate::ui::crate_engines::{CrateEngineTab, CrateTabMessage};
use crate::ui::data::{ApplicationData, AssettoCorsaData, BeamNGData, CrateEngineData};
use crate::ui::settings::Setting;
use crate::ui::swap::EngineSource;

const HEADER_SIZE: u16 = 32;
const TAB_PADDING: u16 = 16;

pub fn launch() -> Result<(), Error> {
    UIMain::run(Settings::default())
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(usize),
    RequestPathSelect(Setting),
    RevertSettingToDefault(Setting),
    EngineSwap(EngineSwapMessage),
    EngineSwapRequested,
    CrateTab(CrateTabMessage),
    Edit(EditMessage),
    #[allow(dead_code)]
    Settings(SettingsMessage),
    DeleteCrateEngine(String),
    RefreshCrateEngines
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
    crate_engine_tab: CrateEngineTab,
    edit_tab: EditTab,
    settings_tab: SettingsTab
}

impl UIMain {
    #[allow(dead_code)]
    pub fn get_settings(&self) -> &GlobalSettings {
        &self.app_data.settings
    }

    #[allow(dead_code)]
    pub fn get_ac_data(&self) -> &AssettoCorsaData {
        &self.app_data.assetto_corsa_data
    }

    #[allow(dead_code)]
    pub fn get_beam_ng_data(&self) -> &BeamNGData {
        &self.app_data.beam_ng_data
    }

    #[allow(dead_code)]
    pub fn get_crate_engine_data(&self) -> &CrateEngineData { &self.app_data.crate_engine_data }

    pub fn notify_app_data_update(&mut self, update_event: &Message) {
        match self.app_data.settings.write() {
            Ok(_) => { info!("Wrote settings successfully"); }
            Err(e) => { error!("Failed to write settings. {}", e.to_string()); }
        }
        self.settings_tab.app_data_update(&self.app_data, update_event);
        self.engine_swap_tab.app_data_update(&self.app_data, update_event);
        self.crate_engine_tab.app_data_update(&self.app_data, update_event);
        self.edit_tab.app_data_update(&self.app_data, update_event);
    }

    pub fn notify_action_success(&mut self, action_event: &Message) {
        self.settings_tab.notify_action_success(action_event);
        self.engine_swap_tab.notify_action_success(action_event);
        self.crate_engine_tab.notify_action_success(action_event);
        self.edit_tab.notify_action_success(action_event);
    }

    pub fn notify_action_failure(&mut self, action_event: &Message, reason: String) {
        self.settings_tab.notify_action_failure(action_event, &reason);
        self.engine_swap_tab.notify_action_failure(action_event, &reason);
        self.crate_engine_tab.notify_action_failure(action_event, &reason);
        self.edit_tab.notify_action_failure(action_event, &reason);
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
        let crate_engine_tab = CrateEngineTab::new(&app_data);
        info!("Created crate engine tab");
        let edit_tab = EditTab::new(&app_data);
        info!("Created edit tab");
        UIMain {
            app_data,
            active_tab: 0,
            engine_swap_tab,
            crate_engine_tab,
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
            Message::CrateTab(message) => self.crate_engine_tab.update(message, &self.app_data),
            Message::Edit(message) => self.edit_tab.update(message, &self.app_data),
            Message::Settings(message) => self.settings_tab.update(message, &self.app_data),
            Message::RequestPathSelect(setting) => {
                let current_path = match setting {
                    Setting::AcPath => self.app_data.get_ac_install_path(),
                    Setting::BeamNGModPath => self.app_data.get_beam_ng_mod_path(),
                    Setting::CrateEnginePath => self.app_data.get_crate_engine_path(),
                    Setting::LegacyAutomationUserdataPath => self.app_data.get_legacy_automation_userdata_path(),
                    Setting::AutomationUserdataPath => self.app_data.get_automation_userdata_path()
                };
                if let Some(path) = open_dir_select_dialog(current_path.as_ref()) {
                    match setting {
                        Setting::AcPath => self.app_data.update_ac_install_path(path),
                        Setting::BeamNGModPath => self.app_data.update_beamng_mod_path(path),
                        Setting::CrateEnginePath => self.app_data.update_crate_engine_path(path),
                        Setting::LegacyAutomationUserdataPath => self.app_data.update_legacy_automation_userdata_path(path),
                        Setting::AutomationUserdataPath => self.app_data.update_automation_userdata_path(path)
                    }
                    self.notify_app_data_update(&message);
                }
            }
            Message::RevertSettingToDefault(setting) => {
                self.app_data.revert_to_default(setting);
                self.notify_app_data_update(&message);
            }
            Message::EngineSwapRequested => {
                let ac_install = match &self.app_data.get_ac_install_path() {
                    None => {
                        self.engine_swap_tab.update_status(String::from("Please set the Assetto Corsa install path in the settings tab"));
                        return;
                    }
                    Some(path) => assetto_corsa::Installation::from_path(path.clone())
                };

                if self.engine_swap_tab.current_car.is_none() {
                    self.engine_swap_tab.update_status(String::from("Please select an Assetto Corsa car"));
                    return;
                }

                match self.engine_swap_tab.current_source {
                    EngineSource::BeamNGMod => {
                        if self.engine_swap_tab.current_mod.is_none() {
                            self.engine_swap_tab.update_status(String::from("Please select an BeamNG mod"));
                            return;
                        }
                    }
                    EngineSource::CrateEngine => {
                        if self.engine_swap_tab.current_crate_eng.is_none() {
                            self.engine_swap_tab.update_status(String::from("Please select a crate engine"));
                            return;
                        }
                    }
                }

                let new_spec_name = self.engine_swap_tab.current_new_spec_name.as_str();
                if new_spec_name.is_empty() {
                    self.engine_swap_tab.update_status(String::from("Please enter a spec name"));
                    return;
                }
                let new_car_path = {
                    let span = span!(Level::INFO, "Creating new car spec");
                    let _enter = span.enter();

                    let current_car_path = match self.engine_swap_tab.current_car.as_ref() {
                        None => {
                            let err_str = "Swap failed: Cannot determine current car path";
                            error!("{}", &err_str);
                            self.engine_swap_tab.update_status(err_str.to_string());
                            return;
                        }
                        Some(path_ref) => {
                            path_ref
                        }
                    };

                    match assetto_corsa::car::create_new_car_spec(&ac_install,
                                                                  current_car_path,
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

                let mut car_settings = AssettoCorsaCarSettings::default();
                car_settings.minimum_physics_level = self.engine_swap_tab.current_minimum_physics;
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
                let additional_car_settings = AdditionalAcCarData::new(current_engine_weight);

                                let res = match self.engine_swap_tab.current_source {
                    EngineSource::BeamNGMod => {
                        let mod_path = match self.engine_swap_tab.current_mod.as_ref() {
                            Some(p) => p,
                            None => {
                                let err_str = "Swap failed: Couldn't get ref to current mod";
                                error!(err_str);
                                self.engine_swap_tab.update_status(format!("{}", err_str));
                                return;
                            }
                        };
                        let span = span!(Level::INFO, "Updating car physics from BeamNG mod");
                        let _enter = span.enter();

                        let mut sandbox_finder = SandboxFinder::default();
                        if let Some(path) = self.app_data.get_legacy_automation_userdata_path() {
                            sandbox_finder.set_legacy_userdata_path(path)
                        }
                        if let Some(path) = self.app_data.get_automation_userdata_path() {
                            sandbox_finder.set_userdata_path(path)
                        }
                        fabricator::swap_automation_engine_into_ac_car(mod_path.as_path(),
                                                                       new_car_path.as_path(),
                                                                       sandbox_finder,
                                                                       car_settings,
                                                                       additional_car_settings)
                    }
                    EngineSource::CrateEngine => {
                        let crate_eng_name = match self.engine_swap_tab.current_crate_eng.as_ref() {
                            Some(c) => c,
                            None => {
                                let err_str = "Couldn't get currently selected crate engine name";
                                error!(err_str);
                                self.engine_swap_tab.update_status(format!("{}", err_str));
                                return;
                            }
                        };
                        let crate_path = match self.app_data.crate_engine_data.get_path_for(crate_eng_name) {
                            Some(p) => p,
                            None => {
                                let err_str = format!("Path for crate engine {} not found", crate_eng_name);
                                error!(err_str);
                                self.engine_swap_tab.update_status(format!("{}", err_str));
                                return;
                            }
                        };
                        let span = span!(Level::INFO, "Updating car physics from crate engine");
                        let _enter = span.enter();
                        fabricator::swap_crate_engine_into_ac_car(crate_path.as_path(),
                                                                  new_car_path.as_path(),
                                                                  car_settings,
                                                                  additional_car_settings)
                    }
                };
                match res {
                    Ok(_) => {
                        self.engine_swap_tab.update_status(format!("Created {} successfully", new_car_path.display()));
                        self.app_data.refresh_available_cars();
                        self.notify_app_data_update(&message);
                    }
                    Err(err_str) => {
                        if let Some(car_folder_name) = new_car_path.file_name() {
                            delete_car(&ac_install, Path::new(car_folder_name)).unwrap_or_else(|e|{
                                error!("Failed to delete {}. {}", new_car_path.display(), e.to_string());
                            });
                        } else {
                            error!("Failed to delete {}. Couldn't get car folder name", new_car_path.display());
                        }
                        error!("{}", &err_str);
                        self.engine_swap_tab.update_status(err_str.to_string())
                    }
                }
            },
            Message::RefreshCrateEngines => {
                self.app_data.refresh_crate_engines();
                self.notify_app_data_update(&message);
            }
            Message::DeleteCrateEngine(ref eng_id) => {
                if let Some(path) = self.app_data.crate_engine_data.get_location_for(eng_id.as_str()) {
                    match std::fs::remove_file(path) {
                        Ok(_) => {
                            self.notify_action_success(&message);
                            self.app_data.refresh_crate_engines();
                        },
                        Err(e) => {
                            self.notify_action_failure(&message, e.to_string());
                        }
                    }
                }
                self.notify_app_data_update(&message);
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
                self.crate_engine_tab.tab_label(),
                self.crate_engine_tab.view(&self.app_data)
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
