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
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use rusqlite::{Connection, Row};
use sha2::{Sha256, Digest};
use tracing::info;
use serde::{Serialize,Deserialize};

#[cfg(target_os = "windows")]
use directories::{BaseDirs, UserDirs};

#[cfg(target_os = "linux")]
use crate::STEAM_GAME_ID;

use utils::numeric::round_float_to;

pub enum SandboxVersion {
    Legacy,
    FourDotTwo,
    Ellisbury
}

impl SandboxVersion {
    pub fn from_version_number(version_num: u64) -> SandboxVersion {
        if version_num >= 2312150000 {
            return SandboxVersion::Ellisbury;
        } else if version_num < 2111220000 {
            return SandboxVersion::Legacy;
        }
        return SandboxVersion::FourDotTwo;
    }

    pub fn get_path(&self) -> Option<OsString> {
        return match self {
            SandboxVersion::Legacy => { get_db_path() }
            SandboxVersion::FourDotTwo => { get_db_path_4_2() }
            SandboxVersion::Ellisbury => { get_db_path_ellisbury() }
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SandboxVersion::Legacy => { "pre 4.2" }
            SandboxVersion::FourDotTwo => { "post 4.2" }
            SandboxVersion::Ellisbury => { "4.3 Ellisbury" }
        }
    }
}

impl Default for SandboxVersion {
    fn default() -> Self {
        SandboxVersion::FourDotTwo
    }
}

impl Display for SandboxVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EngineV1 {
    pub uuid: String,
    pub family_version: u64,
    pub variant_version: u64,
    pub family_uuid: String,
    pub family_name: String,
    pub variant_name: String,
    pub family_game_days: i32,
    pub variant_game_days: i32,
    pub family_quality: i32,
    pub block_config: String,
    pub block_material: String,
    pub block_type: String,
    pub head_type: String,
    pub head_material: String,
    pub valves: String,
    pub vvl: String,
    pub max_bore: f64,
    pub max_stroke: f64,
    pub crank: String,
    pub conrods: String,
    pub pistons: String,
    pub vvt: String,
    pub aspiration: String,
    pub intercooler_setting: f64,
    pub fuel_system_type: String,
    pub fuel_system: String,
    pub intake_manifold: String,
    pub intake: String,
    pub fuel_type: Option<String>,
    pub fuel_leaded: Option<i32>,
    pub headers: String,
    pub exhaust_count: String,
    pub exhaust_bypass_valves: String,
    pub cat: String,
    pub muffler_1: String,
    pub muffler_2: String,
    pub bore: f64,
    pub stroke: f64,
    pub capacity: f64,
    pub compression: f64,
    pub cam_profile_setting: f64,
    pub vvl_cam_profile_setting: f64,
    pub afr: Option<f64>,
    pub afr_lean: Option<f64>,
    pub rpm_limit: f64,
    pub ignition_timing_setting: f64,
    pub exhaust_diameter: f64,
    pub quality_bottom_end: i32,
    pub quality_top_end: i32,
    pub quality_aspiration: i32,
    pub quality_fuel_system: i32,
    pub quality_exhaust: i32,
    pub balance_shaft: Option<String>,
    pub spring_stiffness: Option<f64>,
    pub listed_octane: Option<i32>,
    pub tune_octane_offset: Option<i32>,
    pub aspiration_setup: Option<String>,
    pub aspiration_item_1: Option<String>,
    pub aspiration_item_2: Option<String>,
    pub aspiration_item_suboption_1: Option<String>,
    pub aspiration_item_suboption_2: Option<String>,
    pub aspiration_boost_control: Option<String>,
    pub charger_size_1: Option<f64>,
    pub charger_size_2: Option<f64>,
    pub charger_tune_1: Option<f64>,
    pub charger_tune_2: Option<f64>,
    pub charger_max_boost_1: Option<f64>,
    pub charger_max_boost_2: Option<f64>,
    pub turbine_size_1: Option<f64>,
    pub turbine_size_2: Option<f64>,
    pub adjusted_afr: f64,
    pub average_cruise_econ: f64,
    pub cooling_required: f64,
    pub econ: f64,
    pub econ_eff: f64,
    pub min_econ: f64,
    pub worst_econ: f64,
    pub emissions: f64,
    pub engineering_cost: f64,
    pub engineering_time: f64,
    pub idle: f64,
    pub idle_speed: f64,
    pub mttf: f64,
    pub man_hours: f64,
    pub material_cost: f64,
    pub noise: f64,
    pub peak_boost: f64,
    pub peak_boost_rpm: Option<f64>,
    pub performance_index: f64,
    pub ron: f64,
    pub reliability_post_engineering: Option<f64>,
    pub responsiveness: f64,
    pub service_cost: f64,
    pub smoothness: f64,
    pub tooling_costs: f64,
    pub total_cost: f64,
    pub weight: f64,
    pub peak_torque_rpm: f64,
    pub peak_torque: f64,
    pub peak_power: f64,
    pub peak_power_rpm: f64,
    pub max_rpm: f64,
    pub rpm_curve: Vec<f64>,
    pub power_curve: Vec<f64>,
    pub torque_curve: Vec<f64>,
    pub boost_curve: Vec<f64>,
    pub econ_curve: Vec<f64>,
    pub econ_eff_curve: Vec<f64>
}

