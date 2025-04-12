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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DataV1 {
    pub exporter_script_version: u32,
    pub string_data: BTreeMap<String, BTreeMap<String, String>>,
    pub float_data: BTreeMap<String, BTreeMap<String, f32>>,
    pub curve_data: BTreeMap<String, BTreeMap<usize, f32>>,
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

    pub fn add_string(&mut self, group_name: String, key: String, value: String) {
        if !self.string_data.contains_key(&group_name) {
            self.string_data.insert(group_name.clone(), BTreeMap::new());
        };
        let group_map = self.string_data.get_mut(&group_name).unwrap();
        group_map.insert(key, value);
    }

    pub fn add_float(&mut self, group_name: String, key: String, value: f32) {
        if !self.float_data.contains_key(&group_name) {
            self.float_data.insert(group_name.clone(), BTreeMap::new());
        };
        let group_map = self.float_data.get_mut(&group_name).unwrap();
        group_map.insert(key, value);
    }

    pub fn add_curve_data(&mut self, curve_name: String, index: usize, value: f32) {
        let curve_map = self.curve_data.entry(curve_name).or_insert(BTreeMap::new());
        curve_map.insert(index, value);
    }

    pub fn deduce_engine_name(&self) -> String {
        let backup_fam_name = String::from("UnknownFamily");
        let backup_var_name = String::from("UnknownVariant");
        match self.string_data.get("Info") {
            Some(info_data) => {
                let fam = info_data.get("FamilyName").unwrap_or(&backup_fam_name);
                let var = info_data.get("VariantName").unwrap_or(&backup_var_name);
                format!("{}-{}", fam, var)
            },
            None => format!("{}-{}", backup_fam_name, backup_var_name)
        }
    }
}
