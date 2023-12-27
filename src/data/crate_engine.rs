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
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Write};
use std::mem;
use std::path::{Path, PathBuf};
use bincode::{deserialize, deserialize_from, serialize_into};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use automation::sandbox::{BlockConfig, EngineV1, HeadConfig, SandboxVersion};
use beam_ng::jbeam;
use crate::data::{AutomationSandboxCrossChecker};

pub(crate) const CRATE_ENGINE_FILE_SUFFIX: &'static str = "eng";

type CurrentMetadataType = MetadataV1;
type CurrentDataType = DataV1;


pub struct CrateEngine {
    metadata: CrateEngineMetadata,
    data: CrateEngineData
}

impl CrateEngine {
    pub fn from_beamng_mod_zip(mod_path: &Path, options: CreationOptions) -> Result<CrateEngine, String> {
        let data = DataV1::from_beamng_mod_zip(mod_path, options)?;
        let engine_data = data.jbeam_file_data.get(&data.main_engine_jbeam_filename).unwrap();
        let name =
            _get_name_from_jbeam_data(engine_data).unwrap_or_else(
                || {
                    let m = mod_path.file_name().unwrap_or("unknown".as_ref()).to_str().unwrap_or("unknown");
                    m.strip_suffix(".zip").unwrap_or(m).to_string()
                });

        let mut auto_hasher = Sha256::new();
        auto_hasher.update(&data.automation_variant_data.family_data_checksum_data());
        auto_hasher.update(&data.automation_variant_data.variant_data_checksum_data());
        auto_hasher.update(&data.automation_variant_data.result_data_checksum_data());
        let automation_data_hash = create_sha256_hash_array(auto_hasher);
        if automation_data_hash.is_none() {
            warn!("Failed to calculate automation data hash");
        }

        let mut jbeam_hasher = Sha256::new();
        jbeam_hasher.update(engine_data);
        let engine_jbeam_hash = create_sha256_hash_array(jbeam_hasher);
        if automation_data_hash.is_none() {
            warn!("Failed to calculate engine jbeam data hash");
        }

        let aspiration = data.automation_variant_data.aspiration.clone();
        let fuel = match data.automation_variant_data.fuel_type.as_ref() {
            None => "Unknown".to_string(),
            Some(f) => f.clone()
        };
        let metadata = CurrentMetadataType {
            data_version: CurrentDataType::VERSION.as_u16(),
            automation_version: data.automation_variant_data.variant_version,
            name,
            automation_data_hash,
            engine_jbeam_hash,
            build_year: data.automation_variant_data.get_variant_build_year(),
            block_config: data.automation_variant_data.get_block_config(),
            head_config: data.automation_variant_data.get_head_config(),
            capacity: data.automation_variant_data.get_capacity_cc(),
            aspiration,
            fuel,
            peak_power: data.automation_variant_data.peak_power.round() as u32,
            peak_power_rpm: data.automation_variant_data.peak_power_rpm.round() as u32,
            peak_torque: data.automation_variant_data.peak_torque.round() as u32,
            peak_torque_rpm: data.automation_variant_data.peak_torque_rpm.round() as u32,
            max_rpm: data.automation_variant_data.max_rpm.round() as u32
        };

        Ok(CrateEngine{
            metadata: CrateEngineMetadata::from_current_version(metadata),
            data: CrateEngineData::from_current_version(data)
        })
    }

    pub fn deserialize_from(reader: &mut impl Read) -> Result<CrateEngine, String> {
        let metadata = CrateEngineMetadata::from_reader(reader)?;
        let data = CrateEngineData::from_reader(&metadata, reader)?;
        Ok(CrateEngine { metadata, data })
    }

    pub fn serialize_to(&self, writer: &mut impl Write) -> bincode::Result<()> {
        self.metadata.serialize_into(writer)?;
        self.data.serialize_into(writer)
    }

    pub fn name(&self) -> &str {
        self.metadata.name()
    }

    pub fn version(&self) -> DataVersion {
        self.metadata.data_version().unwrap()
    }

    pub fn get_automation_car_file_data(&self) -> &Vec<u8> {
        self.data.get_automation_car_file_data()
    }

    pub fn get_automation_engine_data(&self) -> &EngineV1 {
        self.data.get_automation_engine_data()
    }

    pub fn get_engine_jbeam_data(&self) -> Option<&Vec<u8>> {
        self.data.get_engine_jbeam_data()
    }
}