impl EngineV1 {
    pub fn load_from_row(row: &Row) -> rusqlite::Result<EngineV1> {
        Ok(EngineV1 {
            uuid: row.get("v_uuid")?,
            family_version: row.get("f_version")?,
            variant_version: row.get("v_version")?,
            family_uuid: row.get("f_uuid")?,
            family_name: row.get("f_name")?,
            variant_name: row.get("v_name")?,
            family_game_days: row.get("f_days")?,
            variant_game_days: row.get("v_days")?,
            block_config: row.get("BlockConfig")?,
            block_material: row.get("BlockMaterial")?,
            block_type: row.get("BlockType")?,
            head_type: row.get("Head")?,
            head_material: row.get("HeadMaterial")?,
            valves: row.get("Valves")?,
            vvl: row.get("VVL")?,
            max_bore: row.get("MaxBore")?,
            max_stroke: row.get("MaxStroke")?,
            family_quality: row.get("QualityFamily").unwrap_or(0),
            crank: row.get("Crank")?,
            conrods: row.get("Conrods")?,
            pistons: row.get("Pistons")?,
            vvt: row.get("VVT")?,
            aspiration: row.get("AspirationType")?,
            intercooler_setting: row.get("IntercoolerSetting")?,
            fuel_system_type: row.get("FuelSystemType")?,
            fuel_system: row.get("FuelSystem")?,
            fuel_leaded: row.get("FuelLeaded").unwrap_or(None),
            intake_manifold: row.get("IntakeManifold")?,
            intake: row.get("Intake")?,
            fuel_type: row.get("FuelType").unwrap_or(None),
            headers: row.get("Headers")?,
            exhaust_count: row.get("ExhaustCount")?,
            exhaust_bypass_valves: row.get("ExhaustBypassValves")?,
            cat: row.get("Cat")?,
            muffler_1: row.get("Muffler1")?,
            muffler_2: row.get("Muffler2")?,
            bore: row.get("VBore")?,
            stroke: row.get("VStroke")?,
            capacity: row.get("Capacity")?,
            compression: row.get("Compression")?,
            cam_profile_setting: row.get("CamProfileSetting")?,
            vvl_cam_profile_setting: row.get("VVLCamProfileSetting")?,
            afr: row.get("AFR")?,
            afr_lean: row.get("AFRLean")?,
            rpm_limit: row.get("RPMLimit")?,
            ignition_timing_setting: row.get("IgnitionTimingSetting")?,
            exhaust_diameter: row.get("ExhaustDiameter")?,
            quality_bottom_end: row.get("QualityBottomEnd")?,
            quality_top_end: row.get("QualityTopEnd")?,
            quality_aspiration: row.get("QualityAspiration")?,
            quality_fuel_system: row.get("QualityFuelSystem")?,
            quality_exhaust: row.get("QualityExhaust")?,
            balance_shaft: row.get("BalanceShaft").unwrap_or(None),
            spring_stiffness: row.get("SpringStiffnessSetting").unwrap_or(None),
            listed_octane: row.get("ListedOctane").unwrap_or(None),
            tune_octane_offset: row.get("TuneOctaneOffset").unwrap_or(None),
            aspiration_setup: row.get("AspirationSetup").unwrap_or(None),
            aspiration_item_1: row.get("AspirationItemOption_1").unwrap_or(None),
            aspiration_item_2: row.get("AspirationItemOption_2").unwrap_or(None),
            aspiration_item_suboption_1: row.get("AspirationItemSubOption_1").unwrap_or(None),
            aspiration_item_suboption_2: row.get("AspirationItemSubOption_2").unwrap_or(None),
            aspiration_boost_control: row.get("AspirationBoostControl").unwrap_or(None),
            charger_size_1: row.get("ChargerSize_1").unwrap_or(None),
            charger_size_2: row.get("ChargerSize_2").unwrap_or(None),
            charger_tune_1: row.get("ChargerTune_1").unwrap_or(None),
            charger_tune_2: row.get("ChargerTune_2").unwrap_or(None),
            charger_max_boost_1: row.get("ChargerMaxBoost_1").unwrap_or(None),
            charger_max_boost_2: row.get("ChargerMaxBoost_2").unwrap_or(None),
            turbine_size_1: row.get("TurbineSize_1").unwrap_or(None),
            turbine_size_2: row.get("TurbineSize_2").unwrap_or(None),
            adjusted_afr: row.get("AdjustedAFR")?,
            average_cruise_econ: row.get("AverageCruiseEcon")?,
            cooling_required: row.get("CoolingRequired")?,
            econ: row.get("Econ")?,
            econ_eff: row.get("EconEff")?,
            min_econ: row.get("MinEcon")?,
            worst_econ: row.get("WorstEcon")?,
            emissions: row.get("Emissions")?,
            engineering_cost: row.get("EngineeringCost")?,
            engineering_time: row.get("EngineeringTime")?,
            idle: row.get("Idle")?,
            idle_speed: row.get("IdleSpeed")?,
            mttf: row.get("MTTF")?,
            man_hours: row.get("ManHours")?,
            material_cost: row.get("MaterialCost")?,
            noise: row.get("Noise")?,
            peak_boost: row.get("PeakBoost")?,
            peak_boost_rpm: row.get("PeakBoostRPM").unwrap_or(None),
            performance_index: row.get("PerformanceIndex")?,
            ron: row.get("RON")?,
            reliability_post_engineering: row.get("ReliabilityPostEngineering").unwrap_or(None),
            responsiveness: row.get("Responsiveness")?,
            service_cost: row.get("ServiceCost")?,
            smoothness: row.get("Smoothness")?,
            tooling_costs: row.get("ToolingCosts")?,
            total_cost: row.get("TotalCost")?,
            weight: row.get("Weight")?,
            peak_torque_rpm: row.get("PeakTorqueRPM")?,
            peak_torque: row.get("PeakTorque")?,
            peak_power: row.get("PeakPower")?,
            peak_power_rpm: row.get("PeakPowerRPM")?,
            max_rpm: row.get("MaxRPM")?,
            rpm_curve: EngineV1::decode_graph_data(row, "RPMCurve")?,
            power_curve: EngineV1::decode_graph_data(row, "PowerCurve")?,
            torque_curve: EngineV1::decode_graph_data(row, "TorqueCurve")?,
            boost_curve: EngineV1::decode_graph_data(row, "BoostCurve")?,
            econ_curve: EngineV1::decode_graph_data(row, "EconCurve")?,
            econ_eff_curve: EngineV1::decode_graph_data(row, "EconEffCurve")?
        })
    }

