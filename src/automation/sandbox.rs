use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use directories::{BaseDirs, UserDirs};
use rusqlite::{Connection, params, Row};

use crate::steam;
use crate::automation::{STEAM_GAME_ID};

#[derive(Debug)]
pub struct EngineV1 {
    pub uuid: String,
    pub family_name: String,
    pub variant_name: String,
    pub family_game_days: i32,
    pub variant_game_days: i32,
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
    pub fuel_type: String,
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
    pub afr: f64,
    pub afr_lean: f64,
    pub rpm_limit: f64,
    pub ignition_timing_setting: f64,
    pub exhaust_diameter: f64,
    pub quality_bottom_end: i32,
    pub quality_top_end: i32,
    pub quality_aspiration: i32,
    pub quality_fuel_system: i32,
    pub quality_exhaust: i32,
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
    pub peak_boost_rpm: f64,
    pub performance_index: f64,
    pub ron: f64,
    pub reliability_post_engineering: f64,
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

pub fn load_engine_by_uuid(uuid: &str) -> Result<Option<EngineV1>, String> {
    let conn = Connection::open(get_db_path().unwrap()).unwrap();
    let mut stmt = conn.prepare(load_engine_by_uuid_query()).unwrap();
    let engs = stmt.query_map(&[(":uid", uuid)],
                                                         EngineV1::load_from_row).unwrap();
    for row in engs {
        let eng = row.map_err(|err|{
            format!("Failed to read sandbox.db. {}", err.to_string())
        })?;
        return Ok(Some(eng));
    }
    Ok(None)
}

fn load_all_engines_query() -> &'static str {
    r#"select f.name as f_name, f.InternalDays as f_days, f.Bore as MaxBore, f.Stroke as MaxStroke, f.*,
    v.uid as v_uuid, v.name as v_name, v.InternalDays as v_days, v.Bore as VBore, f.Stroke as VStroke, v.*,
    r.*,
    c.*
    from "Variants" as v
    join "Families" as f on v.FUID = f.UID
    join "EngineResults" as r using(uid)
    join "EngineCurves" as c using(uid) ;"#
}

fn load_engine_by_uuid_query() -> &'static str {
    r#"select f.name as f_name, f.InternalDays as f_days, f.Bore as MaxBore, f.Stroke as MaxStroke, f.*,
    v.uid as v_uuid, v.name as v_name, v.InternalDays as v_days, v.Bore as VBore, f.Stroke as VStroke, v.*,
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

mod tests {
    use std::path::PathBuf;
    use crate::automation::sandbox::{get_db_path, get_db_path_4_2};

    #[test]
    fn get_sandbox_db_path() -> Result<(), String> {
        let path = PathBuf::from(get_db_path_4_2().unwrap());
        println!("Sandbox path is {}", path.display());
        Ok(())
    }

    #[test]
    fn get_legacy_sandbox_db_path() -> Result<(), String> {
        let path = PathBuf::from(get_db_path().unwrap());
        println!("Sandbox path is {}", path.display());
        Ok(())
    }
}
