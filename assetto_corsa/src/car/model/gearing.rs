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

use std::collections::BTreeMap;
use crate::Car;
use crate::{Result, Error, ErrorKind};
use crate::car::data::{Drivetrain, Engine};
use crate::car::data::drivetrain::{Gearbox, Traction};
use crate::car::data::drivetrain::traction::DriveType;
use crate::car::data::engine::{EngineData, PowerCurve};
use crate::car::data::tyres::tyre_sets::TyreCompounds;
use crate::car::data::tyres::Tyres;
use crate::traits::MandatoryDataSection;

pub struct GearingCalculator {
    power_curve_data: BTreeMap<i32, f64>,
    idle_rpm: i32,
    rpm_limit: i32,
    gear_ratios: Vec<f64>,
    final_drive: f64,
    drive_wheel_radius: f64,
}

impl GearingCalculator {
    pub fn from_car(car: &mut Car) -> Result<GearingCalculator> {
        let power_curve_data: BTreeMap<i32, f64>;
        let idle_rpm: i32;
        let rpm_limit: i32;
        let gear_ratios: Vec<f64>;
        let final_drive: f64;
        let drivetype;
        let drive_wheel_radius: f64;

        {
            let engine = Engine::from_car(car)?;
            let power_curve = PowerCurve::load_from_parent(&engine)?;
            power_curve_data = power_curve.get_curve_data();
            let engine_data = EngineData::load_from_parent(&engine)?;
            idle_rpm = engine_data.minimum;
            rpm_limit = engine_data.limiter;
        }

        {
            let drivetrain = Drivetrain::from_car(car)?;
            let gearbox_data = Gearbox::load_from_parent(&drivetrain)?;
            gear_ratios = gearbox_data.gear_ratios().clone();
            final_drive = gearbox_data.final_drive();
            let traction_data  = Traction::load_from_parent(&drivetrain)?;
            drivetype = traction_data.drive_type;
        }

        {
            let tyres = Tyres::from_car(car)?;
            let tyre_compound = TyreCompounds::load_from_parent(&tyres)?;
            let tyre_set = tyre_compound.get_default_set().ok_or(
                Error::new(ErrorKind::IniParseError, "Couldn't find default tyre set".to_string())
            )?;
            match drivetype {
                DriveType::FWD => {
                    drive_wheel_radius = tyre_set.front_data().radius();
                }
                DriveType::RWD | DriveType::AWD | DriveType::AWD2 => {
                    drive_wheel_radius = tyre_set.rear_data().radius();
                }
            }
        }

        Ok( GearingCalculator {
            power_curve_data, idle_rpm, rpm_limit, gear_ratios, final_drive, drive_wheel_radius
        })
    }

    pub fn set_gear_ratios(&mut self, ratios: Vec<f64>) {
        self.gear_ratios = ratios
    }

    pub fn set_final_drive(&mut self, ratio: f64) {
        self.final_drive = ratio
    }

    pub fn min_rpm(&self) -> i32 {
        self.idle_rpm
    }

    pub fn max_rpm(&self) -> i32 {
        self.rpm_limit
    }

    pub fn max_gear_idx(&self) -> usize {
        self.gear_ratios.len() - 1
    }

    /// Get the max possible speed (KM/H) the provided gear
    /// could reach
    ///
    /// - `gear_idx`: The index of the gear (indexed from 0)
    pub fn max_speed_for_gear(&self, gear_idx: usize) -> f64 {
        self.engine_rpm_to_wheel_speed(self.max_rpm(), gear_idx) * 3.6
    }

    /// Get the max possible speed (KM/H) the provided gear
    /// ratio reach
    ///
    /// - `ratio`: The ratio of the gear expressed as a decimal number
    pub fn max_speed_for_ratio(&self, ratio: f64) -> f64 {
        ((self.max_rpm() as f64 * 2.0 * std::f64::consts::PI * self.drive_wheel_radius) /
            (60.0 * ratio * self.final_drive)) * 3.6
    }

    pub fn max_speed(&self) -> f64 {
        let mut max_speed: f64 = 0.0;
        self.gear_ratios.iter().for_each(|ratio|{
            max_speed = max_speed.max(self.max_speed_for_ratio(*ratio));
        });
        max_speed
    }

    pub fn calculate_speed_plot_for_gear(&self, gear_idx: usize, rpm_increments: Option<i32>) -> Vec<(f64, f64)> {
        let mut plot_data: Vec<(f64, f64)> = Vec::new();
        if gear_idx >= self.gear_ratios.len() {
            return plot_data;
        }
        let increment = rpm_increments.unwrap_or( 100);
        let mut engine_rpm = self.min_rpm();
        loop {
            if engine_rpm > self.max_rpm() {
                break;
            }
            plot_data.push((self.engine_rpm_to_wheel_speed(engine_rpm, gear_idx)  *3.6, engine_rpm as f64));
            engine_rpm += increment;
        }
        plot_data
    }

    pub fn calculate_speed_plot(&self, rpm_increments: Option<i32>) -> Vec<Vec<(f64, f64)>> {
        let mut plot_data: Vec<Vec<(f64, f64)>> = Vec::new();
        for gear_idx in 0..self.gear_ratios.len() {
            plot_data.push(self.calculate_speed_plot_for_gear(gear_idx, rpm_increments))
        }
        plot_data
    }

    pub fn wheel_torque_at(&self, rpm: i32, gear_index: usize) -> f64 {
        let engine_torque_at_rpm = self.interpolate_engine_torque(rpm);
        let wheel_torque = (engine_torque_at_rpm * self.gear_ratios[gear_index] * self.final_drive) / self.drive_wheel_radius;
        wheel_torque
    }

    pub fn wheel_force_at(&self, rpm: i32, gear_index: usize) -> f64 {
        self.wheel_torque_at(rpm, gear_index) / self.drive_wheel_radius
    }

    pub fn engine_rpm_to_wheel_speed(&self, engine_rpm: i32, gear_ratio_idx: usize) -> f64 {
        // Calculate vehicle speed in m/s
        let vehicle_speed = (engine_rpm as f64 * 2.0 * std::f64::consts::PI * self.drive_wheel_radius) /
            (60.0 * self.gear_ratios[gear_ratio_idx] * self.final_drive);
        vehicle_speed
    }

    fn interpolate_engine_torque(&self, rpm: i32) -> f64 {
        // Linear interpolation to find torque at a given RPM
        let mut prev_rpm = 0.0;
        let mut prev_torque = 0.0;
        let mut found = false;

        for (i, rpm_value) in self.power_curve_data.keys().enumerate() {
            if *rpm_value >= rpm {
                found = true;
                let next_torque = self.power_curve_data[rpm_value];
                let next_rpm = *rpm_value as f64;
                if i == 0 {
                    return next_torque;
                }

                // Linear interpolation
                let slope = (next_torque - prev_torque) / (next_rpm - prev_rpm);
                return prev_torque + slope * (rpm as f64 - prev_rpm);
            }

            prev_rpm = *rpm_value as f64;
            prev_torque = self.power_curve_data[rpm_value];
        }

        if !found {
            return prev_torque;
        }
        0.0
    }
}