    pub fn friendly_name(&self) -> String {
        format!("{} - {}", self.family_name, self.variant_name)
    }

    pub fn get_family_build_year(&self) -> u16 {
        internal_days_to_year(self.family_game_days)
    }

    pub fn get_variant_build_year(&self) -> u16 {
        internal_days_to_year(self.variant_game_days)
    }

    pub fn get_capacity_cc(&self) -> u32 {
        (self.capacity * 1000.0).round() as u32
    }

    pub fn get_block_config(&self) -> BlockConfig {
        self.block_config.parse().unwrap()
    }

    pub fn get_head_config(&self) -> HeadConfig {
        self.head_type.parse().unwrap()
    }

    pub fn get_aspiration(&self) -> AspirationType {
        self.aspiration.parse().unwrap()
    }

    pub fn get_valve_type(&self) -> Valves {
        self.valves.parse().unwrap()
    }

    pub fn family_data_checksum_data(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.family_version.to_string().as_bytes());
        hasher.update(&self.family_uuid.as_bytes());
        hasher.update(&self.family_name.as_bytes());
        hasher.update(&self.family_game_days.to_string().as_bytes());
        hasher.update(&self.family_quality.to_string().as_bytes());
        hasher.update(&self.block_config.as_bytes());
        hasher.update(&self.block_material.as_bytes());
        hasher.update(&self.block_type.as_bytes());
        hasher.update(&self.head_type.as_bytes());
        hasher.update(&self.head_material.as_bytes());
        hasher.update(&self.valves.as_bytes());
        hasher.update(&round_float_to(self.max_stroke, 10).to_string().as_bytes());
        hasher.update(&round_float_to(self.max_bore, 10).to_string().as_bytes());
        hasher.finalize().iter().map(|b| *b).collect()
    }

    pub fn family_data_checksum(&self) -> String {
        _sha256_data_to_string(self.family_data_checksum_data())
    }

    pub fn variant_data_checksum_data(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.variant_version.to_string().as_bytes());
        hasher.update(&self.family_uuid.as_bytes());
        hasher.update(&self.uuid.as_bytes());
        hasher.update(&self.variant_name.as_bytes());
        hasher.update(&self.variant_game_days.to_string().as_bytes());
        hasher.update(&self.vvl.as_bytes());
        hasher.update(&self.crank.as_bytes());
        hasher.update(&self.conrods.as_bytes());
        hasher.update(&self.pistons.as_bytes());
        hasher.update(&self.vvt.as_bytes());
        hasher.update(&self.aspiration.as_bytes());
        hasher.update(&round_float_to(self.intercooler_setting, 10).to_string().as_bytes());
        hasher.update(&self.fuel_system_type.as_bytes());
        hasher.update(&self.fuel_system.as_bytes());
        if let Some(str) = &self.fuel_type { hasher.update(str.as_bytes()); }
        if let Some(str) = &self.fuel_leaded { hasher.update(str.to_string().as_bytes()); }
        hasher.update(&self.intake_manifold.as_bytes());
        hasher.update(&self.intake.as_bytes());
        hasher.update(&self.headers.as_bytes());
        hasher.update(&self.exhaust_count.as_bytes());
        hasher.update(&self.exhaust_bypass_valves.as_bytes());
        hasher.update(&self.cat.as_bytes());
        hasher.update(&self.muffler_1.as_bytes());
        hasher.update(&self.muffler_2.as_bytes());
        hasher.update(&round_float_to(self.bore, 10).to_string().as_bytes());
        hasher.update(&round_float_to(self.stroke, 10).to_string().as_bytes());
        hasher.update(&round_float_to(self.capacity, 10).to_string().as_bytes());
        hasher.update(&round_float_to(self.compression, 10).to_string().as_bytes());
        hasher.update(&round_float_to(self.cam_profile_setting, 10).to_string().as_bytes());
        hasher.update(&round_float_to(self.vvl_cam_profile_setting, 10).to_string().as_bytes());
        if let Some(str) = &self.afr { hasher.update(&round_float_to(*str, 10).to_string().as_bytes()); }
        if let Some(str) = &self.afr_lean { hasher.update(&round_float_to(*str, 10).to_string().as_bytes()); }
        hasher.update(&round_float_to(self.rpm_limit, 10).to_string().as_bytes());
        hasher.update(&round_float_to(self.ignition_timing_setting, 10).to_string().as_bytes());
        hasher.update(&round_float_to(self.exhaust_diameter, 10).to_string().as_bytes());
        hasher.update(&self.quality_bottom_end.to_string().as_bytes());
        hasher.update(&self.quality_top_end.to_string().as_bytes());
        hasher.update(&self.quality_aspiration.to_string().as_bytes());
        hasher.update(&self.quality_fuel_system.to_string().as_bytes());
        hasher.update(&self.quality_exhaust.to_string().as_bytes());
        if let Some(str) = &self.balance_shaft { hasher.update(str.as_bytes()); }
        if let Some(str) = &self.spring_stiffness { hasher.update(round_float_to(*str, 10).to_string().as_bytes()); }
        if let Some(str) = &self.listed_octane { hasher.update(str.to_string().as_bytes()); }
        if let Some(str) = &self.tune_octane_offset { hasher.update(str.to_string().as_bytes()); }
        if let Some(str) = &self.aspiration_setup { hasher.update(str.as_bytes()); }
        if let Some(str) = &self.aspiration_item_1 { hasher.update(str.as_bytes()); }
        if let Some(str) = &self.aspiration_item_2 { hasher.update(str.as_bytes()); }
        if let Some(str) = &self.aspiration_item_suboption_1 { hasher.update(str.as_bytes()); }
        if let Some(str) = &self.aspiration_item_suboption_2 { hasher.update(str.as_bytes()); }
        if let Some(str) = &self.aspiration_boost_control { hasher.update(str.as_bytes()); }
        if let Some(str) = &self.charger_size_1 { hasher.update(round_float_to(*str, 10).to_string().as_bytes()); }
        if let Some(str) = &self.charger_size_2 { hasher.update(round_float_to(*str, 10).to_string().as_bytes()); }
        if let Some(str) = &self.charger_tune_1 { hasher.update(round_float_to(*str, 10).to_string().as_bytes()); }
        if let Some(str) = &self.charger_tune_2 { hasher.update(round_float_to(*str, 10).to_string().as_bytes()); }
        if let Some(str) = &self.charger_max_boost_1 { hasher.update(round_float_to(*str, 10).to_string().as_bytes()); }
        if let Some(str) = &self.charger_max_boost_2 { hasher.update(round_float_to(*str, 10).to_string().as_bytes()); }
        if let Some(str) = &self.turbine_size_1 { hasher.update(round_float_to(*str, 10).to_string().as_bytes()); }
        if let Some(str) = &self.turbine_size_2 { hasher.update(round_float_to(*str, 10).to_string().as_bytes()); }
        hasher.finalize().iter().map(|b| *b).collect()
    }

    pub fn variant_data_checksum(&self) -> String {
        _sha256_data_to_string(self.variant_data_checksum_data())
    }

    pub fn result_data_checksum_data(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.adjusted_afr.to_string().as_bytes());
        hasher.update(&self.average_cruise_econ.to_string().as_bytes());
        hasher.update(&self.cooling_required.to_string().as_bytes());
        hasher.update(&self.econ.to_string().as_bytes());
        hasher.update(&self.econ_eff.to_string().as_bytes());
        hasher.update(&self.min_econ.to_string().as_bytes());
        hasher.update(&self.worst_econ.to_string().as_bytes());
        hasher.update(&self.emissions.to_string().as_bytes());
        hasher.update(&self.engineering_cost.to_string().as_bytes());
        hasher.update(&self.engineering_time.to_string().as_bytes());
        hasher.update(&self.idle.to_string().as_bytes());
        hasher.update(&self.idle_speed.to_string().as_bytes());
        hasher.update(&self.mttf.to_string().as_bytes());
        hasher.update(&self.man_hours.to_string().as_bytes());
        hasher.update(&self.material_cost.to_string().as_bytes());
        hasher.update(&self.noise.to_string().as_bytes());
        hasher.update(&self.peak_boost.to_string().as_bytes());
        hasher.update(&self.performance_index.to_string().as_bytes());
        hasher.update(&self.ron.to_string().as_bytes());
        if let Some(reliability) = &self.reliability_post_engineering {
            hasher.update(reliability.to_string().as_bytes());
        }
        hasher.update(&self.responsiveness.to_string().as_bytes());
        hasher.update(&self.service_cost.to_string().as_bytes());
        hasher.update(&self.smoothness.to_string().as_bytes());
        hasher.update(&self.service_cost.to_string().as_bytes());
        hasher.update(&self.tooling_costs.to_string().as_bytes());
        hasher.update(&self.total_cost.to_string().as_bytes());
        hasher.update(&self.weight.to_string().as_bytes());
        hasher.update(&self.peak_torque_rpm.to_string().as_bytes());
        hasher.update(&self.peak_torque.to_string().as_bytes());
        hasher.update(&self.peak_power.to_string().as_bytes());
        hasher.update(&self.peak_power_rpm.to_string().as_bytes());
        hasher.update(&self.max_rpm.to_string().as_bytes());
        if let Some(peak_boost_rpm) = &self.peak_boost_rpm {
            hasher.update(peak_boost_rpm.to_string().as_bytes());
        }
        hasher.finalize().iter().map(|b| *b).collect()
    }

    pub fn result_data_checksum(&self) -> String {
        _sha256_data_to_string(self.result_data_checksum_data())
    }

    fn decode_graph_data(row: &Row, graph_row_name: &str) -> rusqlite::Result<Vec<f64>> {
        let blob_packet = row.get_ref(graph_row_name)?.as_bytes()?;
        let data = &blob_packet[2..];
        let num_data_points = u64::from_le_bytes((&data[0..8]).try_into().expect("Failed to read number of data points")) as usize;
        let data = &data[8..];
        let mut out_vec: Vec<f64> = Vec::new();
        let mut cur_pos = 0;
        while out_vec.len() < num_data_points {
            cur_pos += 10;
            out_vec.push(f64::from_le_bytes((&data[cur_pos..cur_pos+8]).try_into().expect("Failed to decode graph point")));
            cur_pos += 8;
        }
        Ok(out_vec)
    }
}

