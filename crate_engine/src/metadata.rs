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

use std::io::{Read, Write};
use std::mem;
use bincode::serde::{decode_from_std_read, encode_into_std_write};
use serde::{Deserialize, Serialize};
use zephyrj_automation_tools as automation;
use automation::{AspirationType, BlockConfig, HeadConfig, Valves};
use zephyrj_automation_tools::BlockType;
use crate::source::DataSource;


pub(crate) type CurrentMetadataType = MetadataV3;

pub enum CrateEngineMetadata {
    MetadataV1(MetadataV1),
    MetadataV2(MetadataV2),
    MetadataV3(MetadataV3),
}

impl CrateEngineMetadata {
    pub fn from_current_version(inner_type: CurrentMetadataType) -> CrateEngineMetadata {
        CrateEngineMetadata::MetadataV3(inner_type)
    }

    pub fn from_reader(reader: &mut impl Read) -> Result<CrateEngineMetadata, String> {
        let mut buf = [0u8; mem::size_of::<u16>()];
        reader.read_exact(&mut buf).map_err(|e| format!("Failed to read metadata. {}", e.to_string()))?;
        let metadata_version = u16::from_le_bytes(buf);
        match metadata_version {
            MetadataV1::VERSION_U16 => {
                let metadata = decode_from_std_read(reader, bincode::config::legacy()).map_err(|e| format!("Failed to deserialize metadata. {}", e.to_string()))?;
                Ok(CrateEngineMetadata::MetadataV1(metadata))
            },
            MetadataV2::VERSION_U16 => {
                let metadata = decode_from_std_read(reader, bincode::config::legacy()).map_err(|e| format!("Failed to deserialize metadata. {}", e.to_string()))?;
                Ok(CrateEngineMetadata::MetadataV2(metadata))
            },
            MetadataV3::VERSION_U16 => {
                let metadata = decode_from_std_read(reader, bincode::config::standard()).map_err(|e| format!("Failed to deserialize metadata. {}", e.to_string()))?;
                Ok(CrateEngineMetadata::MetadataV3(metadata))
            }
            _ => Err(format!("Unknown metadata version {}", metadata_version))
        }
    }

    pub fn get_source(&self) -> DataSource {
        match self {
            CrateEngineMetadata::MetadataV1(m) => {
                DataSource::from_beam_ng_mod(vec![m.engine_jbeam_hash, m.automation_data_hash])
            },
            CrateEngineMetadata::MetadataV2(m) => m.source.clone(),
            CrateEngineMetadata::MetadataV3(m) => m.source.clone(),
        }
    }

