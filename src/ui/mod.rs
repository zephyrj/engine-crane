use std::borrow::Borrow;
use std::collections::HashMap;
use std::ops::Add;
use std::path::{Path, PathBuf};
use iced::{Column, Element, Length, pick_list, PickList, Sandbox, Align, Text, Settings, Error, text_input, TextInput, Row, button, Button, Color, HorizontalAlignment};
use crate::{assetto_corsa, beam_ng, fabricator};
use crate::automation;

pub fn launch() -> Result<(), Error> {
    CarSelector::run((Settings::default()))
}

#[derive(Default)]
pub struct CarSelector {
    available_cars: Vec<String>,
    available_mods: Vec<String>,
    current_car: Option<String>,
    current_mod: Option<String>,
    current_new_spec_name: String,
    car_pick_list: pick_list::State<String>,
    new_spec_name: text_input::State,
    mod_pick_list: pick_list::State<String>,
    swap_button: button::State,
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
        let mods = to_filename_vec(&beam_ng::get_mod_list());
        CarSelector {
            available_cars: to_filename_vec(&assetto_corsa::get_list_of_installed_cars().unwrap()),
            available_mods: mods,
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
            Message::SwapButtonPressed => {
                if self.current_car.is_none() {
                    self.status_message = String::from("Please select an Assetto Corsa car");
                    return;
                } else if self.current_mod.is_none() {
                    self.status_message = String::from("Please select an BeamNG mod");
                    return;
                }
                let new_car_path = assetto_corsa::car::create_new_car_spec((&self.current_car).as_ref().unwrap().as_str(),
                                                                           self.current_new_spec_name.as_str()).unwrap();
                let mut mod_path = beam_ng::get_mod_path().unwrap();
                if let Some(mod_name) = &self.current_mod {
                    mod_path = mod_path.join(Path::new(mod_name.as_str()));
                }
                match fabricator::swap_automation_engine_into_ac_car(mod_path.as_path(), new_car_path.as_path()) {
                    Ok(_) => { self.status_message = format!("Created {} successfully", new_car_path.display()) }
                    Err(err_str) => { self.status_message = err_str }
                }
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let placeholder = match self.current_new_spec_name.as_str() {
            "" => { "Enter new car name" }
            s => { s }
        };
        let input = TextInput::new(
            &mut self.new_spec_name,
            placeholder,
            &self.current_new_spec_name,
            Message::NameEntered,
        );
        let car_name_container = Column::new()
            .align_items(Align::Center)
            .padding(10)
            .push(Text::new("New spec name (this will be appended to the created car)"))
            .push(input);

        let car_select_container = Column::new()
            .align_items(Align::Center)
            .padding(10)
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
        let select_container = Column::new()
            .align_items(Align::Center)
            .spacing(10)
            .push(car_select_container)
            .push(mod_select_container);

        let selection_row = Row::new()
            .align_items(Align::Center)
            .push(select_container)
            .push(car_name_container);

        let swap_button = Button::new(&mut self.swap_button, Text::new("Swap"))
            .min_width(60)
            .on_press(Message::SwapButtonPressed);
        let control_row = Row::new()
            .align_items(Align::Start)
            .padding(20)
            .push(swap_button);

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
