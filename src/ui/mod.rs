use std::path::Path;
use iced::{Column, Element, Length, pick_list, PickList, Sandbox, Align, Text, Settings, Error};
use crate::assetto_corsa;

pub fn launch() -> Result<(), Error> {
    CarSelector::run((Settings::default()))
}

#[derive(Default)]
pub struct CarSelector {
    available_cars: Vec<String>,
    current_car: Option<String>,
    pick_list: pick_list::State<String>
}

#[derive(Debug, Clone)]
pub enum Message {
    CarSelected(String),
}

impl Sandbox for CarSelector {
    type Message = Message;

    fn new() -> Self {
        CarSelector { available_cars: assetto_corsa::get_list_of_installed_cars().unwrap()
            .iter()
            .map(|car_path| String::from(Path::new(car_path.as_os_str()).file_name().unwrap().to_str().unwrap()))
            .collect(),
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
        }
    }

    fn view(&mut self) -> Element<Message> {
        let pick_list = PickList::new(
            &mut self.pick_list,
            &self.available_cars,
            self.current_car.clone(),
            Message::CarSelected,
        );

        Column::new().width(Length::Fill)
            .align_items(Align::Center)
            .spacing(10)
            .push(Text::new("Assetto Corsa car"))
            .push(pick_list).into()
    }
}
