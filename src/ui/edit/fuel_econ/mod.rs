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
use iced::widget::{Column, Row, Text, TextInput};
use tracing::{error, info, warn};
use assetto_corsa::Car;
use assetto_corsa::car::data;
use assetto_corsa::car::data::car_ini_data::CarVersion;
use assetto_corsa::car::data::{CarIniData, Engine};
use assetto_corsa::car::data::engine::{EngineData, FuelConsumptionFlowRate, PowerCurve};
use assetto_corsa::car::lut_utils::LutInterpolator;
use assetto_corsa::car::model::GearingCalculator;
use assetto_corsa::traits::{CarDataFile, MandatoryDataSection, update_car_data};
use crate::fabricator::FabricationError::FailedToLoad;
use crate::ui::edit::EditMessage;
use crate::ui::edit::EditMessage::FuelConsumptionUpdate;
use crate::ui::edit::gears::GearConfig;


pub fn consumption_configuration_builder(ac_car_path: &PathBuf) -> Result<FuelConsumptionConfig, String> {
    let mut config = FuelConsumptionConfig {
        original_data: BTreeMap::new(),
        updated_data: BTreeMap::new(),
    };

    let mut car = match Car::load_from_path(ac_car_path) {
        Ok(c) => { c }
        Err(err) => {
            let err_str = format!("Failed to load {}. {}", ac_car_path.display(), err.to_string());
            error!("{}", &err_str);
            return Err(err_str);
        }
    };

    {
        let mut engine = Engine::from_car(&mut car).map_err(|err| {
            err.to_string()
        })?;

        match FuelConsumptionFlowRate::load_from_data(&engine.ini_data(), engine.data_interface()) {
            Ok(rate_opt) => {
                if let Some(rate_data) = rate_opt {
                    config.original_data = rate_data.get_max_fuel_flow_lut_data()
                }
            }
            Err(_) => {}
        }

        if config.original_data.is_empty() {
            let idle;
            let limiter;
            match EngineData::load_from_parent(&engine) {
                Ok(ed) => {
                    idle = ed.minimum;
                    limiter = ed.limiter;
                }
                Err(e) => {
                    return Err(format!("Failed to load engine data. {}", e.to_string()));
                }
            };
            let start_rpm = idle;
            let end_rpm = limiter;
            for rpm in (start_rpm..=end_rpm).rev().step_by(500) {
                if rpm < 0 {
                    continue;
                }
                let _ = config.updated_data.insert(rpm, None);
            }
            if *config.updated_data.first_key_value().ok_or(String::from("Updated data unexpectedly empty"))?.0 != start_rpm {
                let _ = config.updated_data.insert(start_rpm, None);
            }
        }
        else {
            for rpm in config.original_data.keys() {
                let _ = config.updated_data.insert(*rpm, None);
            }
        }

        for rpm in config.updated_data.keys() {
            // get interpolated torque
        }
    }
    Ok(config)
}

pub struct FuelConsumptionConfig {
    original_data: BTreeMap<i32, i32>,
    updated_data: BTreeMap<i32, Option<String>>,
}

impl FuelConsumptionConfig {
    pub(crate) fn add_editable_list<'a, 'b>(
        &'a self,
        layout: Column<'b, EditMessage>
    ) -> Column<'b, EditMessage>
    where 'b: 'a
    {
        let mut holder = Column::new().width(Length::Shrink).spacing(10);
        holder = holder.push(
            Row::new().width(Length::Shrink).spacing(30)
                .push(Text::new("RPM"))
                .push(Text::new("Efficiency %"))
        );
        for (rpm, val_opt) in self.updated_data.iter() {
            let mut row = Row::new().spacing(35).width(Length::Shrink).align_items(Alignment::Center);
            row = row.push(Text::new(rpm.to_string()));
            let placeholder = match self.original_data.get(&rpm){
                None => String::new(),
                Some(v) => v.to_string()
            };
            let val = match &val_opt {
                None => String::new(),
                Some(v) => v.clone()
            };
            let rpm_copy = *rpm;
            row = row.push(
                TextInput::new(
                    &*placeholder, &*val, move |new_value| FuelConsumptionUpdate(rpm_copy, new_value)
                ).width(Length::Units(32))
            );
            holder = holder.push(row);
        }
        layout.push(holder)
    }