impl Display for EngineV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.family_name, self.variant_name)
    }
}

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

fn _sha256_data_to_string(data: Vec<u8>) -> String {
    let mut hash = String::new();
    for byte in data {
        hash += &format!("{:X?}", byte);
    }
    hash
}

fn internal_days_to_year(days: i32) -> u16 {
    (1940 + (days / 360)) as u16
}

pub fn load_engines() -> HashMap<String, EngineV1> {
    let conn = Connection::open(get_db_path_4_2().unwrap()).unwrap();
    let mut stmt = conn.prepare(load_all_engines_query()).unwrap();
    let engs = stmt.query_map([], EngineV1::load_from_row).unwrap();
    let mut out_map = HashMap::new();
    for eng in engs {
        match eng {
            Ok(e) => { out_map.insert(String::from(&e.uuid), e); }
            Err(err) => { println!("Failed to load row: {}", err); }
        }
    }
    out_map
}

pub fn load_engine_by_uuid(uuid: &str, version: SandboxVersion) -> Result<Option<EngineV1>, String> {
    let db_path = version.get_path().ok_or_else(||{
        format!("No sandbox db file available for {}", version)
    })?;
    info!("Loading {} from {}", uuid, PathBuf::from(&db_path).display());
    let conn = Connection::open(&db_path).map_err(|e|{
        format!("Failed to connect to {}. {}", db_path.to_string_lossy(), e.to_string())
    })?;
    let mut stmt = conn.prepare(load_engine_by_uuid_query()).map_err(|e|{
        format!("Failed to prepare engine load by uuid statement. {}", e.to_string())
    })?;
    let engs = stmt.query_map(&[(":uid", uuid)], EngineV1::load_from_row).map_err(|e|{
        format!("Failed to query sandbox db for engine data. {}", e.to_string())
    })?;
    for row in engs {
        let eng = row.map_err(|err|{
            format!("Failed to read sandbox.db. {}", err.to_string())
        })?;
        return Ok(Some(eng));
    }
    Ok(None)
}

