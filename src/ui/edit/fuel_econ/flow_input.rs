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
use std::path::PathBuf;
use iced::{Alignment, Length};
use iced::widget::{Column, Row, Text, TextInput};
use iced_native::widget::vertical_rule;
use tracing::{error, info, warn};
use assetto_corsa::Car;
use assetto_corsa::car::data::{CarIniData, Engine};
use assetto_corsa::car::data::car_ini_data::CarVersion;
use assetto_corsa::car::data::engine::{EngineData, FuelConsumptionFlowRate};
use assetto_corsa::car::lut_utils::LutInterpolator;
use assetto_corsa::traits::{update_car_data, CarDataFile, MandatoryDataSection};
use utils::units::g_min_to_kg_hour;
use crate::ui::edit::EditMessage;
use crate::ui::edit::EditMessage::FuelConsumptionUpdate;
use crate::ui::edit::fuel_econ::helpers::{get_min_max_rpms, load_drive_type};

pub struct FuelFlowInput {
    original_data: BTreeMap<i32, i32>,
    mechanical_efficiency: f64,
    updated_data: BTreeMap<i32, Option<String>>,
}

const RPM_STEP: usize = 500;

impl FuelFlowInput {
    pub fn from_car(car: &mut Car) -> Result<FuelFlowInput, String> {
        let drive_type= load_drive_type(car)?;
        let mechanical_efficiency = drive_type.mechanical_efficiency();
        info!("Existing car is {} with assumed mechanical efficiency of {}", drive_type, mechanical_efficiency);

        let original_data;
        let mut updated_data = BTreeMap::new();
        {
            let engine = Engine::from_car(car).map_err(|err| {
                err.to_string()
            })?;
            original_data =
                match FuelConsumptionFlowRate::load_from_data(&engine.ini_data(), engine.data_interface()) {
                    Ok(rate_opt) => {
                        match rate_opt {
                            Some(rate_data) => {
                                rate_data.get_max_fuel_flow_lut_data()
                            },
                            None => BTreeMap::new()
                        }
                    }
                    Err(e) => {
                        warn!("Error trying to read fuel consumption data. {}", e.to_string());
                        BTreeMap::new()
                    }
                };
            
            if original_data.is_empty() {
                let (start_rpm, end_rpm) = get_min_max_rpms(&engine)?;
                for rpm in (start_rpm..=end_rpm).rev().step_by(RPM_STEP) {
                    if rpm < 0 {
                        continue;
                    }
                    let _ = updated_data.insert(rpm, None);
                }
                if *updated_data.first_key_value().ok_or(String::from("Updated data unexpectedly empty"))?.0 != start_rpm {
                    let _ = updated_data.insert(start_rpm, None);
                }
            } else {
                for rpm in original_data.keys() {
                    let _ = updated_data.insert(*rpm, None);
                }
            }
        }
        Ok(FuelFlowInput {
            original_data,
            mechanical_efficiency,
            updated_data
        })
    }

    pub(crate) fn add_editable_list<'a, 'b>(
        &'a self,
        layout: Column<'b, EditMessage>
    ) -> Column<'b, EditMessage>
    where 'b: 'a
    {
        let mut rpm_column = Column::new().width(Length::Shrink).spacing(7).align_items(Alignment::Center).push(Text::new("RPM").size(16));
        let mut flow_input_col = Column::new().width(Length::Shrink).spacing(7).align_items(Alignment::Center).push(Text::new("Fuel Flow g/min").size(16));

        let row_height = Length::Units(28);
        for (rpm, val_opt) in self.updated_data.iter() {
            let val = match &val_opt {
                None => String::new(),
                Some(v) => v.clone()
            };
            let rpm_copy = *rpm;
            let flow_input = Row::new().height(row_height).align_items(Alignment::Center).push(
                TextInput::new(
                    "", &*val, move |new_value| FuelConsumptionUpdate(rpm_copy, new_value)
                ).width(Length::Units(50))
            );
            flow_input_col = flow_input_col.push(flow_input);
            rpm_column = rpm_column.push(
                Row::new()
                    .height(row_height)
                    .align_items(Alignment::Center)
                    .push(Text::new(format!("{}:", rpm)))
            );
        }
        let mut holder = Column::new().width(Length::Shrink).align_items(Alignment::Fill).spacing(10);
        holder = holder.push(
            Row::new().width(Length::Shrink).align_items(Alignment::Fill).spacing(10)
                .push(rpm_column)
                .push(flow_input_col)
                .push(vertical_rule(5))
        );
        layout.push(holder)
    }

    pub fn update_for_rpm(&mut self, rpm: i32, new_value: String) {
        if self.updated_data.contains_key(&rpm) {
            if new_value.is_empty() {
                let _ = self.updated_data.insert(rpm, None);
            } else {
                let _ = self.updated_data.insert(rpm, Some(new_value));
            }
        }
    }

    pub fn write_car_updates(&self, ac_car_path: &PathBuf) -> Result<(), String> {
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

            let mut max_fuel_flow: i32 = i32::MIN;
            let flow_vec: Vec<(i32, i32)> = self.updated_data.iter()
                .filter_map(|(key, value)| {
                    value.clone().and_then(|s| s.parse::<i32>().ok().map(|parsed_value| {
                        if parsed_value > max_fuel_flow {
                            max_fuel_flow = parsed_value;
                        }
                        (*key, parsed_value)
                    } 
                    ))
                })
                .collect();
            let flow_interpolator = LutInterpolator::from_vec(flow_vec);
            
            let mut max_flow_lut: Vec<(i32, i32)> = Vec::new();
            for (rpm, _) in self.updated_data.iter() {
                let flow_g_min = match flow_interpolator.get_value(*rpm) {
                    Some(eff) => eff.round() as i32,
                    None => {
                        warn!("Couldn't interpolate efficiency input @{}rpm. Skipping value in max_flow lut", rpm);
                        continue
                    }
                };
                max_flow_lut.push((*rpm, g_min_to_kg_hour(flow_g_min as f64).round() as i32));
            }
            if max_flow_lut.is_empty() {
                return Err("Not enough efficiency data to create fuel consumption data".to_string())
            }

            let fuel_flow = FuelConsumptionFlowRate::new(
                0.03,
                idle + 100,
                self.mechanical_efficiency,
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