fn create_sha256_hash_array(hasher: impl Digest) -> Option<[u8; 32]> {
    let hash: Vec<u8> = hasher.finalize().iter().map(|b| *b).collect();
    match <[u8; 32]>::try_from(hash) {
        Ok(hash_array) => Some(hash_array),
        Err(_) => {
            None
        }
    }
}

pub enum CrateEngineMetadata {
    MetadataV1(MetadataV1)
}

impl CrateEngineMetadata {
    fn from_current_version(inner_type: CurrentMetadataType) -> CrateEngineMetadata {
        return CrateEngineMetadata::MetadataV1(inner_type)
    }

    pub fn from_reader(reader: &mut impl Read) -> Result<CrateEngineMetadata, String> {
        let mut buf = [0u8; mem::size_of::<u16>()];
        reader.read_exact(&mut buf).map_err(|e| format!("Failed to read metadata. {}", e.to_string()))?;
        let metadata_version = u16::from_le_bytes(buf);
        match metadata_version {
            MetadataV1::VERSION_U16 => {
                let internal_type: MetadataV1 =
                    deserialize_from(reader)
                        .map_err(|e| format!("Failed to deserialize metadata. {}", e.to_string()))?;
                Ok(CrateEngineMetadata::MetadataV1(internal_type))
            },
            _ => Err(format!("Unknown metadata version {}", metadata_version))
        }
    }

