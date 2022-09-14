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

use std::path::{Path, PathBuf};
use iced::{Column, Element, Length, pick_list, PickList, Sandbox, Alignment, Text, Settings, Error, text_input, TextInput, Row, button, Button, Checkbox};
use iced::alignment::Horizontal;
use crate::{assetto_corsa, beam_ng, fabricator};
use crate::fabricator::{AdditionalAcCarData, AssettoCorsaCarSettings, AssettoCorsaPhysicsLevel};
use tracing::{span, Level, info, error};
use rfd::FileDialog;


pub fn launch() -> Result<(), Error> {
    CarSelector::run((Settings::default()))
}

#[derive(Default)]
pub struct CarSelector {
    ac_install_path: Option<String>,
    beamng_mod_path: Option<String>,
    available_cars: Vec<String>,
    available_mods: Vec<String>,
    available_physics: Vec<AssettoCorsaPhysicsLevel>,
    current_car: Option<String>,
    current_mod: Option<String>,
    current_new_spec_name: String,
    current_engine_weight: Option<String>,
    current_minimum_physics: AssettoCorsaPhysicsLevel,
    car_pick_list: pick_list::State<String>,
    new_spec_name: text_input::State,
    mod_pick_list: pick_list::State<String>,
    swap_button: button::State,
    ac_path_select_button: button::State,
    beamng_mod_path_select_button: button::State,
    minimum_physics_pick_list: pick_list::State<AssettoCorsaPhysicsLevel>,
    current_engine_weight_input: text_input::State,
    unpack_physics_data: bool,
    status_message: String
}

impl CarSelector {
    fn load_available_cars(ac_install_path: &PathBuf) -> Vec<String> {
        let span = span!(Level::INFO, "Loading Assetto Corsa cars");
        let _enter = span.enter();
        return match &assetto_corsa::get_list_of_installed_cars_for(ac_install_path) {
            Ok(vec) => {
                info!("Found {} cars", vec.len());
                to_filename_vec(vec)
            }
            Err(err) => {
                error!("{}", err.to_string());
                Vec::new()
            }
        }
    }

    fn load_available_mods(beamng_mod_path: &PathBuf) -> Vec<String> {
        let span = span!(Level::INFO, "Loading beamNG mods");
        let _enter = span.enter();
        let mods = to_filename_vec(&beam_ng::get_mod_list_for(beamng_mod_path));
        info!("Found {} mods", mods.len());
        mods
    }

    fn set_ac_install_path(&mut self, ac_install_path: &PathBuf) {
        self.current_car = None;
        self.available_cars = Self::load_available_cars(ac_install_path);
        self.available_cars.sort();
        self.ac_install_path = Some(String::from(ac_install_path.to_string_lossy()));
    }

    fn set_beam_ng_mod_path(&mut self, beam_ng_mod_path: &PathBuf) {
        self.current_mod = None;
        self.available_mods = Self::load_available_mods(beam_ng_mod_path);
        self.beamng_mod_path = Some(String::from(beam_ng_mod_path.to_string_lossy()));
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct EngineRef {
    uid: String,
    display_name: String
}

impl std::fmt::Display for EngineRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    CarSelected(String),
    NameEntered(String),
    ModSelected(String),
    PhysicsLevelSelected(AssettoCorsaPhysicsLevel),
    OldEngineWeightEntered(String),
    UnpackToggled(bool),
    SwapButtonPressed,
    AcPathSelectPressed,
    BeamNGModPathSelectPressed
}

fn to_filename_vec(path_vec: &Vec<PathBuf>) -> Vec<String> {
    path_vec.iter().map(|path|{
        String::from(path.file_name().unwrap().to_string_lossy())
    }).collect()
}

impl Sandbox for CarSelector {
    type Message = Message;

    fn new() -> Self {
        let mut ac_install_path: Option<String> = None;
        let mut available_cars = Vec::new();
        if let Some(base_path) = assetto_corsa::get_default_install_path() {
            available_cars = Self::load_available_cars(&base_path);
            available_cars.sort();
            ac_install_path = Some(base_path.to_string_lossy().into_owned());
        }
        let mut beamng_mod_path: Option<String> = None;
        let mut available_mods = Vec::new();
        if let Some(mod_path) = beam_ng::get_default_mod_path() {
            available_mods = Self::load_available_mods(&mod_path);
            beamng_mod_path = Some(mod_path.to_string_lossy().into_owned());
        }

        CarSelector {
            ac_install_path,
            beamng_mod_path,
            available_cars,
            available_mods,
            available_physics: vec![AssettoCorsaPhysicsLevel::BaseGame, AssettoCorsaPhysicsLevel::CspExtendedPhysics],
            ..Default::default() }
    }

