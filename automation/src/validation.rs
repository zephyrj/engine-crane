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

use tracing::info;
use crate::car::{CarFile, Section};
use crate::sandbox::EngineV1;
use utils::numeric::round_float_to;

pub struct AutomationSandboxCrossChecker<'a, 'b> {
    car_file: &'a CarFile,
    sandbox_data: &'b EngineV1,
    float_precision: u32
}

impl<'a, 'b> AutomationSandboxCrossChecker<'a, 'b> {
    pub fn new(car_file: &'a CarFile,
               sandbox_data: &'b EngineV1) -> AutomationSandboxCrossChecker<'a, 'b> {
        AutomationSandboxCrossChecker {car_file, sandbox_data, float_precision: 10}
    }

    pub fn validate(&self) -> Result<(), String> {
        let version = self.car_file.get_section("Car").unwrap().get_attribute("Version");
        if let Some(attribute) = version {
            if (attribute.value.as_num()?.round() as u64) < 2200000000 {
                self.validate_legacy_family()?;
                self.validate_legacy_variant()?;
                return Ok(());
            }
        }
        self.validate_family()?;
        self.validate_variant()?;
        Ok(())
    }

    fn validate_legacy_family(&self) -> Result<(), String> {
        let family_data = self.car_file.get_section("Car").unwrap().get_section("Family").unwrap();
        self.validate_u64( *&self.sandbox_data.family_version, family_data, "GameVersion")?;
        self.validate_strings(&self.sandbox_data.family_uuid, family_data,"UID")?;
        self.validate_strings(&self.sandbox_data.family_name, family_data,"Name")?;
        self.validate_i32(*&self.sandbox_data.family_game_days, family_data,"InternalDays")?;
        self.validate_strings(&self.sandbox_data.block_config, family_data,"BlockConfig")?;
        self.validate_strings(&self.sandbox_data.block_material, family_data,"BlockMaterial")?;
        self.validate_strings(&self.sandbox_data.block_type, family_data,"BlockType")?;
        self.validate_strings(&self.sandbox_data.head_type, family_data,"Head")?;
        self.validate_strings(&self.sandbox_data.head_material, family_data,"HeadMaterial")?;
        self.validate_strings(&self.sandbox_data.vvl, family_data,"VVL")?;
        self.validate_strings(&self.sandbox_data.valves, family_data,"Valves")?;
        self.validate_floats(*&self.sandbox_data.max_stroke, family_data,"Stroke")?;
        self.validate_floats(*&self.sandbox_data.max_bore, family_data,"Bore")?;
        Ok(())
    }

    fn validate_family(&self) -> Result<(), String> {
        let family_data = self.car_file.get_section("Car").unwrap().get_section("Family").unwrap();
        self.validate_u64( *&self.sandbox_data.family_version, family_data, "GameVersion")?;
        self.validate_strings(&self.sandbox_data.family_uuid, family_data,"UID")?;
        self.validate_strings(&self.sandbox_data.family_name, family_data,"Name")?;
        self.validate_i32(*&self.sandbox_data.family_game_days, family_data,"InternalDays")?;
        self.validate_i32(*&self.sandbox_data.family_quality, family_data,"QualityFamily")?;
        self.validate_strings(&self.sandbox_data.block_config, family_data,"BlockConfig")?;
        self.validate_strings(&self.sandbox_data.block_material, family_data,"BlockMaterial")?;
        self.validate_strings(&self.sandbox_data.block_type, family_data,"BlockType")?;
        self.validate_strings(&self.sandbox_data.head_type, family_data,"Head")?;
        self.validate_strings(&self.sandbox_data.head_material, family_data,"HeadMaterial")?;
        self.validate_strings(&self.sandbox_data.valves, family_data,"Valves")?;
        self.validate_floats(*&self.sandbox_data.max_stroke, family_data,"Stroke")?;
        self.validate_floats(*&self.sandbox_data.max_bore, family_data,"Bore")?;
        Ok(())
    }

