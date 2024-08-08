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
use utils::units::calculate_power_kw;
use crate::fabricator::FabricationError::FailedToLoad;
use crate::ui::edit::EditMessage;
use crate::ui::edit::EditMessage::FuelConsumptionUpdate;
use crate::ui::edit::gears::GearConfig;


pub fn consumption_configuration_builder(ac_car_path: &PathBuf) -> Result<FuelConsumptionConfig, String> {
    let mut car = match Car::load_from_path(ac_car_path) {
        Ok(c) => { c }
        Err(err) => {
            let err_str = format!("Failed to load {}. {}", ac_car_path.display(), err.to_string());
            error!("{}", &err_str);
            return Err(err_str);
        }
    };

    let drive_type= load_drive_type(&mut car)?;
    let mechanical_efficiency = drive_type.mechanical_efficiency();
    info!("Existing car is {} with assumed mechanical efficiency of {}", drive_type, mechanical_efficiency);

    let is_turbo;
    {
        let engine = Engine::from_car(&mut car).map_err(|err| {
            err.to_string()
        })?;

        {
            is_turbo = match Turbo::load_from_parent(&engine) {
                Ok(opt) => opt.is_some(),
                Err(e) => {
                    warn!("Couldn't determine if turbo engine. {}", e.to_string());
                    info!("Assuming NA");
                    false
                }
            };
        }
    }

    let mut boost_interpolator_opt: Option<LutInterpolator<f64, f64>> = None;
    if is_turbo {
        match TurboControllerFile::from_car(&mut car, 0) {
            Ok(ctrl_opt) => {
                match ctrl_opt {
                    None => {
                        warn!("No turbo controller file. No boost corrections applied.")
                    }
                    Some(ctrl_file) => match TurboController::load_from_parent(0, &ctrl_file) {
                        Ok(tc) => boost_interpolator_opt = Some(LutInterpolator::from_vec(tc.get_lut().to_vec())),
                        Err(e) => warn!("Failed to load turbo controller. No boost corrections applied. {}", e.to_string())
                    }
                }
            }
            Err(e) => warn!("Error loading turbo controller file. No boost corrections applied. {}", e.to_string())
        }
    }

    let original_data;
    let mut updated_data = BTreeMap::new();
    let power_curve_interpolator;
    {
        let mut engine = Engine::from_car(&mut car).map_err(|err| {
            err.to_string()
        })?;

        original_data  =
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

        power_curve_interpolator =
            create_engine_power_interpolator(&engine, mechanical_efficiency, boost_interpolator_opt)?;

        if original_data.is_empty() {
            let (start_rpm, end_rpm) = get_min_max_rpms(&engine)?;
            for rpm in (start_rpm..=end_rpm).rev().step_by(500) {
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
    let config = FuelConsumptionConfig {
        original_data,
        mechanical_efficiency,
        updated_data,
        power_curve_interpolator,
        projected_fuel_flow: BTreeMap::new(),
    };
    Ok(config)
}

// kW⋅h/g
const GASOLINE_LHV: f64 = 0.01204;

pub struct FuelConsumptionConfig {
    original_data: BTreeMap<i32, i32>,
    mechanical_efficiency: f64,
    updated_data: BTreeMap<i32, Option<String>>,
    power_curve_interpolator: LutInterpolator<i32, f64>,
    projected_fuel_flow: BTreeMap<i32, i32>
}

impl FuelConsumptionConfig {
    pub(crate) fn add_editable_list<'a, 'b>(
        &'a self,
        layout: Column<'b, EditMessage>
    ) -> Column<'b, EditMessage>
    where 'b: 'a
    {
        let mut rpm_column = Column::new().width(Length::Shrink).spacing(7).align_items(Alignment::Center).push(Text::new("RPM").size(16));
        let mut eff_input_col = Column::new().width(Length::Shrink).spacing(7).align_items(Alignment::Center).push(Text::new("Efficiency %").size(16));
        let mut power_col = Column::new().width(Length::Shrink).spacing(7).align_items(Alignment::Center).push(Text::new("Power").size(16));
        let mut projected_flow_col = Column::new().width(Length::Shrink).spacing(7).align_items(Alignment::Center).push(Text::new("Proj. Flow kg/hr").size(16));

        let row_height = Length::Units(28);
        for (rpm, val_opt) in self.updated_data.iter() {
            let val = match &val_opt {
                None => String::new(),
                Some(v) => v.clone()
            };
            let rpm_copy = *rpm;
            let eff_input = Row::new().height(row_height).align_items(Alignment::Center).push(
                TextInput::new(
                    "", &*val, move |new_value| FuelConsumptionUpdate(rpm_copy, new_value)
                ).width(Length::Units(50))
            );
            eff_input_col = eff_input_col.push(eff_input);
            rpm_column = rpm_column.push(
                Row::new()
                    .height(row_height)
                    .align_items(Alignment::Center)
                    .push(Text::new(format!("{}:", rpm)))
            );

            power_col = power_col.push(
                Row::new().height(row_height).align_items(Alignment::Center).push(Text::new(
                    match self.power_curve_interpolator.get_value(*rpm) {
                        None => "-- kW".to_string(),
                        Some(power) => format!("{} kW", power.round() as i32)
                    }
                ).size(18))
            );

            projected_flow_col = projected_flow_col.push(
                Row::new().height(row_height).align_items(Alignment::Center).push(Text::new(
                    match self.projected_fuel_flow.get(rpm) {
                        None => "".to_string(),
                        Some(val) => format!("{} kg/h", val.to_string())
                    }
                ).size(18))
            );
        }
        let mut holder = Column::new().width(Length::Shrink).align_items(Alignment::Fill).spacing(10);
        holder = holder.push(
            Row::new().width(Length::Shrink).align_items(Alignment::Fill).spacing(10)
                .push(rpm_column)
                .push(eff_input_col)
                .push(vertical_rule(5))
                .push(power_col)
                .push(projected_flow_col)
        );
        layout.push(holder)
    }

    pub fn update_efficiency_string(&mut self, rpm: i32, new_value: String) {
        if self.updated_data.contains_key(&rpm) {
            if new_value.is_empty() {
                _ = self.updated_data.insert(rpm, None);
                _ = self.projected_fuel_flow.remove(&rpm);
            } else if is_valid_percentage(&new_value) {
                _ = self.updated_data.insert(rpm, Some(new_value));
                self.update_projected_fuel_flow(rpm);
            }
        }
    }

    fn update_projected_fuel_flow(&mut self, rpm: i32) {
        if let Some(eff_opt) = self.updated_data.get(&rpm) {
            if let Some(eff_str) = eff_opt {
                match eff_str.parse::<i32>() {
                    Ok(eff) => {
                        if let Some(power) = self.power_curve_interpolator.get_value(rpm) {
                            let fuel_flow = (get_fuel_use_per_sec_at_rpm(eff, GASOLINE_LHV, power) * 3.6).round() as i32;
                            let _ = self.projected_fuel_flow.insert(rpm, fuel_flow);
                        }
                    }
                    Err(e) => warn!("Failed to update projected fuel flow. {}", e.to_string())
                }
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

            let eff_vec : Vec<(i32, i32)> = self.updated_data.iter()
                .filter_map(|(key, value)| {
                    value.clone().and_then(|s| s.parse::<i32>().ok().map(|parsed_value| (*key, parsed_value)))
                })
                .collect();
            let eff_interpolator = LutInterpolator::from_vec(eff_vec);

            // The lut values should be: rpm, kg/hr
            // The max-flow should be weighted to the upper end of the rev-range as racing is usually done in that range.
            // This is probably enough of a fallback as this will only be used if a lut isn't found and that will be
            // calculated below
            let max_flow_entry_rpm: i32 = idle + (0.7 * (limiter - idle) as f64).round() as i32;
            let p_eff = match eff_interpolator.get_value(max_flow_entry_rpm) {
                Some(v) => v.round() as i32,
                None => return Err("Failed to get max_flow eff val".to_string())
            };
            let p_power = match self.power_curve_interpolator.get_value(max_flow_entry_rpm) {
                Some(v) => v,
                None => return Err("Failed to get max_flow eff val".to_string())
            };
            let max_fuel_flow = get_fuel_use_kg_per_hour(p_eff, GASOLINE_LHV, p_power);

            let mut max_flow_lut: Vec<(i32, i32)> = Vec::new();
            for (rpm, eff_opt) in self.updated_data.iter() {
                let power = match self.power_curve_interpolator.get_value(*rpm) {
                    Some(v) => v,
                    None => {
                        warn!("Failed to interpolate power val @{}rpm. Skipping value in max_flow lut", rpm);
                        continue;
                    }
                };
                let eff = match eff_opt {
                    Some(eff_str) => match eff_str.parse::<i32>() {
                        Ok(eff) => eff,
                        Err(e) =>  {
                            warn!("Can't parse efficiency input @{}rpm. {}. Skipping value in max_flow lut", rpm, e.to_string());
                            continue
                        }
                    }
                    None => {
                        warn!("Missing efficiency input for {}. Interpolating from known points", rpm);
                        match eff_interpolator.get_value(*rpm) {
                            Some(eff) => eff.round() as i32,
                            None => {
                                warn!("Couldn't interpolate efficiency input @{}rpm. Skipping value in max_flow lut", rpm);
                                continue
                            }
                        }
                    },
                };
                max_flow_lut.push((*rpm, get_fuel_use_kg_per_hour(eff, GASOLINE_LHV, power)));
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

fn get_fuel_use_kg_per_hour(eff_percentage: i32, fuel_lhv: f64, power_kw: f64) -> i32{
    (get_fuel_use_per_sec_at_rpm(eff_percentage, fuel_lhv, power_kw) * 3.6).round() as i32
}

// TODO this would be a useful func on one of the engine structs; come back and refactor
fn get_min_max_rpms(engine_ini: &Engine) -> Result<(i32, i32), String> {
    match EngineData::load_from_parent(engine_ini) {
        Ok(ed) => {
            Ok((ed.minimum, ed.limiter))
        }
        Err(e) => {
            return Err(format!("Failed to load engine data. {}", e.to_string()));
        }
    }
}

fn load_drive_type(car: &mut Car) -> Result<DriveType, String> {
    let drivetrain = Drivetrain::from_car(car).map_err(|e|{
        format!("Failed to load {}. {}", Drivetrain::INI_FILENAME.to_string(), e.to_string())
    })?;
    Ok(extract_mandatory_section::<data::drivetrain::Traction>(&drivetrain).map_err(|_|{
        format!("{} is missing data section 'Traction'", Drivetrain::INI_FILENAME.to_string())
    })?.drive_type)
}

fn create_engine_power_interpolator(engine: &Engine,
                                    mechanical_efficiency: f64,
                                    boost_interpolator_opt: Option<LutInterpolator<f64, f64>>)
    -> Result<LutInterpolator<i32, f64>, String>
{
    match PowerCurve::load_from_parent(engine) {
        Ok(curve) => {
            let power_curve_vec: Vec<(i32, f64)> = curve.get_lut().to_vec().into_iter().map(
                |(rpm, torque)|{
                    let mut scaled_torque = (torque / (mechanical_efficiency * 100.0)) * 100.0;
                    if let Some(boost_interpolator) = &boost_interpolator_opt {
                        scaled_torque = match boost_interpolator.get_value(rpm as f64) {
                            None => scaled_torque,
                            Some(boost) => scaled_torque * (1.0 + boost)
                        }
                    }
                    let power = (scaled_torque * (rpm as f64) * 2.0 * std::f64::consts::PI) / (60.0 * 1000.0);
                    (rpm, power)
                }
            ).collect();
            Ok(LutInterpolator::from_vec(power_curve_vec))
        },
        Err(e) => {
            Err(format!("Failed to load engine curve data. {}", e.to_string()))
        }
    }
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
