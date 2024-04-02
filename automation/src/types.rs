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

use std::fmt::{Display, Formatter};
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum BlockConfig {
    V16_90,
    V10_90,
    V8_90,
    V6_90,
    V12_60,
    V8_60,
    V6_60,
    I6,
    I5,
    I4,
    I3,
    Boxer6,
    Boxer4,
    Unknown(String)
}

impl FromStr for BlockConfig {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<BlockConfig, std::convert::Infallible> {
        match s {
            "EngBlock_V16_Name" => Ok(BlockConfig::V16_90),
            "EngBlock_V10_Name" => Ok(BlockConfig::V10_90),
            "EngBlock_V8_Name" => Ok(BlockConfig::V8_90),
            "EngBlock_V6_V90_Name" => Ok(BlockConfig::V6_90),
            "EngBlock_V12_Name" => Ok(BlockConfig::V12_60),
            "EngBlock_V8_V60_Name" => Ok(BlockConfig::V8_60),
            "EngBlock_V6_Name" => Ok(BlockConfig::V6_60),
            "EngBlock_Inl6_Name" => Ok(BlockConfig::I6),
            "EngBlock_Inl5_Name" => Ok(BlockConfig::I5),
            "EngBlock_Inl4_Name" => Ok(BlockConfig::I4),
            "EngBlock_Inl3_Name" => Ok(BlockConfig::I3),
            "EngBlock_Box6_Name" => Ok(BlockConfig::Boxer6),
            "EngBlock_Box4_Name" => Ok(BlockConfig::Boxer4),
            _ => Ok(BlockConfig::Unknown(s.to_string())),
        }
    }
}

impl Display for BlockConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockConfig::V16_90 => write!(f, "90° V16"),
            BlockConfig::V10_90 => write!(f, "90° V10"),
            BlockConfig::V8_90 => write!(f, "90° V8"),
            BlockConfig::V6_90 => write!(f, "90° V6"),
            BlockConfig::V12_60 => write!(f, "60° V12"),
            BlockConfig::V8_60 => write!(f, "60° V8"),
            BlockConfig::V6_60 => write!(f, "60° V6"),
            BlockConfig::I6 => write!(f, "Inline 6"),
            BlockConfig::I5 => write!(f, "Inline 5"),
            BlockConfig::I4 => write!(f, "Inline 4"),
            BlockConfig::I3 => write!(f, "Inline 3"),
            BlockConfig::Boxer6 => write!(f, "Boxer 6"),
            BlockConfig::Boxer4 => write!(f, "Boxer 4"),
            BlockConfig::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum HeadConfig {
    OHV,
    SOHC,
    DAOHC,
    DOHC,
    Unknown(String)
}

impl FromStr for HeadConfig {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<HeadConfig, std::convert::Infallible> {
        match s {
            "Head_PushRod_Name" => Ok(HeadConfig::OHV),
            "Head_OHC_Name" => Ok(HeadConfig::SOHC),
            "Head_DirectOHC_Name" => Ok(HeadConfig::DAOHC),
            "Head_DuelOHC_Name" => Ok(HeadConfig::DOHC),
            _ => Ok(HeadConfig::Unknown(s.to_string())),
        }
    }
}

impl Display for HeadConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HeadConfig::OHV => write!(f, "OHV"),
            HeadConfig::SOHC => write!(f, "SOHC"),
            HeadConfig::DAOHC => write!(f, "DAOHC"),
            HeadConfig::DOHC => write!(f, "DOHC"),
            HeadConfig::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Valves {
    Two,
    Three,
    Four,
    Five,
    Unknown(String)
}

impl Valves {
    pub fn from_int(i: u16) -> Result<Valves, std::convert::Infallible> {
        match i {
            2 => Ok(Valves::Two),
            3 => Ok(Valves::Three),
            4 => Ok(Valves::Four),
            5 => Ok(Valves::Five),
            _ => Ok(Valves::Unknown(i.to_string())),
        }
    }
}

impl FromStr for Valves {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Valves, std::convert::Infallible> {
        match s {
            "ValveCount_2_Name" => Ok(Valves::Two),
            "ValveCount_3_Name" => Ok(Valves::Three),
            "ValveCount_4_Name" => Ok(Valves::Four),
            "ValveCount_5_Name" => Ok(Valves::Five),
            _ => Ok(Valves::Unknown(s.to_string())),
        }
    }
}

impl Display for Valves {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Valves::Two => write!(f, "2v"),
            Valves::Three => write!(f, "3v"),
            Valves::Four => write!(f, "4v"),
            Valves::Five => write!(f, "5v"),
            Valves::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AspirationType {
    NA,
    Turbo,
    Unknown(String)
}

impl FromStr for AspirationType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<AspirationType, std::convert::Infallible> {
        match s {
            "Aspiration_Natural_Name" => Ok(AspirationType::NA),
            "Aspiration_Turbo_Name" => Ok(AspirationType::Turbo),
            _ => Ok(AspirationType::Unknown(s.to_string())),
        }
    }
}

impl Display for AspirationType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AspirationType::NA => write!(f, "Naturally Aspirated"),
            AspirationType::Turbo => write!(f, "Turbocharged"),
            AspirationType::Unknown(s) => write!(f, "{}", s),
        }
    }
}