    fn validate_legacy_variant(&self) -> Result<(), String> {
        let variant_data = self.car_file.get_section("Car").unwrap().get_section("Variant").unwrap();
        self.validate_u64( *&self.sandbox_data.variant_version, variant_data, "GameVersion")?;
        self.validate_strings(&self.sandbox_data.family_uuid, variant_data,"FUID")?;
        self.validate_strings(&self.sandbox_data.uuid, variant_data,"UID")?;
        self.validate_strings(&self.sandbox_data.variant_name, variant_data,"Name")?;
        self.validate_i32(*&self.sandbox_data.variant_game_days, variant_data,"InternalDays")?;
        self.validate_strings(&self.sandbox_data.crank, variant_data,"Crank")?;
        self.validate_strings(&self.sandbox_data.conrods, variant_data,"Conrods")?;
        self.validate_strings(&self.sandbox_data.pistons, variant_data,"Pistons")?;
        self.validate_strings(&self.sandbox_data.vvt, variant_data,"VVT")?;
        self.validate_strings(&self.sandbox_data.aspiration, variant_data,"AspirationType")?;
        self.validate_floats(*&self.sandbox_data.intercooler_setting, variant_data,"IntercoolerSetting")?;
        self.validate_strings(&self.sandbox_data.fuel_system_type, variant_data,"FuelSystemType")?;
        self.validate_strings(&self.sandbox_data.fuel_system, variant_data,"FuelSystem")?;
        if let Some(fuel_type) = &self.sandbox_data.fuel_type { self.validate_strings(fuel_type, variant_data,"FuelType")?; }
        self.validate_strings(&self.sandbox_data.intake_manifold, variant_data,"IntakeManifold")?;
        self.validate_strings(&self.sandbox_data.intake, variant_data,"Intake")?;
        self.validate_strings(&self.sandbox_data.headers, variant_data,"Headers")?;
        self.validate_strings(&self.sandbox_data.exhaust_count, variant_data,"ExhaustCount")?;
        self.validate_strings(&self.sandbox_data.exhaust_bypass_valves, variant_data,"ExhaustBypassValves")?;
        self.validate_strings(&self.sandbox_data.cat, variant_data,"Cat")?;
        self.validate_strings(&self.sandbox_data.muffler_1, variant_data,"Muffler1")?;
        self.validate_strings(&self.sandbox_data.muffler_2, variant_data,"Muffler2")?;
        self.validate_floats(*&self.sandbox_data.bore, variant_data,"Bore")?;
        self.validate_floats(*&self.sandbox_data.stroke, variant_data,"Stroke")?;
        self.validate_floats(*&self.sandbox_data.capacity, variant_data,"Capacity")?;
        self.validate_floats(*&self.sandbox_data.compression, variant_data,"Compression")?;
        self.validate_floats(*&self.sandbox_data.cam_profile_setting, variant_data,"CamProfileSetting")?;
        self.validate_floats(*&self.sandbox_data.vvl_cam_profile_setting, variant_data,"VVLCamProfileSetting")?;
        if let Some(afr) = &self.sandbox_data.afr { self.validate_floats(*afr, variant_data,"AFR")?; }
        if let Some(afr_lean) = &self.sandbox_data.afr_lean {self.validate_floats(*afr_lean, variant_data,"AFRLean")?; }
        self.validate_floats(*&self.sandbox_data.rpm_limit, variant_data,"RPMLimit")?;
        self.validate_floats(*&self.sandbox_data.ignition_timing_setting, variant_data,"IgnitionTimingSetting")?;
        self.validate_floats(*&self.sandbox_data.exhaust_diameter, variant_data,"ExhaustDiameter")?;
        self.validate_i32(*&self.sandbox_data.quality_bottom_end, variant_data,"QualityBottomEnd")?;
        self.validate_i32(*&self.sandbox_data.quality_top_end, variant_data,"QualityTopEnd")?;
        self.validate_i32(*&self.sandbox_data.quality_aspiration, variant_data,"QualityAspiration")?;
        self.validate_i32(*&self.sandbox_data.quality_fuel_system, variant_data,"QualityFuelSystem")?;
        self.validate_i32(*&self.sandbox_data.quality_exhaust, variant_data,"QualityExhaust")?;
        Ok(())
    }

