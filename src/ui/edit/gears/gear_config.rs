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

use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use iced::widget::{Column};

use tracing::{error, warn};

use crate::assetto_corsa::Car;
use crate::assetto_corsa::car::data;
use crate::assetto_corsa::car::data::Drivetrain;
use crate::assetto_corsa::car::data::setup::Setup;
use crate::assetto_corsa::car::data::setup::gears::{GearConfig, GearData};
use crate::assetto_corsa::traits::{extract_mandatory_section, MandatoryDataSection};

use crate::ui::edit::EditMessage;
use crate::ui::edit::gears::customizable::CustomizableGears;
use crate::ui::edit::gears::final_drive::{FinalDrive, FinalDriveUpdate};
use crate::ui::edit::gears::fixed::FixedGears;
use crate::ui::edit::gears::{CustomizedGearUpdate, FixedGearUpdate, GearsetUpdate};
use crate::ui::edit::gears::gear_sets::GearSets;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GearConfigChoice {
    Fixed,
    GearSets,
    PerGearConfig
}

impl Display for GearConfigChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            GearConfigChoice::Fixed => { write!(f, "Fixed Gearing") }
            GearConfigChoice::GearSets => { write!(f, "Gear Sets") }
            GearConfigChoice::PerGearConfig => { write!(f, "Fully Customizable") }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GearUpdateType {
    Fixed(FixedGearUpdate),
    Gearset(GearsetUpdate),
    CustomizedGear(CustomizedGearUpdate)
}

pub trait GearConfiguration {
    fn get_config_type(&self) -> GearConfigChoice;
    fn handle_gear_update(&mut self, update_type: GearUpdateType);
    fn handle_final_drive_update(&mut self, update_type: FinalDriveUpdate);
    fn add_editable_gear_list<'a, 'b>(&'a self, layout: Column<'b, EditMessage>) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        layout
    }
}

pub fn gear_configuration_builder(ac_car_path: &PathBuf) -> Result<Box<dyn GearConfiguration>, String> {
    let mut car = match Car::load_from_path(ac_car_path) {
        Ok(c) => { c }
        Err(err) => {
            let err_str = format!("Failed to load {}. {}", ac_car_path.display(), err.to_string());
            error!("{}", &err_str);
            return Err(err_str);
        }
    };
    let drivetrain_data: Vec<f64>;
    let current_final_drive: f64;
    match Drivetrain::from_car(&mut car) {
        Ok(drivetrain) => {
            match extract_mandatory_section::<data::drivetrain::Gearbox>(&drivetrain) {
                Ok(gearbox) => {
                    drivetrain_data = gearbox.gear_ratios().iter().map(|ratio| *ratio).collect();
                    current_final_drive = gearbox.final_gear_ratio;
                }
                Err(err) => {
                    return Err(format!("Failed to load Gearbox data from {}. {}", ac_car_path.display(), err.to_string()));
                }
            }
        },
        Err(err) => {
            return Err(format!("Failed to load drivetrain from {}. {}", ac_car_path.display(), err.to_string()));
        }
    };
    let gear_setup_data: Option<GearData>;
    {
        let setup = Setup::from_car(&mut car);
        gear_setup_data = match setup {
            Ok(opt) => {
                match opt {
                    Some(setup_data) => {
                        match GearData::load_from_parent(&setup_data) {
                            Ok(gear_data) => {
                                Some(gear_data)
                            }
                            Err(err) => {
                                return Err(format!("Failed to load gear data from {}. {}", ac_car_path.display(), err.to_string()));
                            }
                        }
                    }
                    None => None
                }
            }
            Err(err) => {
                warn!("Failed to load {}.{}", ac_car_path.display(), err.to_string());
                None
            }
        };
    }
    let (drivetrain_gear_setup, final_drive_setup) = match gear_setup_data {
        None => (None, None),
        Some(data) => (data.gear_config, data.final_drive)
    };

    let gear_config_type = match &drivetrain_gear_setup {
        None => GearConfigChoice::Fixed,
        Some(config) => {
            match config {
                GearConfig::GearSets(_) => GearConfigChoice::GearSets,
                GearConfig::PerGear(_) => GearConfigChoice::PerGearConfig
            }
        }
    };
    let final_drive_data = FinalDrive::from_gear_data(current_final_drive, final_drive_setup);
    return match gear_config_type {
        GearConfigChoice::Fixed => {
            Ok(Box::new(FixedGears::from_gear_data(drivetrain_data, final_drive_data)))
        }
        GearConfigChoice::GearSets => {
            let current_setup_data =  match drivetrain_gear_setup {
                    None => Vec::new(),
                    Some(gear_config) => { match gear_config {
                        GearConfig::GearSets(gear_set) => gear_set,
                        GearConfig::PerGear(_) => Vec::new()
                    }}
            };
            Ok(Box::new(GearSets::from_gear_data(drivetrain_data, current_setup_data, final_drive_data)))
        }
        GearConfigChoice::PerGearConfig => {
            let current_setup_data =  match drivetrain_gear_setup {
                None => Vec::new(),
                Some(gear_config) => { match gear_config {
                    GearConfig::GearSets(_) => Vec::new(),
                    GearConfig::PerGear(gear_vec) => gear_vec
                }}
            };
            Ok(Box::new(CustomizableGears::from_gear_data(drivetrain_data, current_setup_data, final_drive_data)))
        }
    }
}
