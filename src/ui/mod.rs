use std::path::{Path, PathBuf};
use iced::{Column, Element, Length, pick_list, PickList, Sandbox, Align, Text, Settings, Error, text_input, TextInput, Row, button, Button, HorizontalAlignment, Checkbox};
use crate::{assetto_corsa, beam_ng, fabricator};
use crate::fabricator::{AdditionalAcCarData, AssettoCorsaCarSettings, AssettoCorsaPhysicsLevel};
use tracing::{span, Level, info, error};

pub fn launch() -> Result<(), Error> {
    CarSelector::run((Settings::default()))
}

#[derive(Default)]
pub struct CarSelector {
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
    minimum_physics_pick_list: pick_list::State<AssettoCorsaPhysicsLevel>,
    current_engine_weight_input: text_input::State,
    unpack_physics_data: bool,
    status_message: String
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
    SwapButtonPressed
}

fn to_filename_vec(path_vec: &Vec<PathBuf>) -> Vec<String> {
    path_vec.iter().map(|path|{
        String::from(path.file_name().unwrap().to_string_lossy())
    }).collect()
}

impl Sandbox for CarSelector {
    type Message = Message;

    fn new() -> Self {
        let available_mods = {
            let span = span!(Level::INFO, "Loading beamNG mods");
            let _enter = span.enter();
            let mods = to_filename_vec(&beam_ng::get_mod_list());
            info!("Found {} mods", mods.len());
            mods
        };
        let mut available_cars = {
            let span = span!(Level::INFO, "Loading Assetto Corsa cars");
            let _enter = span.enter();
            let cars = to_filename_vec(&assetto_corsa::get_list_of_installed_cars().unwrap());
            info!("Found {} cars", cars.len());
            cars
        };
        available_cars.sort();
        CarSelector {
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

                let mut mod_path = beam_ng::get_mod_path().unwrap();
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
        }
    }

    fn view(&mut self) -> Element<Message> {
        let car_select_container = Column::new()
            .align_items(Align::Center)
            //.padding(10)
            .push(Text::new("Assetto Corsa car"))
            .push(PickList::new(
                &mut self.car_pick_list,
                &self.available_cars,
                self.current_car.clone(),
                Message::CarSelected,
            ));
        let mod_select_container = Column::new()
            .align_items(Align::Center)
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
            .align_items(Align::Center)
            .padding(10)
            .push(Text::new("New spec name (this will be appended to the created car)"))
            .push(input);
        let selection_row = Row::new()
            .align_items(Align::Center)
            .push(select_container.width(Length::FillPortion(1)))
            .push(car_name_container.width(Length::FillPortion(1)));

        let swap_button = Button::new(&mut self.swap_button, Text::new("Swap"))
            .min_width(60)
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
        let control_row = Row::new()
            .align_items(Align::Start)
            .padding(20)
            .push(swap_button)
            .push(physics_pick_list)
            .push(unpack_checkbox);

        let mut layout = Column::new().width(Length::Fill)
            .align_items(Align::Start)
            .padding(10)
            .spacing(30)
            .push(selection_row)
            .push(control_row);

        if !self.status_message.is_empty() {
            layout = layout.push(
                Row::new()
                    .align_items(Align::Center)
                    .push(Text::new(self.status_message.as_str()).horizontal_alignment(HorizontalAlignment::Center))
            )
        }
        layout.into()
    }
}
