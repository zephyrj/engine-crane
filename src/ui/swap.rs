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

use super::{Message, Tab};
use std::path::{PathBuf};
use iced::{Alignment, button, Button, Checkbox, Column, Container, Element, Length, pick_list, PickList, Row, Text, text_input, TextInput};
use iced_aw::{TabLabel};
use iced::alignment::Horizontal;
use tracing::{span, Level, error};
use crate::{assetto_corsa, fabricator};
use crate::fabricator::{AdditionalAcCarData, AssettoCorsaCarSettings, AssettoCorsaPhysicsLevel};
use crate::ui::{ApplicationData, ListPath};

#[derive(Debug, Clone)]
pub enum EngineSwapMessage {
    CarSelected(ListPath),
    NameEntered(String),
    ModSelected(ListPath),
    PhysicsLevelSelected(AssettoCorsaPhysicsLevel),
    OldEngineWeightEntered(String),
    UnpackToggled(bool),
    SwapButtonPressed
}

#[derive(Default)]
pub struct EngineSwapTab {
    available_physics: Vec<AssettoCorsaPhysicsLevel>,
    current_car: Option<PathBuf>,
    current_mod: Option<PathBuf>,
    current_new_spec_name: String,
    current_engine_weight: Option<String>,
    current_minimum_physics: AssettoCorsaPhysicsLevel,
    car_pick_list: pick_list::State<ListPath>,
    new_spec_name: text_input::State,
    mod_pick_list: pick_list::State<ListPath>,
    swap_button: button::State,
    minimum_physics_pick_list: pick_list::State<AssettoCorsaPhysicsLevel>,
    current_engine_weight_input: text_input::State,
    unpack_physics_data: bool,
    status_message: String
}

impl EngineSwapTab {
    pub(crate) fn new() -> Self {
        EngineSwapTab {
            available_physics: vec![AssettoCorsaPhysicsLevel::BaseGame],
            current_car: None,
            current_mod: None,
            current_new_spec_name: "".to_string(),
            current_engine_weight: None,
            current_minimum_physics: Default::default(),
            car_pick_list: Default::default(),
            new_spec_name: Default::default(),
            mod_pick_list: Default::default(),
            swap_button: Default::default(),
            minimum_physics_pick_list: Default::default(),
            current_engine_weight_input: Default::default(),
            unpack_physics_data: false,
            status_message: "".to_string()
        }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData) {
        self.refresh();
    }

