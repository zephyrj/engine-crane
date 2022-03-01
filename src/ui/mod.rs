use std::collections::HashMap;
use std::path::Path;
use iced::{Column, Element, Length, pick_list, PickList, Sandbox, Align, Text, Settings, Error};
use crate::assetto_corsa;
use crate::automation;

pub fn launch() -> Result<(), Error> {
    CarSelector::run((Settings::default()))
}

#[derive(Default)]
pub struct CarSelector {
    available_cars: Vec<String>,
    available_engines: HashMap<String, automation::sandbox::EngineV1>,
    engine_ref: Vec<EngineRef>,
    current_car: Option<String>,
    current_engine: Option<EngineRef>,
    car_pick_list: pick_list::State<String>,
    engine_pick_list: pick_list::State<EngineRef>
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
    EngineSelected(EngineRef)
}

impl Sandbox for CarSelector {
    type Message = Message;

    fn new() -> Self {
        let engines = automation::sandbox::load_engines();
        let engine_refs: Vec<EngineRef> = engines.iter().map(|(uid, eng)| {
            EngineRef { uid: String::from(uid), display_name: eng.friendly_name() }
        }).collect();
        CarSelector {
            available_cars: assetto_corsa::get_list_of_installed_cars().unwrap()
            .iter()
            .map(|car_path| String::from(Path::new(car_path.as_os_str()).file_name().unwrap().to_str().unwrap()))
            .collect(),
            available_engines: engines,
            engine_ref: engine_refs,
            ..Default::default() }
    }

    fn title(&self) -> String {
        String::from("Engine Crane")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::CarSelected(car_path) => {
                self.current_car = Some(car_path)
            }
            Message::EngineSelected(engine_ref) => {
                self.current_engine = Some(engine_ref)
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
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

        let engine_select_container = Column::new()
            .align_items(Align::Center)
            .push(Text::new("Automation engine"))
            .push(PickList::new(
                &mut self.engine_pick_list,
                &self.engine_ref,
                self.current_engine.clone(),
                Message::EngineSelected
            ));

        Column::new().width(Length::Fill)
            .align_items(Align::Center)
            .spacing(10)
            .push(car_select_container)
            .push(engine_select_container)
            .into()
    }
}
