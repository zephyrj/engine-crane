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

#[derive(Debug)]
pub struct TyreData {
    name: String,
    short_name: String,
    width: f64,
    radius: f64
}

#[derive(Debug)]
pub struct TyreSet {
    front: TyreData,
    rear: TyreData
}

#[derive(Debug)]
pub struct TyreSets {
    sets: Vec<TyreSet>,
    default_set_idx: usize
}