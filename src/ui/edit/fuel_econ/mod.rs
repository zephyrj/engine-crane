/*
 * Copyright (c):
 * 2025 zephyrj
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
use iced::widget::Column;
use tracing::error;
use zephyrj_ac_tools as assetto_corsa;
use assetto_corsa::Car;
use crate::ui::edit::EditMessage;
pub use crate::ui::edit::fuel_econ::eff_input::ThermalEfficiencyInput;
use crate::ui::edit::fuel_econ::flow_input::FuelFlowInput;

mod eff_input;
mod helpers;
mod flow_input;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuelEfficiencyConfigType {
    ByThermalEfficiency,
    ByFuelFlow
}

impl Display for FuelEfficiencyConfigType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            FuelEfficiencyConfigType::ByThermalEfficiency => { write!(f, "Thermal Efficiency") }
            FuelEfficiencyConfigType::ByFuelFlow => { write!(f, "Fuel Flow") }
        }
    }
}

pub enum FuelEfficiencyConfig {
    ThermalEff(ThermalEfficiencyInput),
    FuelFlow(FuelFlowInput)
}

impl FuelEfficiencyConfig {
    pub fn get_config_type(&self) -> FuelEfficiencyConfigType {
        match self {
            FuelEfficiencyConfig::ThermalEff(_e) => FuelEfficiencyConfigType::ByThermalEfficiency,
            FuelEfficiencyConfig::FuelFlow(_e) => FuelEfficiencyConfigType::ByFuelFlow
        }
    }
    
    pub(crate) fn add_editable_list<'a, 'b>(
        &'a self,
        layout: Column<'b, EditMessage>
    ) -> Column<'b, EditMessage>
    where 'b: 'a
    {
        match &self {
            FuelEfficiencyConfig::ThermalEff(e) => e.add_editable_list(layout),
            FuelEfficiencyConfig::FuelFlow(e) => e.add_editable_list(layout)
        }
    }

    pub fn update_for_rpm(&mut self, rpm: i32, new_value: String) {
        match self {
            FuelEfficiencyConfig::ThermalEff(e) => e.update_for_rpm(rpm, new_value),
            FuelEfficiencyConfig::FuelFlow(e) => e.update_for_rpm(rpm, new_value),
        }
    }
    
    pub fn write_car_updates(&self, ac_car_path: &PathBuf) -> Result<(), String> {
        match &self {
            FuelEfficiencyConfig::ThermalEff(e) => e.write_car_updates(ac_car_path),
            FuelEfficiencyConfig::FuelFlow(e) => e.write_car_updates(ac_car_path)
        }
    }
}

pub fn consumption_configuration_builder(config_type: FuelEfficiencyConfigType,
                                         ac_car_path: &PathBuf) -> Result<FuelEfficiencyConfig, String> {
    let mut car = match Car::load_from_path(ac_car_path) {
        Ok(c) => { c }
        Err(err) => {
            let err_str = format!("Failed to load {}. {}", ac_car_path.display(), err.to_string());
            error!("{}", &err_str);
            return Err(err_str);
        }
    };
    match config_type {
        FuelEfficiencyConfigType::ByFuelFlow => {
            Ok(FuelEfficiencyConfig::FuelFlow(FuelFlowInput::from_car(&mut car)?))
        },
        FuelEfficiencyConfigType::ByThermalEfficiency => {
            Ok(FuelEfficiencyConfig::ThermalEff(ThermalEfficiencyInput::from_car(&mut car)?))
        }
    }
    
}
