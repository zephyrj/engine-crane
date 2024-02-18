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

use std::fmt::{Display, Formatter};
use super::{Message, Tab};
use std::path::{PathBuf};
use iced::{Alignment, Element, Length, Padding, Renderer};
use iced::widget::{Button, checkbox, Column, Container, pick_list, PickList, Row, Text, TextInput};
use iced_aw::{TabLabel};
use iced::alignment::Horizontal;
use iced_native::widget::radio;

use crate::fabricator::{AssettoCorsaPhysicsLevel};
use crate::ui::{ApplicationData, ListPath};
use crate::ui::settings::Setting;

#[derive(Debug, Clone)]
pub enum EngineSwapMessage {
    CarSelected(ListPath),
    SourceChanged(EngineSource),
    NameEntered(String),
    ModSelected(ListPath),
    CrateEngineSelected(String),
    PhysicsLevelSelected(AssettoCorsaPhysicsLevel),
    OldEngineWeightEntered(String),
    UnpackToggled(bool),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EngineSource {
    BeamNGMod,
    CrateEngine
}

impl EngineSource {
    pub fn all_options() -> [EngineSource; 2] {
        [EngineSource::BeamNGMod, EngineSource::CrateEngine]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EngineSource::BeamNGMod => "BeamNG Mod",
            EngineSource::CrateEngine => "Crate Engine"
        }
    }
}

impl Default for EngineSource {
    fn default() -> Self {
        EngineSource::BeamNGMod
    }
}

impl Display for EngineSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Default)]
pub struct EngineSwapTab {
    available_physics: Vec<AssettoCorsaPhysicsLevel>,
    pub(crate) current_source: EngineSource,
    pub(crate) current_car: Option<PathBuf>,
    pub(crate) current_mod: Option<PathBuf>,
    pub(crate) current_crate_eng: Option<String>,
    pub(crate) current_new_spec_name: String,
    pub(crate) current_engine_weight: Option<String>,
    pub(crate) current_minimum_physics: AssettoCorsaPhysicsLevel,
    pub(crate) unpack_physics_data: bool,
    status_message: String
}

impl EngineSwapTab {
    pub(crate) fn new() -> Self {
        EngineSwapTab {
            available_physics: vec![AssettoCorsaPhysicsLevel::BaseGame],
            current_source: EngineSource::BeamNGMod,
            current_car: None,
            current_mod: None,
            current_crate_eng: None,
            current_new_spec_name: "".to_string(),
            current_engine_weight: None,
            current_minimum_physics: Default::default(),
            unpack_physics_data: false,
            status_message: "".to_string()
        }
    }

    pub fn app_data_update(&mut self, _app_data: &ApplicationData, update_event: &Message) {
        match update_event {
            Message::AcPathSelectPressed | Message::BeamNGModPathSelectPressed | Message::CrateEnginePathSelectPressed => self.refresh(),
            Message::RevertSettingToDefault(setting) => match setting {
                Setting::AcPath | Setting::BeamNGModPath | Setting::CrateEnginePath => self.refresh(),
                _ => {}
            }
            Message::EngineSwapRequested => {}
            _ => {}
        }
    }

    pub fn update(&mut self, message: EngineSwapMessage, _app_data: &ApplicationData) {
        match message {
            EngineSwapMessage::CarSelected(path_ref) => {
                self.current_car = Some(path_ref.full_path.clone());
            },
            EngineSwapMessage::SourceChanged(e) => {
                self.current_source = e;
                self.current_mod = None;
                self.current_crate_eng = None;
                self.current_new_spec_name = String::from("");
            },
            EngineSwapMessage::ModSelected(path_ref) => {
                const ZIP_PREFIX: &str = ".zip";
                let mut spec_name = path_ref.to_string();
                if spec_name.ends_with(ZIP_PREFIX) {
                    spec_name.truncate(spec_name.len() - ZIP_PREFIX.len())
                }
                self.current_new_spec_name = spec_name;
                self.current_mod = Some(path_ref.full_path.clone())
            },
            EngineSwapMessage::CrateEngineSelected(name) => {
                if let Some(metadata) = _app_data.crate_engine_data.get_metadata_for(&name) {
                    self.current_new_spec_name = metadata.name().to_string()
                } else {
                    self.current_new_spec_name = name.clone();
                }
                self.current_crate_eng = Some(name)
            },
            EngineSwapMessage::NameEntered(new_car_name) => {
                self.current_new_spec_name = new_car_name
            },
            EngineSwapMessage::PhysicsLevelSelected(new_physics_level) => {
                self.current_minimum_physics = new_physics_level;
            }
            EngineSwapMessage::OldEngineWeightEntered(old_weight) => {
                if old_weight.is_empty() {
                    self.current_engine_weight = None;
                    return;
                }
                match old_weight.parse::<u32>() {
                    Ok(_) => {
                        self.current_engine_weight = Some(old_weight);
                    }
                    Err(_) => {
                        self.status_message = "Old weight must be an integer".to_string();
                        self.current_engine_weight = None;
                    }
                }
            }
            EngineSwapMessage::UnpackToggled(bool_val) => {
                self.unpack_physics_data = bool_val;
            }
        }
    }

