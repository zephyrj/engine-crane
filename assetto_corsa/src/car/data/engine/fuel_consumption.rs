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

use std::collections::HashMap;
use crate::ini_utils::Ini;
use crate::error::{Result, Error, ErrorKind};
use crate::ini_utils;
use crate::car::lut_utils::{InlineLut, LutType};
use crate::car::structs::LutProperty;
use crate::traits::{CarDataFile, CarDataUpdater, DataInterface};


#[derive(Debug)]
pub struct ExtendedFuelConsumptionBaseData {
    idle_throttle: Option<f64>,
    idle_cutoff: Option<i32>,
    mechanical_efficiency: Option<f64>
}

impl ExtendedFuelConsumptionBaseData {
    const SECTION_NAME: &'static str = "ENGINE_DATA";

    pub fn new(idle_throttle: Option<f64>,
               idle_cutoff: Option<i32>,
               mechanical_efficiency: Option<f64>) -> ExtendedFuelConsumptionBaseData {
        ExtendedFuelConsumptionBaseData { idle_throttle, idle_cutoff, mechanical_efficiency }
    }

    fn load_from_ini(ini_data: &Ini) -> Result<ExtendedFuelConsumptionBaseData> {
        Ok(ExtendedFuelConsumptionBaseData {
            idle_throttle: ini_utils::get_value(ini_data, Self::SECTION_NAME, "IDLE_THROTTLE"),
            idle_cutoff: ini_utils::get_value(ini_data, Self::SECTION_NAME, "IDLE_CUTOFF"),
            mechanical_efficiency: ini_utils::get_value(ini_data, Self::SECTION_NAME, "MECHANICAL_EFFICIENCY")
        })
    }
}

impl CarDataUpdater for ExtendedFuelConsumptionBaseData {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        if let Some(idle_throttle) = self.idle_throttle {
            ini_utils::set_float(ini_data, Self::SECTION_NAME, "IDLE_THROTTLE", idle_throttle, 3);
        } else if ini_data.section_contains_property(Self::SECTION_NAME, "IDLE_THROTTLE") {
            ini_data.remove_value(Self::SECTION_NAME, "IDLE_THROTTLE");
        }

        if let Some(idle_cutoff) = self.idle_cutoff {
            ini_utils::set_value(ini_data, Self::SECTION_NAME, "IDLE_CUTOFF", idle_cutoff);
        } else if ini_data.section_contains_property(Self::SECTION_NAME, "IDLE_CUTOFF") {
            ini_data.remove_value(Self::SECTION_NAME, "IDLE_CUTOFF");
        }

        if let Some(mechanical_efficiency) = self.mechanical_efficiency {
            ini_utils::set_float(ini_data, Self::SECTION_NAME, "MECHANICAL_EFFICIENCY", mechanical_efficiency, 3);
        } else if ini_data.section_contains_property(Self::SECTION_NAME, "MECHANICAL_EFFICIENCY") {
            ini_data.remove_value(Self::SECTION_NAME, "MECHANICAL_EFFICIENCY");
        }
        Ok(())
    }
}

#[allow(dead_code)]
struct FuelConsumptionEfficiency {
    base_data: ExtendedFuelConsumptionBaseData,
    thermal_efficiency: f64,
    thermal_efficiency_dict: Option<HashMap<i32, f64>>,
    fuel_lhv: i32,
    turbo_efficiency: Option<f64>
}

#[derive(Debug)]
pub struct FuelConsumptionFlowRate {
    base_data: ExtendedFuelConsumptionBaseData,
    max_fuel_flow_lut: Option<LutProperty<i32, i32>>,
    max_fuel_flow: i32
}

impl FuelConsumptionFlowRate {
    pub const SECTION_NAME: &'static str = "FUEL_CONSUMPTION";

    pub fn new(idle_throttle: f64,
               idle_cutoff: i32,
               mechanical_efficiency: f64,
               max_fuel_flow_lut: Option<Vec<(i32, i32)>>,
               max_fuel_flow: i32) -> FuelConsumptionFlowRate
    {
        FuelConsumptionFlowRate{
            base_data: ExtendedFuelConsumptionBaseData {
                idle_throttle: Some(idle_throttle),
                idle_cutoff: Some(idle_cutoff),
                mechanical_efficiency: Some(mechanical_efficiency)
            },
            max_fuel_flow_lut: match max_fuel_flow_lut {
                None => { None }
                Some(lut_vec) => {
                    Some(LutProperty::new(
                        LutType::Inline(InlineLut::from_vec(lut_vec)),
                        String::from(Self::SECTION_NAME),
                        String::from("MAX_FUEL_FLOW_LUT")))
                }},
            max_fuel_flow
        }
    }

    pub fn load_from_data(ini_data: &Ini,
                          data_interface: &dyn DataInterface) -> Result<Option<FuelConsumptionFlowRate>> {
        if !ini_data.contains_section(Self::SECTION_NAME) {
            return Ok(None)
        }

        let max_fuel_flow_lut = LutProperty::optional_from_ini(
            String::from(Self::SECTION_NAME),
            String::from("MAX_FUEL_FLOW_LUT"),
            ini_data,
            data_interface
        ).map_err(|err_str| {
            Error::new(ErrorKind::InvalidCar,
                       format!("Error loading fuel flow consumption lut. {}", err_str))
        })?;
        let mut max_fuel_flow = 0;
        if let Some(val) = ini_utils::get_value(ini_data, Self::SECTION_NAME, "MAX_FUEL_FLOW") {
            max_fuel_flow = val;
        }
        Ok(Some(FuelConsumptionFlowRate{
            base_data: ExtendedFuelConsumptionBaseData::load_from_ini(ini_data)?,
            max_fuel_flow_lut,
            max_fuel_flow
        }))
    }
}

impl CarDataUpdater for FuelConsumptionFlowRate {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        self.base_data.update_car_data(car_data)?;
        let ini_data = car_data.mut_ini_data();
        ini_data.remove_section(Self::SECTION_NAME);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "MAX_FUEL_FLOW", self.max_fuel_flow);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "LOG_FUEL_FLOW", 0);
        if let Some(flow_lut) = &self.max_fuel_flow_lut {
            flow_lut.update_car_data(car_data)?;
        }
        Ok(())
    }
}