    pub fn get_metadata_version_u16(&self) -> u16 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.get_version_u16(),
            CrateEngineMetadata::MetadataV2(m) => m.get_version_u16(),
            CrateEngineMetadata::MetadataV3(m) => m.get_version_u16(),
        }
    }

    pub fn serialize_into(&self, writer: &mut impl Write) -> Result<usize, bincode::error::EncodeError> {
        writer.write(&self.get_metadata_version_u16().to_le_bytes()).map_err(|e| {
            bincode::error::EncodeError::Io {inner:e,index:0}
        })?;
        match self {
            CrateEngineMetadata::MetadataV1(m) => encode_into_std_write(&m, writer, bincode::config::legacy()),
            CrateEngineMetadata::MetadataV2(m) => encode_into_std_write(&m, writer, bincode::config::legacy()),
            CrateEngineMetadata::MetadataV3(m) => encode_into_std_write(&m, writer, bincode::config::standard()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            CrateEngineMetadata::MetadataV1(d) => { &d.name }
            CrateEngineMetadata::MetadataV2(d) => { &d.name }
            CrateEngineMetadata::MetadataV3(d) => { &d.name }
        }
    }

    pub fn data_version(&self) -> u16 {
        match self {
            CrateEngineMetadata::MetadataV1(d) => { *&d.data_version }
            CrateEngineMetadata::MetadataV2(d) => { *&d.data_version }
            CrateEngineMetadata::MetadataV3(d) => { *&d.data_version }
        }
    }

    pub fn automation_version(&self) -> u64 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => { m.automation_version }
            CrateEngineMetadata::MetadataV2(m) => { m.automation_version }
            CrateEngineMetadata::MetadataV3(m) => { m.automation_version }
        }
    }

    pub fn build_year(&self) -> u16 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.build_year,
            CrateEngineMetadata::MetadataV2(m) => m.build_year,
            CrateEngineMetadata::MetadataV3(m) => m.build_year,
        }
    }

    pub fn block_config(&self) -> Option<&BlockConfig> {
        match self {
            CrateEngineMetadata::MetadataV1(m) => Some(&m.block_config),
            CrateEngineMetadata::MetadataV2(m) => Some(&m.block_config),
            CrateEngineMetadata::MetadataV3(_m) => None,
        }
    }

    pub fn block_type(&self) -> Option<&BlockType> {
        match self {
            CrateEngineMetadata::MetadataV1(_m) => None,
            CrateEngineMetadata::MetadataV2(_m) => None,
            CrateEngineMetadata::MetadataV3(m) => Some(&m.block_type),
        }
    }

    pub fn head_config(&self) -> &HeadConfig {
        match self {
            CrateEngineMetadata::MetadataV1(m) => &m.head_config,
            CrateEngineMetadata::MetadataV2(m) => &m.head_config,
            CrateEngineMetadata::MetadataV3(m) => &m.head_config,
        }
    }
    
    pub fn cylinders(&self) -> u16 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.block_config.cylinders(),
            CrateEngineMetadata::MetadataV2(m) => m.block_config.cylinders(),
            CrateEngineMetadata::MetadataV3(m) => m.cylinders
        }
    }

    pub fn valves(&self) -> &Valves {
        match self {
            CrateEngineMetadata::MetadataV1(m) => &m.valves,
            CrateEngineMetadata::MetadataV2(m) => &m.valves,
            CrateEngineMetadata::MetadataV3(m) => &m.valves,
        }
    }

    pub fn capacity(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.capacity,
            CrateEngineMetadata::MetadataV2(m) => m.capacity,
            CrateEngineMetadata::MetadataV3(m) => m.capacity,
        }
    }

    pub fn aspiration(&self) -> &AspirationType {
        match self {
            CrateEngineMetadata::MetadataV1(m) => &m.aspiration,
            CrateEngineMetadata::MetadataV2(m) => &m.aspiration,
            CrateEngineMetadata::MetadataV3(m) => &m.aspiration,
        }
    }

    pub fn fuel(&self) -> &str {
        match self {
            CrateEngineMetadata::MetadataV1(m) => &m.fuel,
            CrateEngineMetadata::MetadataV2(m) => &m.fuel,
            CrateEngineMetadata::MetadataV3(m) => &m.fuel,
        }
    }

    pub fn peak_power(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.peak_power,
            CrateEngineMetadata::MetadataV2(m) => m.peak_power,
            CrateEngineMetadata::MetadataV3(m) => m.peak_power,
        }
    }

    pub fn peak_power_rpm(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.peak_power_rpm,
            CrateEngineMetadata::MetadataV2(m) => m.peak_power_rpm,
            CrateEngineMetadata::MetadataV3(m) => m.peak_power_rpm,
        }
    }

    pub fn peak_torque(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.peak_torque,
            CrateEngineMetadata::MetadataV2(m) => m.peak_torque,
            CrateEngineMetadata::MetadataV3(m) => m.peak_torque,
        }
    }

    pub fn peak_torque_rpm(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.peak_torque_rpm,
            CrateEngineMetadata::MetadataV2(m) => m.peak_torque_rpm,
            CrateEngineMetadata::MetadataV3(m) => m.peak_torque_rpm,
        }
    }

    pub fn max_rpm(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.max_rpm,
            CrateEngineMetadata::MetadataV2(m) => m.max_rpm,
            CrateEngineMetadata::MetadataV3(m) => m.max_rpm,
        }
    }
    
    pub fn block_description(&self) -> String {
        match self {
            CrateEngineMetadata::MetadataV1(m) => format!("{} {} {}", m.block_config, m.head_config, m.valves),
            CrateEngineMetadata::MetadataV2(m) => format!("{} {} {}", m.block_config, m.head_config, m.valves),
            CrateEngineMetadata::MetadataV3(m) => format!("{}{} {} {}", m.block_type, m.cylinders, m.head_config, m.valves),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataV1 {
    pub data_version: u16,
    pub automation_version: u64,
    pub name: String,
    pub engine_jbeam_hash: Option<[u8; 32]>,
    pub automation_data_hash: Option<[u8; 32]>,
    pub build_year: u16,
    pub block_config: BlockConfig,
    pub head_config: HeadConfig,
    pub valves: Valves,
    pub capacity: u32,
    pub aspiration: AspirationType,
    pub fuel: String,
    pub peak_power: u32,
    pub peak_power_rpm: u32,
    pub peak_torque: u32,
    pub peak_torque_rpm: u32,
    pub max_rpm: u32
}

impl MetadataV1 {
    const VERSION_U16: u16 = 1_u16;
    pub fn get_version_u16(&self) -> u16 {
        Self::VERSION_U16
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataV2 {
    pub source: DataSource,
    pub data_version: u16,
    pub automation_version: u64,
    pub name: String,
    pub build_year: u16,
    pub block_config: BlockConfig,
    pub head_config: HeadConfig,
    pub valves: Valves,
    pub capacity: u32,
    pub aspiration: AspirationType,
    pub fuel: String,
    pub peak_power: u32,
    pub peak_power_rpm: u32,
    pub peak_torque: u32,
    pub peak_torque_rpm: u32,
    pub max_rpm: u32
}

impl MetadataV2 {
    const VERSION_U16: u16 = 2_u16;
    pub fn get_version_u16(&self) -> u16 {
        Self::VERSION_U16
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataV3 {
    pub source: DataSource,
    pub data_version: u16,
    pub automation_version: u64,
    pub name: String,
    pub build_year: u16,
    pub block_type: BlockType,
    pub head_config: HeadConfig,
    pub cylinders: u16,
    pub valves: Valves,
    pub capacity: u32,
    pub aspiration: AspirationType,
    pub fuel: String,
    pub peak_power: u32,
    pub peak_power_rpm: u32,
    pub peak_torque: u32,
    pub peak_torque_rpm: u32,
    pub max_rpm: u32
}

impl MetadataV3 {
    const VERSION_U16: u16 = 3_u16;
    pub fn get_version_u16(&self) -> u16 {
        Self::VERSION_U16
    }
}
