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
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum DataSource {
    BeamNGMod(BeamNGHashData),
    TomlExport(TomlHashData)
}

#[derive(Debug, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
struct BeamNGHashData {
    engine_jbeam_hash: Option<[u8; 32]>,
    automation_data_hash: Option<[u8; 32]>,
}

#[derive(Debug, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
struct TomlHashData {
    hash: Option<[u8; 32]>
}
