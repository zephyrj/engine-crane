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
mod automation;
mod ui;
mod beam_ng;

use std::ffi::OsString;
use std::path::Path;

// -> Result<(), iced::Error>
fn main()  {
    //ui::launch()

    match beam_ng::get_mod_list() {
        Some(list) => {
            println!("Found BeamNG mods:");
            for beam_mod in list {
                println!("{}", Path::new(beam_mod.as_os_str()).display());
                let x = beam_ng::extract_data(&beam_mod).unwrap();
                for (filename, data) in x {
                    println!("{}: {:x?}", filename, data);
                    if filename.ends_with(".car") {
                        let c = automation::car::CarFile::from_bytes(data);
                        println!("Car file: {:?}", c);
                    }
                }
            }
        },
        None => println!("No BeamNG mods found")
    };

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

    // let sandox_str = automation::get_sandbox_db_path().unwrap();
    // println!("Automation sandbox.db found at {}", Path::new(sandox_str.as_os_str()).display());
}
