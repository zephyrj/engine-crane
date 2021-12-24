/*
Copyright (c):
2021 zephyrj
zephyrj@protonmail.com

This file is part of engine-crane.

engine-crane is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

engine-crane is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
*/
mod assetto_corsa;
mod steam;
mod beam_ng;
mod automation;

use std::ffi::OsString;
use std::path::Path;
use iced::{Column, Element, Length, pick_list, PickList, Sandbox, Align, Text, Settings};

#[derive(Default)]
struct CarSelector {
    available_cars: Vec<String>,
    current_car: Option<String>,
    pick_list: pick_list::State<String>
}

#[derive(Debug, Clone)]
enum Message {
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

fn main() {
    //CarSelector::run(Settings::default())
    let sandox_str = automation::get_sandbox_db_path().unwrap();
    println!("Automation sandbox.db found at {}", Path::new(sandox_str.as_os_str()).display());

    // if assetto_corsa::is_installed() {
    //     println!("Assetto Corsa is installed");
    //     println!("Installed cars can be found at {}",
    //              assetto_corsa::get_installed_cars_path().unwrap().display())
    // } else {
    //     println!("Assetto Corsa is not installed");
    //     return;
    // }
    //
    // println!("Cars installed:");
    // for car in assetto_corsa::get_list_of_installed_cars().unwrap() {
    //     println!("{}", Path::new(car.as_os_str()).display());
    // }
    //
    // if automation::is_installed() {
    //     println!("Automation is installed");
    // } else {
    //     println!("Automation is not installed");
    //     return;
    // }
    //
    // println!("BeamNG mod folder resolved to {}", beam_ng::get_mod_path().unwrap().display());
    //
    // let car_path = Path::new("C:\\Program Files (x86)\\Steam\\steamapps\\common\\assettocorsa\\content\\cars\\abarth500");
    // let car = assetto_corsa::car::Car::load_from_path(car_path).unwrap();
    // println!("{:?}", car.ui_info.specs().unwrap());
    // println!("{:?}", car.ui_info.torque_curve().unwrap());
}
