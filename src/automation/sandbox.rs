use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use directories::UserDirs;
use rusqlite::{Connection, Result, Row};
use rusqlite::types::ValueRef;

use crate::steam;
use crate::automation::{STEAM_GAME_ID};

#[derive(Debug)]
pub struct EngineV1 {
    uuid: String,
    family_name: String,
    variant_name: String,
    family_game_days: i32,
    variant_game_days: i32,
    block_config: String,
    block_material: String,
    block_type: String,
    head_type: String,
    head_material: String,
    valves: String,
    vvl: String,
    max_bore: f64,
    max_stroke: f64,
    crank: String,
    conrods: String,
    pistons: String,
    vvt: String,
    aspiration: String,
    intercooler_setting: f64,
    fuel_system_type: String,
    fuel_system: String,
    intake_manifold: String,
    intake: String,
    fuel_type: String,
    headers: String,
    exhaust_count: String,
    exhaust_bypass_valves: String,
    cat: String,
    muffler_1: String,
    muffler_2: String,
    bore: f64,
    stroke: f64,
    capacity: f64,
    compression: f64,
    cam_profile_setting: f64,
    vvl_cam_profile_setting: f64,
    afr: f64,
    afr_lean: f64,
    rpm_limit: f64,
    ignition_timing_setting: f64,
    exhaust_diameter: f64,
    quality_bottom_end: i32,
    quality_top_end: i32,
    quality_aspiration: i32,
    quality_fuel_system: i32,
    quality_exhaust: i32,
    adjusted_afr: f64,
    average_cruise_econ: f64,
    cooling_required: f64,
    econ: f64,
    econ_eff: f64,
    min_econ: f64,
    worst_econ: f64,
    emissions: f64,
    engineering_cost: f64,
    engineering_time: f64,
    idle: f64,
    idle_speed: f64,
    mttf: f64,
    man_hours: f64,
    material_cost: f64,
    noise: f64,
    peak_boost: f64,
    peak_boost_rpm: f64,
    performance_index: f64,
    ron: f64,
    reliability_post_engineering: f64,
    responsiveness: f64,
    service_cost: f64,
    smoothness: f64,
    tooling_costs: f64,
    total_cost: f64,
    weight: f64,
    peak_torque_rpm: f64,
    peak_torque: f64,
    peak_power: f64,
    peak_power_rpm: f64,
    max_rpm: f64,
    rpm_curve: Vec<f64>,
    power_curve: Vec<f64>,
    torque_curve: Vec<f64>,
    boost_curve: Vec<f64>,
    econ_curve: Vec<f64>,
    econ_eff_curve: Vec<f64>
}

impl EngineV1 {
    pub fn load_from_row(row: &Row) -> rusqlite::Result<EngineV1> {
        Ok(EngineV1 {
            uuid: row.get("v_uuid")?,
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
            crank: row.get("Crank")?,
            conrods: row.get("Conrods")?,
            pistons: row.get("Pistons")?,
            vvt: row.get("VVT")?,
            aspiration: row.get("AspirationType")?,
            intercooler_setting: row.get("IntercoolerSetting")?,
            fuel_system_type: row.get("FuelSystemType")?,
            fuel_system: row.get("FuelSystem")?,
            intake_manifold: row.get("IntakeManifold")?,
            intake: row.get("Intake")?,
            fuel_type: row.get("Crank")?,  // TODO
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
            peak_boost_rpm: 0.0, // TODO row.get("PeakBoostRPM")? - handle Null,
            performance_index: row.get("PerformanceIndex")?,
            ron: row.get("RON")?,
            reliability_post_engineering: row.get("ReliabilityPostEngineering")?,
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

impl std::fmt::Display for EngineV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.family_name, self.variant_name)
    }
}

pub fn load_engines() -> HashMap<String, EngineV1> {
    let conn = Connection::open(get_db_path_4_2().unwrap()).unwrap();
    let mut stmt = conn.prepare(get_sql_load_query()).unwrap();
    let mut engs = stmt.query_map([], EngineV1::load_from_row).unwrap();
    let mut out_map = HashMap::new();
    for eng in engs {
        match eng {
            Ok(e) => { out_map.insert(String::from(&e.uuid), e); }
            Err(err) => { println!("Failed to load row: {}", err); }
        }
    }
    out_map
}

fn get_sql_load_query() -> &'static str {
    r#"select f.name as f_name, f.InternalDays as f_days, f.Bore as MaxBore, f.Stroke as MaxStroke, f.*,
    v.uid as v_uuid, v.name as v_name, v.InternalDays as v_days, v.Bore as VBore, f.Stroke as VStroke, v.*,
    r.*,
    c.*
    from "Variants" as v
    join "Families" as f on v.FUID = f.UID
    join "EngineResults" as r using(uid)
    join "EngineCurves" as c using(uid) ;"#
}


#[cfg(target_os = "windows")]
pub fn get_db_path() -> Option<OsString> {
    let sandbox_path = UserDirs::new()?.document_dir()?.join(PathBuf::from_iter(legacy_sandbox_path()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

#[cfg(target_os = "linux")]
pub fn get_db_path() -> Option<OsString> {
    let sandbox_path = steam::get_wine_documents_dir(STEAM_GAME_ID)?.join(PathBuf::from_iter(legacy_sandbox_path()));
    match sandbox_path.is_file() {
        true => Some(sandbox_path.into_os_string()),
        false => None
    }
}

#[cfg(target_os = "linux")]
pub fn get_db_path_4_2() -> Option<OsString> {
    let sandbox_path = steam::get_wine_appdata_local_dir(STEAM_GAME_ID)?.join(PathBuf::from_iter(sandbox_dir_4_2()));
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