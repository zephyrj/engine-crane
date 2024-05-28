/*
 * Copyright (c):
 * 2024 zephyrj
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

use std::collections::{BTreeMap, BTreeSet};
use tracing::{info, warn};
use assetto_corsa::{Car, Installation};
use assetto_corsa::car::data::Engine;
use assetto_corsa::car::data::engine::EngineData;
use assetto_corsa::car::ENGINE_CRANE_CAR_TAG;
use assetto_corsa::car::ui::CarUiData;
use assetto_corsa::traits::extract_mandatory_section;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut inertia_map: BTreeMap<String, f64> = BTreeMap::new();
    let ac_install = Installation::new();
    let car_folder_root = ac_install.get_installed_car_path();

    let skip_list: BTreeSet<String> = ["asr_1991_fondmetal_fomet1", "asr_1991_larrousse_lc91", "asr_asr2championship", "asr_gp3series", "lotus_exos_125", "pm3dm_volvo_s40_btcc"]
        .iter()
        .map(|&s| s.to_string())
        .collect();

    for entry in std::fs::read_dir(car_folder_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if skip_list.contains(&name) {
            continue;
        }
        let mut car = match Car::load_from_path(&path) {
            Ok(c ) => { c }
            Err(e) => {
                eprintln!("Warning: Skipping {}. {}", path.display(), e.to_string());
                continue;
            }
        };

        match CarUiData::from_car(&mut car) {
            Ok(ui_data) => {
                if ui_data.ui_info.has_tag(ENGINE_CRANE_CAR_TAG) {
                    println!("Skipping generated engine {}", path.display());
                    continue;
                }
            }
            Err(e) => {
                eprintln!("Warning: Skipping {}. Bad ui data: {}", path.display(), e.to_string());
                continue;
            }
        }

        {
            let engine = Engine::from_car(&mut car)?;
            match extract_mandatory_section::<EngineData>(&engine) {
                Ok(d) => {
                    if d.limiter >= 12000 {
                        println!("Skipping engine with too high max rpm {}", path.display());
                        continue;
                    }
                    inertia_map.insert(name.to_string(), d.inertia);
                }
                Err(e) => {
                    eprintln!("Warning: Skipping {}. {}", path.display(), e.to_string());
                    continue;
                }
            };
        }
    }

    let mut min = f64::MAX;
    let mut min_path = String::new();
    let mut max = f64::MIN;
    let mut max_path = String::new();
    for (path, inertia) in inertia_map {
        if inertia < min {
            min = inertia;
            min_path = path.clone();
        }
        if inertia > max {
            max = inertia;
            max_path = path.clone();
        }
        println!("{} -> {}", path, inertia);
    }

    println!("Max inertia. {} -> {}", max_path, max);
    println!("Min inertia. {} -> {}", min_path, min);

    Ok(())
}