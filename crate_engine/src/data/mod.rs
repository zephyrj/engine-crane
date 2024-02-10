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

pub mod beam_ng_mod;
pub mod direct_export;

use std::fmt::{Display};
use std::io::{Read, Write};
use std::path::Path;
use bincode::{deserialize_from, Options, serialize_into};
use serde::{Deserialize, Serialize};
use sha2::{Digest};
use crate::source::{BEAM_NG_MOD_SOURCE_ID, DIRECT_EXPORT_SOURCE_ID};
use crate::CrateEngineMetadata;

#[derive(Debug)]
pub enum CrateEngineData {
    BeamNGMod(beam_ng_mod::Data),
    DirectExport(direct_export::DataV1)
}

impl CrateEngineData {
    pub fn from_beamng_mod_zip(mod_path: &Path, options: beam_ng_mod::CreationOptions) -> Result<CrateEngineData, String> {
        Ok(CrateEngineData::BeamNGMod(beam_ng_mod::Data::from_beamng_mod_zip(mod_path, options)?))
    }

    pub fn from_reader(metadata: &CrateEngineMetadata, reader: &mut impl Read) -> Result<CrateEngineData, String> {
        let source = metadata.get_source();
        match source.source_id {
            BEAM_NG_MOD_SOURCE_ID => {
                Ok(CrateEngineData::BeamNGMod(beam_ng_mod::Data::from_reader(metadata, reader)?))
            },
            DIRECT_EXPORT_SOURCE_ID=> {
                let internal_data =
                    deserialize_from(reader).map_err(|e| {
                        format!("Failed to deserialise {} crate engine. {}", 1, e.to_string())
                    })?;
                Ok(CrateEngineData::DirectExport(internal_data))
            },
            i => Err(format!("Unknown data source with id {}", i))
        }
    }

    pub fn version_int(&self) -> u16 {
        match self {
            CrateEngineData::BeamNGMod(d) => d.version_int(),
            CrateEngineData::DirectExport(d) => d.version_int()
        }
    }

    pub fn serialize_into(&self, writer: &mut impl Write) -> bincode::Result<()> {
        match self {
            CrateEngineData::BeamNGMod(d) => d.serialize_into(writer),
            CrateEngineData::DirectExport(d) => serialize_into(writer, d),
        }
    }
}