    pub fn notify_action_success(&mut self, action_event: &Message) {
        match action_event {
            Message::DeleteCrateEngine(_) => {
                self.current_crate_eng = None;
                if self.current_source == EngineSource::CrateEngine {
                    self.current_new_spec_name.clear();
                }
            }
            _ => {}
        }
    }

    pub fn notify_action_failure(&mut self, _action_event: &Message, _reason: &str) {
    }

    pub fn update_status(&mut self, status: String) {
        self.status_message = status;
    }

    pub fn refresh(&mut self) {
        self.current_car = None;
        self.current_mod = None;
        self.current_crate_eng = None
    }
}

impl Tab for EngineSwapTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("Engine Swap")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content<'a, 'b>(&'a self, app_data: &'b ApplicationData ) -> Element<'_, Self::Message, Renderer>
        where 'b: 'a
    {
        let current_car = match &self.current_car {
            None => { None }
            Some(path) => {
                Some(ListPath {full_path: path.clone()})
            }
        };
        let car_select_container = Column::new()
            .push(Text::new("Assetto Corsa car"))
            .push(pick_list(
                &app_data.assetto_corsa_data.available_cars,
                current_car,
                move |val| { Message::EngineSwap(EngineSwapMessage::CarSelected(val)) },
            ));

        let mut source_select_container = Column::new()
            .spacing(3)
            .push(Text::new("Source"));
        let selected_option = self.current_source;
        let source_select_row =
            EngineSource::all_options().iter().fold(
                Row::new()
                    //.padding(Padding::from([0, 10]))
                    .spacing(20)
                    .align_items(Alignment::Start),
                |row, config_choice| {
                    row.push(radio(
                        format!("{config_choice}"),
                        *config_choice,
                        Some(selected_option),
                        move |val| { Message::EngineSwap(EngineSwapMessage::SourceChanged(val)) })
                        .spacing(3).size(20).text_size(18))
                }
            );
        source_select_container = source_select_container.push(source_select_row);

        let engine_source_selector = match self.current_source {
            EngineSource::BeamNGMod => {
                let current_mod = match &self.current_mod {
                    None => { None }
                    Some(path) => {
                        Some(ListPath {full_path: path.clone()})
                    }
                };
                Column::new()
                    .push(Text::new("BeamNG mod"))
                    .push(PickList::new(
                        &app_data.beam_ng_data.available_mods,
                        current_mod,
                        move |val| { Message::EngineSwap(EngineSwapMessage::ModSelected(val)) }
                    ))
            }
            EngineSource::CrateEngine => {
                let current_crate_eng = match &self.current_crate_eng {
                    None => { None }
                    Some(path) => {
                        Some(path.clone())
                    }
                };
                Column::new()
                    .push(Text::new("Crate Engine"))
                    .push(PickList::new(
                        &app_data.crate_engine_data.available_engines,
                        current_crate_eng,
                        move |val| { Message::EngineSwap(EngineSwapMessage::CrateEngineSelected(val)) }
                    ))
            }
        };

        let placeholder = match self.current_new_spec_name.as_str() {
            "" => { "Enter new spec name" }
            s => { s }
        };
        let new_spec_input = TextInput::new(
            placeholder,
            &self.current_new_spec_name,
            move|val| { Message::EngineSwap(EngineSwapMessage::NameEntered(val)) },
        ).width(Length::Units(500));
        let car_name_container = Column::new()
            .push(Text::new("New spec name (this will be appended to the created car)"))
            .push(new_spec_input);

        let current_weight_value = match &self.current_engine_weight {
            None => { "" }
            Some(string) => {
                string.as_str()
            }
        };
        let weight_input_container = Column::new()
            .push(Text::new("Existing engine weight in Kgs (Optional)"))
            .push(TextInput::new(
                "",
                current_weight_value,
                move |val| { Message::EngineSwap(EngineSwapMessage::OldEngineWeightEntered(val)) },
            ).width(Length::Units(100)));
        let select_container = Column::new()
            .spacing(20)
            .push(car_select_container)
            .push(source_select_container)
            .push(engine_source_selector)
            .push(car_name_container)
            .push(weight_input_container);

        let swap_button = Button::new(Text::new("Swap"))
            .width(Length::Units(60))
            .on_press(Message::EngineSwapRequested);
        let physics_pick_list = PickList::new(
            &self.available_physics,
            Some(self.current_minimum_physics),
            move |val| { Message::EngineSwap(EngineSwapMessage::PhysicsLevelSelected(val)) }
        );
        let unpack_checkbox = checkbox(
            "Unpack physics data".to_string(),
            self.unpack_physics_data,
            move |val| { Message::EngineSwap(EngineSwapMessage::UnpackToggled(val)) }
        ).spacing(3);

        let control_row = Row::new()
            .align_items(Alignment::Center)
            .padding(Padding::from([10, 0]))
            .spacing(10)
            .push(swap_button)
            .push(physics_pick_list)
            .push(unpack_checkbox);

        let mut layout = Column::new().width(Length::Fill)
            .align_items(Alignment::Start)
            .padding(Padding::from([0, 10]))
            .spacing(30)
            .push(select_container)
            .push(control_row);

        if !self.status_message.is_empty() {
            layout = layout.push(
                Row::new()
                    .align_items(Alignment::Center)
                    .push(Text::new(self.status_message.as_str()).horizontal_alignment(Horizontal::Center))
            )
        }
        Container::new(layout).padding(20).into()
    }
}