    pub fn update(&mut self, message: EngineSwapMessage, app_data: &ApplicationData) {
        match message {
            EngineSwapMessage::CarSelected(path_ref) => {
                self.current_car = Some(path_ref.full_path.clone());
            },
            EngineSwapMessage::ModSelected(path_ref) => {
                self.current_new_spec_name = String::from(path_ref.to_string().strip_suffix(".zip").unwrap());
                self.current_mod = Some(path_ref.full_path.clone())
            },
            EngineSwapMessage::NameEntered(new_car_name) => {
                self.current_new_spec_name = new_car_name
            },
            EngineSwapMessage::PhysicsLevelSelected(new_physics_level) => {
                self.current_minimum_physics = new_physics_level;
            }
            EngineSwapMessage::SwapButtonPressed => {
                if app_data.get_ac_install_path().is_none() {
                    self.status_message = String::from("Please set the Assetto Corsa install path in the settings tab");
                    return;
                }
                if self.current_car.is_none() {
                    self.status_message = String::from("Please select an Assetto Corsa car");
                    return;
                } else if self.current_mod.is_none() {
                    self.status_message = String::from("Please select an BeamNG mod");
                    return;
                }

                let new_spec_name = self.current_new_spec_name.as_str();
                let new_car_path = {
                    let span = span!(Level::INFO, "Creating new car spec");
                    let _enter = span.enter();
                    let ac_install = assetto_corsa::Installation::from_path(
                        app_data.get_ac_install_path().as_ref().unwrap().clone()
                    );
                    match assetto_corsa::car::create_new_car_spec(&ac_install,
                                                                  self.current_car.as_ref().unwrap(),
                                                                  new_spec_name,
                                                                  self.unpack_physics_data)
                    {
                        Ok(path) => { path }
                        Err(e) => {
                            error!("Swap failed: {}", e.to_string());
                            self.status_message = format!("Swap failed: {}", e.to_string());
                            return;
                        }
                    }
                };

                let mod_path = self.current_mod.as_ref().unwrap();
                {
                    let span = span!(Level::INFO, "Updating car physics");
                    let _enter = span.enter();
                    let current_engine_weight =
                        if let Some(weight_string) = &self.current_engine_weight {
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
                    match fabricator::swap_automation_engine_into_ac_car(mod_path.as_path(),
                                                                         new_car_path.as_path(),
                                                                         AssettoCorsaCarSettings::from_physics_level(self.current_minimum_physics),
                                                                         AdditionalAcCarData::new(current_engine_weight)) {
                        Ok(_) => { self.status_message = format!("Created {} successfully", new_car_path.display()) }
                        Err(err_str) => { self.status_message = err_str }
                    }
                }

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
                        self.status_message = format!("Old weight must be an integer");
                        self.current_engine_weight = None;
                    }
                }
            }
            EngineSwapMessage::UnpackToggled(bool_val) => {
                self.unpack_physics_data = bool_val;
            }
        }
    }

    pub fn refresh(&mut self) {
        self.current_car = None;
        self.current_mod = None;
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

    fn content<'a, 'b>(&'a mut self,
                       app_data: &'b ApplicationData ) -> Element<'_, Self::Message>
    where 'b: 'a
    {
        let current_car = match &self.current_car {
            None => { None }
            Some(path) => {
                Some(ListPath {full_path: path.clone()})
            }
        };
        let car_select_container = Column::new()
            .align_items(Alignment::Center)
            //.padding(10)
            .push(Text::new("Assetto Corsa car"))
            .push(PickList::new(
                &mut self.car_pick_list,
                &app_data.assetto_corsa_data.available_cars,
                current_car,
                EngineSwapMessage::CarSelected,
            ));
        let current_mod = match &self.current_mod {
            None => { None }
            Some(path) => {
                Some(ListPath {full_path: path.clone()})
            }
        };
        let mod_select_container = Column::new()
            .align_items(Alignment::Center)
            .push(Text::new("BeamNG mod"))
            .push(PickList::new(
                &mut self.mod_pick_list,
                &app_data.beam_ng_data.available_mods,
                current_mod,
                EngineSwapMessage::ModSelected
            ));
        let current_weight_value = match &self.current_engine_weight {
            None => { "" }
            Some(string) => {
                string.as_str()
            }
        };
        let weight_input_container = Column::new()
            //.align_items(Align::Center)
            .push(Text::new("Existing engine weight in Kgs (Optional)"))
            .push(TextInput::new(
                &mut self.current_engine_weight_input,
                "",
                current_weight_value,
                EngineSwapMessage::OldEngineWeightEntered,
            ).width(Length::Units(100)));
        let select_container = Column::new()
            //.align_items(Align::)
            .padding(10)
            .spacing(20)
            .push(car_select_container)
            .push(mod_select_container)
            .push(weight_input_container);

        let placeholder = match self.current_new_spec_name.as_str() {
            "" => { "Enter new spec name" }
            s => { s }
        };
        let input = TextInput::new(
            &mut self.new_spec_name,
            placeholder,
            &self.current_new_spec_name,
            EngineSwapMessage::NameEntered,
        ).width(Length::Units(500));
        let car_name_container = Column::new()
            .align_items(Alignment::Center)
            .padding(10)
            .push(Text::new("New spec name (this will be appended to the created car)"))
            .push(input);
        let selection_row = Row::new()
            .align_items(Alignment::Center)
            .push(select_container.width(Length::FillPortion(1)))
            .push(car_name_container.width(Length::FillPortion(1)));

        let swap_button = Button::new(&mut self.swap_button, Text::new("Swap"))
            .width(Length::Units(60))
            .on_press(EngineSwapMessage::SwapButtonPressed);
        let physics_pick_list = PickList::new(
            &mut self.minimum_physics_pick_list,
            &self.available_physics,
            Some(self.current_minimum_physics),
            EngineSwapMessage::PhysicsLevelSelected
        );
        let unpack_checkbox = Checkbox::new(
            self.unpack_physics_data,
            "Unpack physics data",
            EngineSwapMessage::UnpackToggled
        );

        let control_row = Row::new()
            .align_items(Alignment::Start)
            .padding(10)
            .spacing(10)
            .push(swap_button)
            .push(physics_pick_list)
            .push(unpack_checkbox);

        let mut layout = Column::new().width(Length::Fill)
            .align_items(Alignment::Start)
            //.padding(10)
            .spacing(30)
            .push(selection_row)
            .push(control_row);

        if !self.status_message.is_empty() {
            layout = layout.push(
                Row::new()
                    .align_items(Alignment::Center)
                    .push(Text::new(self.status_message.as_str()).horizontal_alignment(Horizontal::Center))
            )
        }
        let content : Element<'_, EngineSwapMessage> = Container::new(layout).into();
        content.map(Message::EngineSwap)
    }
}