fn load_all_engines_query() -> &'static str {
    r#"select f.GameVersion as f_version, v.GameVersion as v_version, f.uuid as f.uuid, f.name as f_name, f.InternalDays as f_days, f.Bore as MaxBore, f.Stroke as MaxStroke, f.*,
    v.uid as v_uuid, v.name as v_name, v.InternalDays as v_days, v.Bore as VBore, v.Stroke as VStroke, v.*,
    r.*,
    c.*
    from "Variants" as v
    join "Families" as f on v.FUID = f.UID
    join "EngineResults" as r using(uid)
    join "EngineCurves" as c using(uid) ;"#
}

fn load_engine_by_uuid_query() -> &'static str {
    r#"select f.GameVersion as f_version, v.GameVersion as v_version, f.uid as f_uuid, f.name as f_name, f.InternalDays as f_days, f.Bore as MaxBore, f.Stroke as MaxStroke, f.*,
    v.uid as v_uuid, v.name as v_name, v.InternalDays as v_days, v.Bore as VBore, v.Stroke as VStroke, v.*,
    r.*,
    c.*
    from "Variants" as v
    join "Families" as f on v.FUID = f.UID
    join "EngineResults" as r using(uid)
    join "EngineCurves" as c using(uid)
    where v_uuid = :uid;"#
}

