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

    if assetto_corsa::is_installed() {
        println!("Assetto Corsa is installed");
        println!("Installed cars can be found at {}",
                 assetto_corsa::get_installed_cars_path().unwrap().display())
    } else {
        println!("Assetto Corsa is not installed");
        return;
    }

    for path in assetto_corsa::get_list_of_installed_cars().unwrap() {
        let path_obj = Path::new(path.as_os_str());
        if !path_obj.join("data").is_dir() {
            continue;
        }
        println!("{}", path_obj.display());
        let car = assetto_corsa::car::Car::load_from_path(Path::new(path.as_os_str())).unwrap();
        println!("{:?}", car.ui_info.specs().unwrap());
        println!("{:?}", car.ui_info.torque_curve().unwrap());
        break;
    }

    if automation::is_installed() {
        println!("Automation is installed");
    } else {
        println!("Automation is not installed");
        return;
    }

    println!("BeamNG mod folder resolved to {}", beam_ng::get_mod_path().unwrap().display());
    match beam_ng::get_mod_list() {
        Some(list) => {
            println!("Found BeamNG mods:");
            for beam_mod in list {
                println!("{}", Path::new(beam_mod.as_os_str()).display());
                let x = beam_ng::extract_mod_data(&beam_mod).unwrap();
                for (filename, data) in x {
                    println!("{}: {:x?}", filename, data);
                    if filename.ends_with(".car") {
                        let c = automation::car::CarFile::from_bytes(data);
                        println!("Car file: {:?}", c);
                    } else if filename.ends_with(".jbeam") {
                        let jbeam_data = beam_ng::jbeam::from_slice(&*data).unwrap();
                        println!("inertia: {:?}", jbeam_data["Camso_Engine"].as_object().unwrap()["mainEngine"].as_object().unwrap()["inertia"]);
                    }
                }
            }
        },
        None => println!("No BeamNG mods found")
    };


    let sandox_str = automation::sandbox::get_db_path().unwrap();
    println!("Automation sandbox.db found at {}", Path::new(sandox_str.as_os_str()).display());
}