    fn validate_variant(&self) -> Result<(), String> {
        let variant_data = self.car_file.get_section("Car").unwrap().get_section("Variant").unwrap();
        self.validate_u64( *&self.sandbox_data.variant_version, variant_data, "GameVersion")?;
        self.validate_strings(&self.sandbox_data.family_uuid, variant_data,"FUID")?;
        self.validate_strings(&self.sandbox_data.uuid, variant_data,"UID")?;
        self.validate_strings(&self.sandbox_data.variant_name, variant_data,"Name")?;
        self.validate_i32(*&self.sandbox_data.variant_game_days, variant_data,"InternalDays")?;
        self.validate_strings(&self.sandbox_data.vvl, variant_data,"VVL")?;
        self.validate_strings(&self.sandbox_data.crank, variant_data,"Crank")?;
        self.validate_strings(&self.sandbox_data.conrods, variant_data,"Conrods")?;
        self.validate_strings(&self.sandbox_data.pistons, variant_data,"Pistons")?;
        self.validate_strings(&self.sandbox_data.vvt, variant_data,"VVT")?;
        self.validate_strings(&self.sandbox_data.aspiration, variant_data,"AspirationType")?;
        self.validate_floats(*&self.sandbox_data.intercooler_setting, variant_data,"IntercoolerSetting")?;
        self.validate_strings(&self.sandbox_data.fuel_system_type, variant_data,"FuelSystemType")?;
        self.validate_strings(&self.sandbox_data.fuel_system, variant_data,"FuelSystem")?;
        if let Some(fuel_type) = &self.sandbox_data.fuel_type { self.validate_strings(fuel_type, variant_data,"FuelType")?; }
        if let Some(fuel_leaded) = &self.sandbox_data.fuel_leaded { self.validate_i32(*fuel_leaded, variant_data,"FuelLeaded")?; }
        self.validate_strings(&self.sandbox_data.intake_manifold, variant_data,"IntakeManifold")?;
        self.validate_strings(&self.sandbox_data.intake, variant_data,"Intake")?;
        self.validate_strings(&self.sandbox_data.headers, variant_data,"Headers")?;
        self.validate_strings(&self.sandbox_data.exhaust_count, variant_data,"ExhaustCount")?;
        self.validate_strings(&self.sandbox_data.exhaust_bypass_valves, variant_data,"ExhaustBypassValves")?;
        self.validate_strings(&self.sandbox_data.cat, variant_data,"Cat")?;
        self.validate_strings(&self.sandbox_data.muffler_1, variant_data,"Muffler1")?;
        self.validate_strings(&self.sandbox_data.muffler_2, variant_data,"Muffler2")?;
        self.validate_floats(*&self.sandbox_data.bore, variant_data,"Bore")?;
        self.validate_floats(*&self.sandbox_data.stroke, variant_data,"Stroke")?;
        self.validate_floats(*&self.sandbox_data.capacity, variant_data,"Capacity")?;
        self.validate_floats(*&self.sandbox_data.compression, variant_data,"Compression")?;
        self.validate_floats(*&self.sandbox_data.cam_profile_setting, variant_data,"CamProfileSetting")?;
        self.validate_floats(*&self.sandbox_data.vvl_cam_profile_setting, variant_data,"VVLCamProfileSetting")?;
        if let Some(afr) = &self.sandbox_data.afr { self.validate_floats(*afr, variant_data,"AFR")?; }
        if let Some(afr_lean) = &self.sandbox_data.afr_lean {self.validate_floats(*afr_lean, variant_data,"AFRLean")?; }
        self.validate_floats(*&self.sandbox_data.rpm_limit, variant_data,"RPMLimit")?;
        self.validate_floats(*&self.sandbox_data.ignition_timing_setting, variant_data,"IgnitionTimingSetting")?;
        self.validate_floats(*&self.sandbox_data.exhaust_diameter, variant_data,"ExhaustDiameter")?;
        self.validate_i32(*&self.sandbox_data.quality_bottom_end, variant_data,"QualityBottomEnd")?;
        self.validate_i32(*&self.sandbox_data.quality_top_end, variant_data,"QualityTopEnd")?;
        self.validate_i32(*&self.sandbox_data.quality_aspiration, variant_data,"QualityAspiration")?;
        self.validate_i32(*&self.sandbox_data.quality_fuel_system, variant_data,"QualityFuelSystem")?;
        self.validate_i32(*&self.sandbox_data.quality_exhaust, variant_data,"QualityExhaust")?;
        if let Some(val) = &self.sandbox_data.balance_shaft { self.validate_strings(val, variant_data, "BalanceShaft")?; }
        if let Some(val) = &self.sandbox_data.spring_stiffness { self.validate_floats(*val, variant_data, "SpringStiffnessSetting")?; }
        if let Some(val) = &self.sandbox_data.listed_octane { self.validate_i32(*val, variant_data, "ListedOctane")?; }
        if let Some(val) = &self.sandbox_data.tune_octane_offset { self.validate_i32(*val, variant_data, "TuneOctaneOffset")?; }
        if let Some(val) = &self.sandbox_data.aspiration_setup { self.validate_strings(val, variant_data, "AspirationSetup")?; }
        if let Some(val) = &self.sandbox_data.aspiration_item_1 { self.validate_strings(val, variant_data, "AspirationItemOption_1")?; }
        if let Some(val) = &self.sandbox_data.aspiration_item_2 { self.validate_strings(val, variant_data, "AspirationItemOption_2")?; }
        if let Some(val) = &self.sandbox_data.aspiration_item_suboption_1 { self.validate_strings(val, variant_data, "AspirationItemSubOption_1")?; }
        if let Some(val) = &self.sandbox_data.aspiration_item_suboption_2 { self.validate_strings(val, variant_data, "AspirationItemSubOption_2")?; }
        if let Some(val) = &self.sandbox_data.aspiration_boost_control { self.validate_strings(val, variant_data, "AspirationBoostControl")?; }
        if let Some(val) = &self.sandbox_data.charger_size_1 { self.validate_floats(*val, variant_data, "ChargerSize_1")?; }
        if let Some(val) = &self.sandbox_data.charger_size_2 { self.validate_floats(*val, variant_data, "ChargerSize_2")?; }
        if let Some(val) = &self.sandbox_data.charger_tune_1 { self.validate_floats(*val, variant_data, "ChargerTune_1")?; }
        if let Some(val) = &self.sandbox_data.charger_tune_2 { self.validate_floats(*val, variant_data, "ChargerTune_2")?; }
        if let Some(val) = &self.sandbox_data.charger_max_boost_1 { self.validate_floats(*val, variant_data, "ChargerMaxBoost_1")?; }
        if let Some(val) = &self.sandbox_data.charger_max_boost_2 { self.validate_floats(*val, variant_data, "ChargerMaxBoost_2")?; }
        if let Some(val) = &self.sandbox_data.turbine_size_1 { self.validate_floats(*val, variant_data, "TurbineSize_1")?; }
        if let Some(val) = &self.sandbox_data.turbine_size_2 { self.validate_floats(*val, variant_data, "TurbineSize_2")?; }
        Ok(())
    }

