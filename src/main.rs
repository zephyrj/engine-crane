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
along with sim-racing-tools. If not, see <https://www.gnu.org/licenses/>.
*/
mod assetto_corsa;
mod steam;
mod automation;
mod beam_ng;

use std::path::Path;

fn main() {
    if assetto_corsa::is_installed() {
        println!("Assetto Corsa is installed");
        println!("Installed cars can be found at {}",
                 assetto_corsa::get_installed_cars_path().unwrap().display())
    } else {
        println!("Assetto Corsa is not installed");
        return;
    }

    if automation::is_installed() {
        println!("Automation is installed");
    } else {
        println!("Automation is not installed");
        return;
    }

    println!("BeamNG mod folder resolved to {}", beam_ng::get_mod_path().unwrap().display());

    let car_path = Path::new("C:\\Program Files (x86)\\Steam\\steamapps\\common\\assettocorsa\\content\\cars\\abarth500");
    let car = assetto_corsa::car::Car::load_from_path(car_path).unwrap();
    println!("{:?}", car.ui_info.specs().unwrap());
    println!("{:?}", car.ui_info.torque_curve().unwrap());
}
