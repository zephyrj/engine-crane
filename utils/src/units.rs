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

pub fn kw_to_bhp(power_kw: f64) -> f64 {
    power_kw * 1.341
}

pub fn calculate_power_kw(rpm: f32, torque: f32) -> f32 {
    (torque * rpm * 2.0 * std::f32::consts::PI) / (60.0 * 1000.0)
}

pub fn g_min_to_kg_hour(g_per_min: f64) -> f64 {
    (g_per_min / 1000.0) * 60.0
}

pub fn kg_hour_to_g_min(kg_per_hour: f64) -> f64 {
    (kg_per_hour * 1000.0) / 60.0
}

