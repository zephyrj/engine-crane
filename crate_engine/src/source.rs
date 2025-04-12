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
use serde::{Deserialize, Serialize};

pub const BEAM_NG_MOD_SOURCE_ID: u16 = 1;
pub const DIRECT_EXPORT_SOURCE_ID: u16 = 2;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct DataSource {
    pub source_id: u16,
    pub hashes: Vec<Option<[u8; 32]>>
}

impl DataSource {
    pub fn from_beam_ng_mod(hashes: Vec<Option<[u8; 32]>>) -> Self {
        DataSource { source_id: BEAM_NG_MOD_SOURCE_ID, hashes }
    }

    pub fn from_direct_export() -> Self {
        DataSource { source_id: DIRECT_EXPORT_SOURCE_ID, hashes: Vec::new() }
    }

    pub fn source_name(&self) -> String {
        match self.source_id {
            BEAM_NG_MOD_SOURCE_ID => String::from("BeamNG Mod"),
            DIRECT_EXPORT_SOURCE_ID => String::from("Direct Automation Export"),
            _ => String::from("Unknown")
        }
    }
}