    pub fn update_efficiency_string(&mut self, rpm: i32, new_value: String) {
        if self.updated_data.contains_key(&rpm) {
            if new_value.is_empty() {
                self.updated_data.insert(rpm, None);
            } else if is_valid_percentage(&new_value) {
                self.updated_data.insert(rpm, Some(new_value));
            }
        }
    }

    pub fn update_car(&self, ac_car_path: &PathBuf) -> Result<(), String> {
        let mut car = match Car::load_from_path(ac_car_path) {
            Ok(c) => { c }
            Err(err) => {
                let err_str = format!("Failed to load {}. {}", ac_car_path.display(), err.to_string());
                error!("{}", &err_str);
                return Err(err_str);
            }
        };

        {
            let mut engine = Engine::from_car(&mut car).map_err(|err| { err.to_string() })?;
            let idle;
            let limiter;
            match EngineData::load_from_parent(&engine) {
                Ok(ed) => {
                    idle = ed.minimum;
                    limiter = ed.limiter;
                }
                Err(e) => {
                    return Err(format!("Failed to load engine data. {}", e.to_string()));
                }
            };

            let power_interpolator = match PowerCurve::load_from_parent(&engine) {
                Ok(curve) => {
                    LutInterpolator::from_lut(curve.get_lut())
                },
                Err(e) => {
                    return Err(format!("Failed to load engine curve data. {}", e.to_string()));
                }
            };
            let eff_vec : Vec<(i32, i32)> = self.updated_data.iter()
                .filter_map(|(key, value)| {
                    value.clone().and_then(|s| s.parse::<i32>().ok().map(|parsed_value| (*key, parsed_value)))
                })
                .collect();
            let eff_interpolator = LutInterpolator::from_vec(eff_vec);

            // kW⋅h/g
            const GASOLINE_LHV: f64 = 0.01204;

            let mechanical_efficiency = 0.8;
            // The lut values should be: rpm, kg/hr
            // The max-flow should be weighted to the upper end of the rev-range as racing is usually done in that range.
            // This is probably enough of a fallback as this will only be used if a lut isn't found and that will be
            // calculated below
            let max_flow_entry_rpm: i32 = idle + (0.7 * (limiter - idle) as f64).round() as i32;
            let p_eff = match eff_interpolator.get_value(max_flow_entry_rpm) {
                Some(v) => v.round() as i32,
                None => return Err("Failed to get max_flow eff val".to_string())
            };
            let p_power = match power_interpolator.get_value(max_flow_entry_rpm) {
                Some(v) => v,
                None => return Err("Failed to get max_flow eff val".to_string())
            };
            let max_fuel_flow = (get_fuel_use_per_sec_at_rpm(p_eff, GASOLINE_LHV, p_power) * 3.6).round() as i32;

            let mut max_flow_lut: Vec<(i32, i32)> = Vec::new();
            for (rpm, eff_opt) in self.updated_data.iter() {
                let power = match power_interpolator.get_value(*rpm) {
                    Some(v) => v,
                    None => return Err("Failed to get max_flow eff val".to_string())
                };
                match eff_opt {
                    Some(eff_str) => {
                        match eff_str.parse::<i32>() {
                            Ok(eff) => {
                                max_flow_lut.push((*rpm, (get_fuel_use_per_sec_at_rpm(eff, GASOLINE_LHV, power) * 3.6).round() as i32));
                            }
                            Err(e) =>  warn!("Can't parse efficiency input for {}. {}. Skipping", rpm, e.to_string())
                        }
                    }
                    None => {
                        warn!("Missing efficiency input for {}. Interpolating from known points", rpm);
                        if let Some(i_eff) = eff_interpolator.get_value(*rpm) {
                            max_flow_lut.push(
                                (*rpm, (get_fuel_use_per_sec_at_rpm(i_eff.round() as i32, GASOLINE_LHV, power) * 3.6).round() as i32)
                            );
                        } else {
                            warn!("Couldn't interpolate efficiency input for {}. Skipping", rpm);
                        }
                    },
                }
            }
            if max_flow_lut.is_empty() {
                return Err("Not enough efficiency data to create fuel consumption data".to_string())
            }

            let fuel_flow = FuelConsumptionFlowRate::new(
                0.03,
                idle + 100,
                mechanical_efficiency,
                Some(max_flow_lut),
                max_fuel_flow
            );
            update_car_data(&mut engine, &fuel_flow).map_err(|err| {
                err.to_string()
            })?;
            info!("Writing engine ini files");
            engine.write().map_err(|err| {
                format!("Failed to write engine.ini. {}", err.to_string())
            })?;
        }

        {
            let mut ini_data = CarIniData::from_car(&mut car).map_err(|e|{
                e.to_string()
            })?;
            ini_data.set_version(CarVersion::CspExtendedPhysics);
            ini_data.clear_fuel_consumption();
            info!("Writing car ini files");
            ini_data.write().map_err(|e| {
                format!("Failed to write car.ini. {}", e.to_string())
            })?;
        }
        Ok(())
    }
}

