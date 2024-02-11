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

use std::collections::{BTreeMap};
use tracing::warn;
use crate::error::Result;
use crate::ini_utils;
use crate::ini_utils::Ini;
use crate::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};


#[allow(dead_code)]
#[derive(Debug)]
pub struct TyreData {
    section_name: String,
    name: String,
    short_name: String,
    width: f64,
    radius: f64,
    rim_radius: f64
}

impl TyreData {
    pub fn from_ini(section_name: String, ini_data: &Ini) -> Result<TyreData> {
        ini_utils::validate_section_exists(ini_data, &section_name)?;
        let name = ini_utils::get_value(ini_data, &section_name, "NAME").unwrap_or(section_name.clone());
        let short_name = match ini_utils::get_value(ini_data, &section_name, "SHORT_NAME") {
            None => section_name.chars().next().unwrap().to_string(),
            Some(n) => n
        };
        let width = ini_utils::get_mandatory_property(ini_data, &section_name, "WIDTH")?;
        let radius = ini_utils::get_mandatory_property(ini_data, &section_name, "RADIUS")?;
        let rim_radius = ini_utils::get_mandatory_property(ini_data, &section_name, "RIM_RADIUS")?;

        Ok(TyreData {
            section_name,
            name,
            short_name,
            width,
            radius,
            rim_radius
        })
    }

    pub fn radius(&self) -> f64 {
        self.radius
    }
}

enum TyreType {
    FRONT,
    REAR
}

impl TyreType {
    pub fn idx_to_section_name(&self, idx: usize) -> String {
        match idx {
            0 => self.ini_section_prefix().to_string(),
            _ => format!("{}_{}", self.ini_section_prefix(), idx)
        }
    }

    pub fn ini_section_prefix(&self) -> &'static str {
        match self {
            TyreType::FRONT => "FRONT",
            TyreType::REAR => "REAR"
        }
    }
}

#[derive(Debug)]
pub struct TyreSet {
    front: TyreData,
    rear: TyreData
}

impl TyreSet {
    pub fn from_ini_data(front_section_name: String,
                         rear_section_name: String,
                         ini_data: &Ini) -> Result<TyreSet>
    {
        Ok(TyreSet {
            front: TyreData::from_ini(front_section_name, ini_data)?,
            rear: TyreData::from_ini(rear_section_name, ini_data)?
        })
    }

    pub fn front_data(&self) -> &TyreData {
        &self.front
    }

    pub fn rear_data(&self) -> &TyreData {
        &self.rear
    }
}

#[derive(Debug)]
pub struct TyreCompounds {
    sets: BTreeMap<usize, TyreSet>,
    default_set_idx: Option<usize>
}

impl TyreCompounds {
    pub fn new() -> TyreCompounds {
        TyreCompounds { sets: BTreeMap::new(), default_set_idx: None }
    }

    pub fn from_ini_data(ini_data: &Ini) -> TyreCompounds {
        let mut compounds = TyreCompounds::new();
        let front_tyre_map = ini_data.get_section_names_with_prefix(TyreType::FRONT.ini_section_prefix());
        let rear_tyre_map = ini_data.get_section_names_with_prefix(TyreType::REAR.ini_section_prefix());
        for idx in front_tyre_map.keys() {
            let front_name = front_tyre_map.get(idx);
            let rear_name = rear_tyre_map.get(idx);
            if front_name.is_none() || rear_name.is_none() {
                continue
            }
            match TyreSet::from_ini_data(front_name.unwrap().to_string(),
                                         rear_name.unwrap().to_string(),
                                         ini_data) {
                Err(e) => {
                    warn!("Couldn't parse tyre set. {}", e.to_string());
                    continue
                },
                Ok(set) => compounds.sets.insert(*idx, set)
            };
        }
        if let Some(idx_str) = ini_data.get_value("COMPOUND_DEFAULT", "INDEX") {
            match idx_str.parse::<usize>() {
                Ok(idx) => {
                    if compounds.sets.len() > idx {
                        compounds.default_set_idx = Some(idx)
                    }
                },
                Err(_) => {}
            }
        }
        compounds
    }

    pub fn get_default_set(&self) -> Option<&TyreSet> {
        if self.sets.is_empty() {
            return None;
        }
        match self.default_set_idx {
            None => {
                self.sets.get(&0)
            }
            Some(idx) => {
                self.sets.get(&idx)
            }
        }
    }
}

impl MandatoryDataSection for TyreCompounds {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<TyreCompounds> where Self: Sized {
        Ok(TyreCompounds::from_ini_data(parent_data.ini_data()))
    }
}

impl CarDataUpdater for TyreCompounds {
    fn update_car_data(&self, _car_data: &mut dyn CarDataFile) -> Result<()> {
        Ok(())
    }
}