#[cfg(target_os = "windows")]
pub fn get_db_path() -> Option<OsString> {
    let sandbox_path = UserDirs::new()?.document_dir()?.join(PathBuf::from_iter(legacy_sandbox_path()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

#[cfg(target_os = "windows")]
pub fn get_db_path_4_2() -> Option<OsString> {
    let sandbox_path = BaseDirs::new()?.cache_dir().join(PathBuf::from_iter(sandbox_dir_4_2()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

#[cfg(target_os = "windows")]
pub fn get_db_path_ellisbury() -> Option<OsString> {
    let sandbox_path = BaseDirs::new()?.cache_dir().join(PathBuf::from_iter(sandbox_dir_ellisbury()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

#[cfg(target_os = "linux")]
pub fn get_db_path() -> Option<OsString> {
    let sandbox_path = steam::get_wine_documents_dir(STEAM_GAME_ID).join(PathBuf::from_iter(legacy_sandbox_path()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

#[cfg(target_os = "linux")]
pub fn get_db_path_4_2() -> Option<OsString> {
    let sandbox_path = steam::get_wine_appdata_local_dir(STEAM_GAME_ID).join(PathBuf::from_iter(sandbox_dir_4_2()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

#[cfg(target_os = "linux")]
pub fn get_db_path_ellisbury() -> Option<OsString> {
    let sandbox_path = steam::get_wine_appdata_local_dir(STEAM_GAME_ID).join(PathBuf::from_iter(sandbox_dir_ellisbury()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}


pub fn get_engine_names() -> Option<Vec<String>>
{
    let conn = Connection::open(get_db_path()?).unwrap();
    let mut stmt = conn.prepare("select f.name || \" - \" || v.name as \"Full Name\" \
                                          from \"Variants\" as v inner join \"Families\" as f \
                                          on v.FUID = f.UID;").unwrap();
    let mut rows = stmt.query([]).unwrap();
    let mut ret_vec: Vec<String> = Vec::new();
    loop {
        match rows.next() {
            Ok(option) => {
                match option {
                    None => break,
                    Some(row) => {
                        let value: String = row.get("Full Name").unwrap();
                        ret_vec.push(value);
                    }
                }
            }
            Err(_) => {
                return None
            }
        }
    }
    Some(ret_vec)
}

fn legacy_sandbox_path() -> Vec<&'static str> {
    vec!["My Games", "Automation", "Sandbox_openbeta.db"]
}

fn sandbox_dir_4_2() -> Vec<&'static str> {
    vec!["AutomationGame", "Saved", "UserData", "Sandbox_211122.db"]
}

fn sandbox_dir_ellisbury() -> Vec<&'static str> {
    vec!["AutomationGame", "Saved", "UserData", "Sandbox_230915.db"]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::sandbox::{get_db_path, get_db_path_4_2, SandboxVersion};

    #[test]
    fn get_sandbox_db_path() -> Result<(), String> {
        let path = PathBuf::from(get_db_path_4_2().unwrap());
        println!("Sandbox path is {}", path.display());
        let engine_data = crate::sandbox::load_engine_by_uuid("7F98B2EA4A9E928278F355860DF3B4DF", SandboxVersion::FourDotTwo)?;
        if let Some(engine) = engine_data {
            println!("Econ data {:?}", engine.econ_eff_curve);
        }
        Ok(())
    }

    #[test]
    fn get_legacy_sandbox_db_path() -> Result<(), String> {
        let path = PathBuf::from(get_db_path().unwrap());
        println!("Sandbox path is {}", path.display());
        Ok(())
    }
}