    fn validate_strings(&self,
                        sandbox_value: &str,
                        car_file_section: &Section,
                        car_file_key: &str) -> Result<(), String> {
        let attr = match car_file_section.get_attribute(car_file_key) {
            None => { Err(format!("Car file section {} is missing attribute {}", car_file_section.name(), car_file_key))}
            Some(attribute) => Ok(attribute.value.as_str())
        }?;
        info!("Checking {}", car_file_key);
        if sandbox_value != attr {
            return Err(format!("{}: {} != {}", car_file_key, sandbox_value, attr))
        }
        Ok(())
    }

    fn validate_u64(&self,
                    sandbox_value: u64,
                    car_file_section: &Section,
                    car_file_key: &str) -> Result<(), String> {
        let attr = match car_file_section.get_attribute(car_file_key) {
            None => { Err(format!("Car file section {} is missing attribute {}", car_file_section.name(), car_file_key))}
            Some(attribute) => Ok(attribute.value.as_num()?.round() as u64)
        }?;
        info!("Checking {}", car_file_key);
        if sandbox_value != attr {
            return Err(format!("{}: {} != {}", car_file_key, sandbox_value, attr))
        }
        Ok(())
    }

    fn validate_i32(&self,
                    sandbox_value: i32,
                    car_file_section: &Section,
                    car_file_key: &str) -> Result<(), String> {
        let attr = match car_file_section.get_attribute(car_file_key) {
            None => { Err(format!("Car file section {} is missing attribute {}", car_file_section.name(), car_file_key))}
            Some(attribute) => Ok(attribute.value.as_num()?.round() as i32)
        }?;
        info!("Checking {}", car_file_key);
        if sandbox_value != attr {
            return Err(format!("{}: {} != {}", car_file_key, sandbox_value, attr))
        }
        Ok(())
    }

    fn validate_floats(&self,
                       sandbox_value: f64,
                       car_file_section: &Section,
                       car_file_key: &str) -> Result<(), String> {
        let attr = match car_file_section.get_attribute(car_file_key) {
            None => { Err(format!("Car file section {} is missing attribute {}", car_file_section.name(), car_file_key))}
            Some(attribute) => Ok(attribute.value.as_num()?)
        }?;
        info!("Checking {}", car_file_key);
        if round_float_to(sandbox_value, self.float_precision) !=
            round_float_to(attr, self.float_precision) {
            return Err(format!("{}: {} != {}", car_file_key, sandbox_value, attr))
        }
        Ok(())
    }
}
