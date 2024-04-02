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
use std::io::{Read, Write};
use bincode::{deserialize_from, serialize_into};
use serde::{Deserialize, Serialize};
use crate::CrateEngineMetadata;

#[derive(Debug)]
pub struct CreationOptions {
}

impl CreationOptions {
    pub fn default() -> CreationOptions {
        CreationOptions {}
    }
}

#[derive(Debug)]
pub enum Data {
    V1(DataV1)
}

impl Data {
    pub fn version_int(&self) -> u16 {
        match self {
            Data::V1(d) => d.version_int()
        }
    }

    pub fn from_reader(_metadata: &CrateEngineMetadata, reader: &mut impl Read) -> Result<Data, String> {
        let internal_data =
            deserialize_from(reader).map_err(|e| {
                format!("Failed to deserialise {} crate engine. {}", 1, e.to_string())
            })?;
        Ok(Data::V1(internal_data))
    }

    pub fn serialise_into(&self, writer: &mut impl Write) -> bincode::Result<()> {
        match self {
            Data::V1(d) => {
                serialize_into(writer, d)
            }
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DataV1 {
    pub exporter_script_version: u32,
    pub string_data: BTreeMap<String, String>,
    pub float_data: BTreeMap<String, f32>,
    pub curve_data: BTreeMap<String, Vec<f64>>,
    _car_file_data: Option<Vec<u8>>,
}

impl DataV1 {
    pub const VERSION: u16 = 1;
    pub fn version_int(&self) -> u16 {
        Self::VERSION
    }

    pub fn new() -> DataV1 {
        DataV1::default()
    }
}
