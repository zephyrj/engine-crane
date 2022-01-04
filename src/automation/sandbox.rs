use std::ffi::OsString;
use std::path::PathBuf;
use directories::UserDirs;
use rusqlite::{Connection, Result, Row};

use crate::steam;
use crate::automation::{STEAM_GAME_ID};

struct EngineV1 {
    family_name: String,
    variant_name: String,
    family_game_days: i64,
    variant_game_days: i64,
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
    aspiration_option: String,
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
    rpm_limit: u32,
    ignition_timing_setting: f64,
    ar_ratio: f64,
    boost_cut_off: f64,
    compressor_fraction: f64,
    turbine_fraction: f64,
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
    idle: u32,
    idle_speed: f64,
    mttf: f64,
    man_hours: f64,
    material_cost: f64,
    noise: f64,
    peak_boost: f64,
    peak_boost_rpm: u32,
    performance_index: f64,
    ron: f64,
    reliability_post_engineering: f64,
    responsiveness: f64,
    service_cost: f64,
    smoothness: f64,
    tooling_costs: f64,
    total_cost: f64,
    weight: f64,
    peak_torque_rpm: u32,
    peak_torque: f64,
    peak_power: f64,
    peak_power_rpm: u32,
    max_rpm: u32,
    rpm_curve: Vec<f64>,
    power_curve: Vec<f64>,
    torque_curve: Vec<f64>,
    boost_curve: Vec<f64>,
    econ_curve: Vec<f64>,
    econ_eff_curve: Vec<f64>
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
                        let value: String = row.get(0).unwrap();
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