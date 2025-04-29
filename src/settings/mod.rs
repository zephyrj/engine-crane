use std::fs;
use std::path::PathBuf;
use config::{Config, ConfigError};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use zephyrj_ac_tools as assetto_corsa;

macro_rules! default_config_builder_helper {
    ($($config_type:ty),+) => {
        {
            let mut builder = Config::builder();
            $(builder = builder.set_default(<$config_type>::param_name(), <$config_type>::default())?;)+
            builder
        }
    }
}

macro_rules! default_config_builder {
    () => {
        default_config_builder_helper!(
            AcInstallPath,
            BeamNGModPath,
            CrateEnginePath,
            LegacyAutomationUserdataPath,
            AutomationUserdataPath
        )
    }
}

pub trait Setting {
    type ValueType;

    fn param_name() -> &'static str;
    fn friendly_name() -> &'static str;
    fn default() -> Self::ValueType;
    fn get(global_settings: &GlobalSettings) -> &Self::ValueType;
    fn set(global_settings: &mut GlobalSettings, new_val: Self::ValueType);
    fn set_default(global_settings: &mut GlobalSettings) {
        Self::set(global_settings, Self::default())
    }
}

pub trait PathSetting : Setting<ValueType=String>
{
    fn resolve_path(global_settings: &GlobalSettings) -> Option<PathBuf> {
        let path = PathBuf::from(Self::get(global_settings));
        match path.is_dir() {
            true => Some(path),
            false => None
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalSettings {
    ac_install_path: String,
    beamng_mod_path: String,
    crate_engine_path: String,
    legacy_automation_userdata_path: String,
    automation_userdata_path: String
}

impl GlobalSettings {
    const CONFIG_FILENAME: &'static str = "engine-crane-conf";

    pub fn default() -> Self {
        GlobalSettings {
            ac_install_path: AcInstallPath::default(),
            beamng_mod_path: BeamNGModPath::default(),
            crate_engine_path: CrateEnginePath::default(),
            legacy_automation_userdata_path: LegacyAutomationUserdataPath::default(),
            automation_userdata_path: AutomationUserdataPath::default(),
        }
    }

    pub fn load() -> Result<Self, ConfigError> {
        return match default_config_builder!()
            .add_source(config::File::with_name(GlobalSettings::CONFIG_FILENAME))
            .add_source(config::Environment::with_prefix("APP"))
            .build() {
            Ok(settings) => {
                settings.try_deserialize()
            }
            Err(e) => {
                warn!("Failed to load settings. {}", e.to_string());
                let settings = default_config_builder!().build()?;
                let ret: GlobalSettings = settings.try_deserialize()?;
                ret.write().unwrap_or_else(|e| { error!("Failed to write settings. {}", e.to_string())});
                Ok(ret)
            }
        }
    }

    pub fn get<T: Setting>(&self) -> &T::ValueType {
        T::get(self)
    }

    pub fn set<T: Setting>(&mut self, val: T::ValueType) {
        T::set(self, val)
    }

    pub fn set_default<T: Setting>(&mut self) {
        T::set_default(self)
    }

    pub fn write(&self) -> std::io::Result<()> {
        fs::write(format!("{}.toml", GlobalSettings::CONFIG_FILENAME), toml::to_string(&self).map_err(|_e|{
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to encode settings to toml")
        })?)
    }
}

pub struct AcInstallPath {}
impl PathSetting for AcInstallPath {}
impl Setting for AcInstallPath {
    type ValueType = String;
    fn param_name() -> &'static str { "ac_install_path" }

    fn friendly_name() -> &'static str { "Assetto Corsa install path" }

    fn default() -> Self::ValueType {
        assetto_corsa::get_default_install_path().to_string_lossy().into_owned()
    }

    fn get(global_settings: &GlobalSettings) -> &Self::ValueType {
        &global_settings.ac_install_path
    }

    fn set(global_settings: &mut GlobalSettings, new_val: Self::ValueType) {
        global_settings.ac_install_path = new_val
    }
}

pub struct BeamNGModPath {}
impl PathSetting for BeamNGModPath {}
impl Setting for BeamNGModPath {
    type ValueType = String;
    fn param_name() -> &'static str { "beamng_mod_path" }

    fn friendly_name() -> &'static str { "BeamNG mod path" }

    fn default() -> Self::ValueType {
        beam_ng::get_default_mod_path().to_string_lossy().into_owned()
    }

    fn get(global_settings: &GlobalSettings) -> &Self::ValueType {
        &global_settings.beamng_mod_path
    }

    fn set(global_settings: &mut GlobalSettings, new_val: Self::ValueType) {
        global_settings.beamng_mod_path = new_val
    }
}

pub struct CrateEnginePath {}
impl PathSetting for CrateEnginePath {}

impl Setting for CrateEnginePath {
    type ValueType = String;
    fn param_name() -> &'static str { "crate_engine_path" }
    fn friendly_name() -> &'static str { "Crate engine path" }
    fn default() -> Self::ValueType {
        crate::data::get_default_crate_engine_path().to_string_lossy().into_owned()
    }

    fn get(global_settings: &GlobalSettings) -> &Self::ValueType {
        &global_settings.crate_engine_path
    }

    fn set(global_settings: &mut GlobalSettings, new_val: Self::ValueType) {
        global_settings.crate_engine_path = new_val
    }
}

pub struct LegacyAutomationUserdataPath {}
impl PathSetting for LegacyAutomationUserdataPath {}
impl Setting for LegacyAutomationUserdataPath {
    type ValueType = String;
    fn param_name() -> &'static str { "legacy_automation_userdata_path" }
    fn friendly_name() -> &'static str { "Legacy Automation userdata path" }

    fn default() -> Self::ValueType {
        zephyrj_automation_tools::sandbox::get_default_legacy_user_data_path().to_string_lossy().into_owned()
    }

    fn get(global_settings: &GlobalSettings) -> &Self::ValueType {
        &global_settings.legacy_automation_userdata_path
    }

    fn set(global_settings: &mut GlobalSettings, new_val: Self::ValueType) {
        global_settings.legacy_automation_userdata_path = new_val
    }
}

pub struct AutomationUserdataPath {}
impl PathSetting for AutomationUserdataPath {}

impl Setting for AutomationUserdataPath {
    type ValueType = String;
    fn param_name() -> &'static str { "automation_userdata_path" }
    fn friendly_name() -> &'static str { "Automation userdata path" }
    fn default() -> Self::ValueType {
        zephyrj_automation_tools::sandbox::get_default_user_data_path().to_string_lossy().into_owned()
    }

    fn get(global_settings: &GlobalSettings) -> &Self::ValueType {
        &global_settings.automation_userdata_path
    }

    fn set(global_settings: &mut GlobalSettings, new_val: Self::ValueType) {
        global_settings.automation_userdata_path = new_val
    }
}