    fn title(&self) -> String {
        String::from("Engine Crane")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::CarSelected(car_path) => {
                self.current_car = Some(car_path);
            },
            Message::ModSelected(mod_name) => {
                self.current_new_spec_name = String::from(mod_name.strip_suffix(".zip").unwrap());
                self.current_mod = Some(mod_name)
            },
            Message::NameEntered(new_car_name) => {
                self.current_new_spec_name = new_car_name
            },
            Message::PhysicsLevelSelected(new_physics_level) => {
                self.current_minimum_physics = new_physics_level;
            }
            Message::SwapButtonPressed => {
                if self.current_car.is_none() {
                    self.status_message = String::from("Please select an Assetto Corsa car");
                    return;
                } else if self.current_mod.is_none() {
                    self.status_message = String::from("Please select an BeamNG mod");
                    return;
                }

                let existing_car_name = (&self.current_car).as_ref().unwrap().as_str();
                let new_spec_name = self.current_new_spec_name.as_str();

                let new_car_path = {
                    let span = span!(Level::INFO, "Creating new car spec");
                    let _enter = span.enter();
                    match assetto_corsa::car::create_new_car_spec(existing_car_name, new_spec_name, self.unpack_physics_data) {
                        Ok(path) => { path }
                        Err(e) => {
                            error!("Swap failed: {}", e.to_string());
                            self.status_message = format!("Swap failed: {}", e.to_string());
                            return;
                        }
                    }
                };

                let mut mod_path = beam_ng::get_default_mod_path().unwrap();
                if let Some(mod_name) = &self.current_mod {
                    mod_path = mod_path.join(Path::new(mod_name.as_str()));
                }

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
            Message::OldEngineWeightEntered(old_weight) => {
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
            Message::UnpackToggled(bool_val) => {
                self.unpack_physics_data = bool_val;
            }
            Message::AcPathSelectPressed => {
                let install_path = FileDialog::new()
                    .set_directory(match &self.ac_install_path {
                        Some(str) => str,
                        None => "/"
                    })
                    .pick_folder();
                match install_path {
                    Some(p) => {
                        self.status_message.clear();
                        self.set_ac_install_path(&p);
                        if self.available_cars.is_empty() {
                            self.status_message = format!("No cars found")
                        }
                    },
                    None => self.status_message = format!("No path selected")
                };
            }
            Message::BeamNGModPathSelectPressed => {
                let mod_path = FileDialog::new()
                    .set_directory(match &self.beamng_mod_path {
                        Some(str) => str,
                        None => "/"
                    })
                    .pick_folder();
                match mod_path {
                    Some(p) => {
                        self.status_message.clear();
                        self.set_beam_ng_mod_path(&p);
                        if self.available_mods.is_empty() {
                            self.status_message = format!("No mods found")
                        }
                    },
                    None => self.status_message = format!("No path selected")
                };
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let car_select_container = Column::new()
            .align_items(Alignment::Center)
            //.padding(10)
            .push(Text::new("Assetto Corsa car"))
            .push(PickList::new(
                &mut self.car_pick_list,
                &self.available_cars,
                self.current_car.clone(),
                Message::CarSelected,
            ));
        let mod_select_container = Column::new()
            .align_items(Alignment::Center)
            .push(Text::new("BeamNG mod"))
            .push(PickList::new(
                &mut self.mod_pick_list,
                &self.available_mods,
                self.current_mod.clone(),
                Message::ModSelected
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
                Message::OldEngineWeightEntered,
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
            Message::NameEntered,
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
            .on_press(Message::SwapButtonPressed);
        let physics_pick_list = PickList::new(
            &mut self.minimum_physics_pick_list,
            &self.available_physics,
            Some(self.current_minimum_physics),
            Message::PhysicsLevelSelected
        );
        let unpack_checkbox = Checkbox::new(
            self.unpack_physics_data,
            "Unpack physics data",
            Message::UnpackToggled
        );

        let base_path_str = match &self.ac_install_path {
            None => format!("Assetto Corsa install path: Not Set"),
            Some(path) => format!("Assetto Corsa install path: {}", path)
        };
        let path_select_button =
            Button::new(&mut self.ac_path_select_button, Text::new("Browse"))
                .on_press(Message::AcPathSelectPressed);
        let ac_path_select_row = Row::new()
            .align_items(Alignment::Start)
            .padding([10, 0, 1, 10])
            .spacing(20)
            .push(Text::new(base_path_str))
            .push(path_select_button);

        let mod_path_str = match &self.beamng_mod_path {
            None => format!("BeamNG mod path: Not Set"),
            Some(path) => format!("BeamNG mod path: {}", path)
        };
        let mod_path_select_button =
            Button::new(&mut self.beamng_mod_path_select_button, Text::new("Browse"))
                .on_press(Message::BeamNGModPathSelectPressed);
        let mod_path_select_row = Row::new()
            .align_items(Alignment::Start)
            .padding([1, 10, 2, 10])
            .spacing(20)
            .push(Text::new(mod_path_str))
            .push(mod_path_select_button);

        let control_row = Row::new()
            .align_items(Alignment::Start)
            .padding(10)
            .spacing(10)
            .push(swap_button)
            .push(physics_pick_list)
            .push(unpack_checkbox);

        let mut layout = Column::new().width(Length::Fill)
            .align_items(Alignment::Start)
            .padding(10)
            .spacing(30)
            .push(ac_path_select_row)
            .push(mod_path_select_row)
            .push(iced::Rule::horizontal(5))
            .push(selection_row)
            .push(control_row);

        if !self.status_message.is_empty() {
            layout = layout.push(
                Row::new()
                    .align_items(Alignment::Center)
                    .push(Text::new(self.status_message.as_str()).horizontal_alignment(Horizontal::Center))
            )
        }
        layout.into()
    }
}