    pub fn get_metadata_version_u16(&self) -> u16 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => {
                m.get_version_u16()
            }
        }
    }

    pub fn serialize_into(&self, writer: &mut impl Write) -> bincode::Result<()> {
        writer.write(&self.get_metadata_version_u16().to_le_bytes())?;
        match self {
            CrateEngineMetadata::MetadataV1(m) => {
                serialize_into(writer, &m)
            }
        }
    }

    pub fn name(&self) -> &str {
        match self {
            CrateEngineMetadata::MetadataV1(d) => { &d.name }
        }
    }

    pub fn data_version(&self) -> Result<DataVersion, String> {
        let val = match self {
            CrateEngineMetadata::MetadataV1(d) => { &d.data_version }
        };
        DataVersion::from_u16(*val)
    }

    pub fn automation_version(&self) -> u64 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => { m.automation_version }
        }
    }

    pub fn build_year(&self) -> u16 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.build_year
        }
    }

    pub fn block_config(&self) -> &BlockConfig {
        match self {
            CrateEngineMetadata::MetadataV1(m) => &m.block_config
        }
    }

    pub fn head_config(&self) -> &HeadConfig {
        match self {
            CrateEngineMetadata::MetadataV1(m) => &m.head_config
        }
    }

    pub fn capacity(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.capacity
        }
    }

    pub fn aspiration(&self) -> &str {
        match self {
            CrateEngineMetadata::MetadataV1(m) => &m.aspiration
        }
    }

    pub fn fuel(&self) -> &str {
        match self {
            CrateEngineMetadata::MetadataV1(m) => &m.fuel
        }
    }

    pub fn peak_power(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.peak_power
        }
    }

    pub fn peak_power_rpm(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.peak_power_rpm
        }
    }

    pub fn peak_torque(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.peak_torque
        }
    }

    pub fn peak_torque_rpm(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.peak_torque_rpm
        }
    }

    pub fn max_rpm(&self) -> u32 {
        match self {
            CrateEngineMetadata::MetadataV1(m) => m.max_rpm
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataV1 {
    data_version: u16,
    automation_version: u64,
    name: String,
    engine_jbeam_hash: Option<[u8; 32]>,
    automation_data_hash: Option<[u8; 32]>,
    build_year: u16,
    block_config: BlockConfig,
    head_config: HeadConfig,
    capacity: u32,
    aspiration: String,
    fuel: String,
    peak_power: u32,
    peak_power_rpm: u32,
    peak_torque: u32,
    peak_torque_rpm: u32,
    max_rpm: u32
}

impl MetadataV1 {
    const VERSION_U16: u16 = 1_u16;
    pub fn get_version_u16(&self) -> u16 {
        MetadataV1::VERSION_U16
    }
}


#[derive(Debug, Deserialize, Serialize)]
pub enum DataVersion {
    V1
}

impl DataVersion {
    pub const VERSION_1_STRING: &'static str = "v1";

    pub fn from_u16(val: u16) -> Result<DataVersion, String> {
        match val {
            1 => Ok(DataVersion::V1),
            _ => Err(format!("Unknown data version {}", val))
        }
    }

    pub fn as_u16(&self) -> u16 {
        match self {
            DataVersion::V1 => 1
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DataVersion::V1 => DataVersion::VERSION_1_STRING
        }
    }
}

impl Display for DataVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.as_str())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreationOptions {
    pub xref_mod_with_sandbox: bool
}

impl CreationOptions {
    pub fn default() -> CreationOptions {
        CreationOptions { xref_mod_with_sandbox: true }
    }
}

pub enum CrateEngineData {
    DataV1(DataV1)
}

impl CrateEngineData {
    fn from_current_version(inner_type: CurrentDataType) -> CrateEngineData {
        return CrateEngineData::DataV1(inner_type)
    }

    pub fn from_reader(metadata: &CrateEngineMetadata, reader: &mut impl Read) -> Result<CrateEngineData, String> {
        match metadata.data_version()? {
            DataVersion::V1 => {
                let internal_data =
                    deserialize_from(reader).map_err(|e| {
                        format!("Failed to deserialise {} crate engine. {}", DataVersion::V1, e.to_string())
                    })?;
                Ok(CrateEngineData::DataV1(internal_data))
            }
        }
    }

    pub fn serialize_into(&self, writer: &mut impl Write) -> bincode::Result<()> {
        match self {
            CrateEngineData::DataV1(d) => {
                serialize_into(writer, d)
            }
        }
    }

    pub fn get_automation_car_file_data(&self) -> &Vec<u8> {
        match self {
            CrateEngineData::DataV1(d) => {
                &d.car_file_data
            }
        }
    }

    pub fn get_automation_engine_data(&self) -> &EngineV1 {
        match self {
            CrateEngineData::DataV1(d) => {
                &d.automation_variant_data
            }
        }
    }

    pub fn get_engine_jbeam_data(&self) -> Option<&Vec<u8>> {
        match self {
            CrateEngineData::DataV1(d) => {
                d.jbeam_file_data.get(&d.main_engine_jbeam_filename)
            }
        }
    }
}


#[derive(Debug, Deserialize, Serialize)]
pub struct DataV1 {
    mod_info_json_data: Option<Vec<u8>>,
    main_engine_jbeam_filename: String,
    jbeam_file_data: HashMap<String, Vec<u8>>,
    car_file_data: Vec<u8>,
    automation_variant_data: EngineV1,
    license_data: Option<Vec<u8>>
}

impl DataV1 {
    pub const VERSION: DataVersion = DataVersion::V1;

    pub fn version(&self) -> DataVersion {
        Self::VERSION
    }

    pub fn from_beamng_mod_zip(mod_path: &Path, options: CreationOptions) -> Result<DataV1, String> {
        let mut mod_data = beam_ng::ModData::from_path(mod_path)?;
        let car_file_data = match mod_data.get_automation_car_file_data() {
            None => return Err("Failed to load .car file from mod. File is missing".to_string()),
            Some(data) => data.clone()
        };
        let automation_car_file = automation::car::CarFile::from_bytes(car_file_data.clone())?;
        let uid = _get_engine_uuid_from_car_file(&automation_car_file)?;
        if uid.len() < 5 {
            return Err(format!("Invalid engine uuid found {}", uid));
        }
        info!("Engine uuid: {}", &uid);
        let expected_engine_data_filename = format!("camso_engine_{}.jbeam", &uid[0..5]);
        info!("Expect to find engine data in {}", &expected_engine_data_filename);

        let mut main_engine_data_file = match mod_data.contains_jbeam_file(&expected_engine_data_filename) {
            false => {
                let legacy_engine_filename = "camso_engine.jbeam".to_string();
                match mod_data.contains_jbeam_file(&legacy_engine_filename) {
                    true => {
                        Some(legacy_engine_filename)
                    },
                    false => {
                        None
                    }
                }
            }
            true => {
                info!("Found main engine data file: {}", &expected_engine_data_filename);
                Some(expected_engine_data_filename)
            }
        };

        if main_engine_data_file.is_none() {
            for name in mod_data.jbeam_filenames() {
                if name.contains("camso_engine_")
                {
                    if !name.contains("structure") &&
                        !name.contains("internals") &&
                        !name.contains("balancing")
                    {
                        main_engine_data_file = Some(name.clone());
                        break;
                    }
                }
            }
        }

        let main_engine_jbeam_filename =
            main_engine_data_file.ok_or("Failed to find the main engine data".to_string())?;
        info!("Found main engine data file: {}", main_engine_jbeam_filename);

        let version = _get_engine_version_from_car_file(&automation_car_file)?;
        info!("Engine version number: {}", version);
        let version = SandboxVersion::from_version_number(version);
        info!("Deduced as {}", version);
        let automation_variant_data = match automation::sandbox::load_engine_by_uuid(&uid, version)? {
            None => {
                return Err(format!("No engine found with uuid {}", uid));
            }
            Some(eng) => { eng }
        };

        if options.xref_mod_with_sandbox {
            AutomationSandboxCrossChecker::new(&automation_car_file, &automation_variant_data).validate().map_err(|err|{
                format!("{}. The BeamNG mod may be out-of-date; try recreating a mod with the latest engine version", err)
            })?;
        }

        let mut mod_info_json_data = match mod_data.get_info_json() {
            Ok(data_str) => {
                Some(data_str.into_bytes())
            }
            Err(err_str) => {
                warn!("Couldn't read info.json from {}. {}", mod_path.display(), &err_str);
                None
            }
        };

        Ok(DataV1 {
            mod_info_json_data,
            main_engine_jbeam_filename,
            jbeam_file_data: mod_data.take_jbeam_file_data(),
            car_file_data,
            automation_variant_data,
            license_data: mod_data.take_license_data()
        })
    }

    pub fn from_eng_file(file_path: &Path) -> bincode::Result<DataV1> {
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        deserialize(&buffer)
    }

    pub fn write_to_file(&self, file: &mut File) -> bincode::Result<()> {
        serialize_into(file, &self)
    }
}

fn _is_legacy_main_engine_data_file(filename: &str) -> bool {
    if filename.ends_with("camso_engine.jbeam") {
        return true;
    }
    if filename.contains("camso_engine_")
    {
        if !filename.contains("structure") &&
            !filename.contains("internals") &&
            !filename.contains("balancing")
        {
            return true;
        }
    }
    false
}

fn _get_variant_section_from_car_file(automation_car_file: &automation::car::CarFile)
                                      -> Result<&automation::car::Section, String>
{
    Ok(automation_car_file
        .get_section("Car").ok_or("Failed to find Car section in .car file".to_string())?
        .get_section("Variant").ok_or("Failed to find Car.Variant section in .car file".to_string())?)
}

fn _get_engine_uuid_from_car_file(automation_car_file: &automation::car::CarFile) -> Result<String, String> {
    let variant_info = _get_variant_section_from_car_file(automation_car_file)?;
    let uid =
        variant_info.get_attribute("UID").ok_or("No UID in Car.Variant section".to_string())?.value.as_str();
    Ok(uid.to_string())
}

fn _get_engine_version_from_car_file(automation_car_file: &automation::car::CarFile) -> Result<u64, String> {
    let variant_info = _get_variant_section_from_car_file(automation_car_file)?;
    let version_num = variant_info.get_attribute("GameVersion").unwrap().value.as_num().unwrap();
    Ok(version_num as u64)
}

fn _get_name_from_jbeam_data(engine_data: &Vec<u8>) -> Option<String> {
    let data_map = match jbeam::from_slice(&*engine_data) {
        Ok(d) => d,
        Err(_) => { return None; }
    };

    let mut engine_key = String::from("Camso_Engine");
    let test_key = String::from(engine_key.clone() + "_");
    for key in data_map.keys() {
        if key.starts_with(&test_key) {
            engine_key = String::from(key);
            break;
        }
    }
    let eng_info = data_map.get(&engine_key)?.as_object()?.get("information")?.as_object()?;
    Some(eng_info.get("name")?.as_str()?.to_string())
}

#[test]
fn create_crate_engine() -> Result<(), String> {
    let path = PathBuf::from("C:/Users/zephy/AppData/Local/BeamNG.drive/mods/dawnv6.zip");
    let eng = CrateEngine::from_beamng_mod_zip(&path, CreationOptions::default())?;
    println!("Loaded {} from mod", eng.name());
    let crate_path = PathBuf::from(format!("{}.eng", eng.name()));
    {
        let mut file = File::create(format!("{}.eng", eng.name())).expect("Failed to open file");
        match eng.serialize_to(&mut file) {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(e.to_string())
            }
        }?
    }

    {
        let calc = crate::fabricator::AcEngineParameterCalculatorV1::from_crate_engine(&crate_path)?;
        println!("Limiter {}", calc.limiter());
        println!("Torque {}", calc.peak_torque());
        println!("BHP {}", calc.peak_bhp());
    }
    Ok(())
}
