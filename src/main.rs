/*
 * Copyright (c):
 * 2023 zephyrj
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



mod ui;
mod data;
mod fabricator;

use std::env;
use std::path::PathBuf;
use tracing_subscriber;
use tracing_appender;
use tracing::{info};

use assetto_corsa;
use automation;
use beam_ng;
use utils;


// -> Result<(), iced::Error>
fn main() -> Result<(), iced::Error> {
    match env::current_dir() {
        Ok(current_dir) => {
            let file_appender = tracing_appender::rolling::never(current_dir, "engine_crane.log");
            let subscriber = tracing_subscriber::fmt()
                .with_writer(file_appender)
                .with_ansi(false)
                .compact()
                .finish();
            match tracing::subscriber::set_global_default(subscriber) {
                Ok(_) => {
                    info!("Logging initialised");
                }
                Err(e) => {
                    eprintln!("Failed to init logging. {}", e.to_string());
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to init logging. Couldn't determine current dir {}", e.to_string());
        }
    }

    if let Some(legacy_db_path) = automation::sandbox::get_db_path() {
        info!("Automation sandbox.db for game version < 4.2 found at {}", PathBuf::from(legacy_db_path).display())
    }
    if let Some(db_path) = automation::sandbox::get_db_path_4_2() {
        info!("Automation sandbox.db for game version == 4.2 found at {}", PathBuf::from(db_path).display())
    }
    if let Some(db_path) = automation::sandbox::get_db_path_ellisbury() {
        info!("Automation sandbox.db for game version >= 4.3 found at {}", PathBuf::from(db_path).display())
    }

    info!("Launching UI");
    ui::launch()
}


// println!("BeamNG mod folder resolved to {}", beam_ng::get_mod_path().unwrap().display());
// match beam_ng::get_mod_list() {
//     Some(list) => {
//         println!("Found BeamNG mods:");
//         for beam_mod in list {
//             println!("{}", Path::new(beam_mod.as_os_str()).display());
//             let x = beam_ng::extract_mod_data(&beam_mod).unwrap();
//             for (filename, data) in x {
//                 println!("{}: {:x?}", filename, data);
//                 if filename.ends_with(".car") {
//                     let c = automation::car::CarFile::from_bytes(data);
//                     println!("Car file: {:?}", c);
//                 } else if filename.ends_with(".jbeam") {
//                     let jbeam_data = beam_ng::jbeam::from_slice(&*data).unwrap();
//                     println!("inertia: {:?}", jbeam_data["Camso_Engine"].as_object().unwrap()["mainEngine"].as_object().unwrap()["inertia"]);
//                 }
//             }
//         }
//     },
//     None => println!("No BeamNG mods found")
// };


//
// if let Some(variants) = automation::sandbox::get_engine_names() {
//     for name in variants {
//         println!("Found variant: {}", name);
//     }
// }
//
//
//

//
// let eng_results = automation::sandbox::load_engines();
// for (uuid, eng) in eng_results {
//     println!("Found engine: {:?}", eng);
// }