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
use std::collections::BTreeMap;
use std::num::ParseIntError;
use std::path::PathBuf;
use iced::{Alignment, Length};
use iced::widget::{Column, container, Row, Text, TextInput};
use iced_native::Widget;
use iced_native::widget::vertical_rule;
use tracing::{error, info, warn};
use assetto_corsa::Car;
use assetto_corsa::car::data;
use assetto_corsa::car::data::car_ini_data::CarVersion;
use assetto_corsa::car::data::{CarIniData, Drivetrain, Engine};
use assetto_corsa::car::data::drivetrain::traction::DriveType;
use assetto_corsa::car::data::engine::{EngineData, FuelConsumptionFlowRate, PowerCurve, Turbo, TurboControllerFile};
use assetto_corsa::car::data::engine::turbo_ctrl::TurboController;
use assetto_corsa::car::lut_utils::LutInterpolator;
use assetto_corsa::car::model::GearingCalculator;
use assetto_corsa::traits::{CarDataFile, extract_mandatory_section, MandatoryDataSection, update_car_data, OptionalDataSection};
use crate::fabricator::FabricationError::FailedToLoad;
use crate::ui::edit::EditMessage;
use crate::ui::edit::EditMessage::FuelConsumptionUpdate;
use crate::ui::edit::gears::GearConfig;

pub use crate::ui::edit::fuel_econ::eff_input::FuelConsumptionConfig;

mod eff_input;
mod helpers;

pub fn consumption_configuration_builder(ac_car_path: &PathBuf) -> Result<FuelConsumptionConfig, String> {
    let mut car = match Car::load_from_path(ac_car_path) {
        Ok(c) => { c }
        Err(err) => {
            let err_str = format!("Failed to load {}. {}", ac_car_path.display(), err.to_string());
            error!("{}", &err_str);
            return Err(err_str);
        }
    };
    Ok(FuelConsumptionConfig::from_car(&mut car)?)
}