fn is_valid_percentage(val: &str) -> bool {
    if val.is_empty() {
        return true;
    }
    match val.parse::<i32>() {
        Ok(v) => {
            if v >= 0 && v <= 100 {
                return true;
            }
            false
        }
        Err(_) => false
    }
}

fn get_fuel_use_per_sec_at_rpm(eff_percentage: i32, fuel_lhv: f64, power_kw: f64) -> f64 {
    // fuel lhv in kWh/g
    // BSFC [g/(kW⋅h)] = 1 / (eff * fuel_lhv)
    // https://en.wikipedia.org/wiki/Brake-specific_fuel_consumption
    // BSFC (g/J) = fuel_consumption (g/s) / power (watts)
    // fuel_consumption (g/s) = BSFC * power (watts)
    // BSFC value stored in econ curve as g/kWh
    // BSFC [g/(kW⋅h)] = BSFC [g/J] × (3.6 × 10^6)
    let power_watts = power_kw * 1000.0;
    let eff: f64 = eff_percentage as f64 / 100.0;
    let bsfc = 1.0 / (eff * fuel_lhv);
    (bsfc / 3600000_f64) * power_watts
}


// pub fn fuel_flow_consumption(mechanical_efficiency: f64) -> data::engine::FuelConsumptionFlowRate {
//     // The lut values should be: rpm, kg/hr
//     // The max-flow should be weighted to the upper end of the rev-range as racing is usually done in that range.
//     // This is probably enough of a fallback as this will only be used if a lut isn't found and that will be
//     // calculated below
//     let max_flow_entry_index = (self.engine_sqlite_data.rpm_curve.len() as f64 * 0.70).round() as usize;
//     let max_fuel_flow = (self.get_fuel_use_per_sec_at_rpm(max_flow_entry_index) * 3.6).round() as i32;
//
//     let mut max_flow_lut: Vec<(i32, i32)> = Vec::new();
//     for (rpm_idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
//         max_flow_lut.push((*rpm as i32, (self.get_fuel_use_per_sec_at_rpm(rpm_idx) * 3.6).round() as i32))
//     }
//     data::engine::FuelConsumptionFlowRate::new(
//         0.03,
//         (self.idle_speed().unwrap() + 100_f64).round() as i32,
//         mechanical_efficiency,
//         Some(max_flow_lut),
//         max_fuel_flow
//     )
// }
