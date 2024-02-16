use std::fs;
use std::path::{Path, PathBuf};
use config::{Config, ConfigError};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalSettings {
    ac_install_path: String,
    beamng_mod_path: String,
    crate_engine_path: String,
    legacy_automation_userdata_path: String,
    automation_userdata_path: String
}

impl GlobalSettings {
    const AC_INSTALL_PATH: &'static str = "ac_install_path";
    const BEAMNG_MOD_PATH: &'static str = "beamng_mod_path";
    const CRATE_ENGINE_PATH: &'static str = "crate_engine_path";
    const LEGACY_AUTOMATION_USERDATA_PATH: &'static str = "legacy_automation_userdata_path";
    const AUTOMATION_USERDATA_PATH: &'static str = "automation_userdata_path";
    const CONFIG_FILENAME: &'static str = "engine-crane-conf";


    pub fn default() -> Self {
        GlobalSettings {
            ac_install_path: assetto_corsa::get_default_install_path().to_string_lossy().into_owned(),
            beamng_mod_path: beam_ng::get_default_mod_path().to_string_lossy().into_owned(),
            crate_engine_path: crate::data::get_default_crate_engine_path().to_string_lossy().into_owned(),
            legacy_automation_userdata_path: automation::sandbox::get_default_legacy_user_data_path().to_string_lossy().into_owned(),
            automation_userdata_path: automation::sandbox::get_default_user_data_path().to_string_lossy().into_owned(),
        }
    }

    pub fn load() -> Result<Self, ConfigError> {
        let builder = Config::builder();
        return match builder
            .set_default(GlobalSettings::AC_INSTALL_PATH, assetto_corsa::get_default_install_path().to_string_lossy().into_owned())?
            .set_default(GlobalSettings::BEAMNG_MOD_PATH, beam_ng::get_default_mod_path().to_string_lossy().into_owned())?
            .set_default(GlobalSettings::CRATE_ENGINE_PATH, crate::data::get_default_crate_engine_path().to_string_lossy().into_owned())?
            .set_default(GlobalSettings::LEGACY_AUTOMATION_USERDATA_PATH, automation::sandbox::get_default_legacy_user_data_path().to_string_lossy().into_owned())?
            .set_default(GlobalSettings::AUTOMATION_USERDATA_PATH, automation::sandbox::get_default_user_data_path().to_string_lossy().into_owned())?
            .add_source(config::File::with_name(GlobalSettings::CONFIG_FILENAME))
            .add_source(config::Environment::with_prefix("APP"))
            .build() {
            Ok(settings) => {
                settings.try_deserialize()
            }
            Err(e) => {
                warn!("Failed to load settings. {}", e.to_string());
                let builder = Config::builder();
                let settings = builder
                    .set_default(GlobalSettings::AC_INSTALL_PATH, assetto_corsa::get_default_install_path().to_string_lossy().into_owned())?
                    .set_default(GlobalSettings::BEAMNG_MOD_PATH, beam_ng::get_default_mod_path().to_string_lossy().into_owned())?
                    .set_default(GlobalSettings::CRATE_ENGINE_PATH, crate::data::get_default_crate_engine_path().to_string_lossy().into_owned())?
                    .set_default(GlobalSettings::LEGACY_AUTOMATION_USERDATA_PATH, automation::sandbox::get_default_legacy_user_data_path().to_string_lossy().into_owned())?
                    .set_default(GlobalSettings::AUTOMATION_USERDATA_PATH, automation::sandbox::get_default_user_data_path().to_string_lossy().into_owned())?
                    .build()?;
                let ret: GlobalSettings = settings.try_deserialize()?;
                ret.write().unwrap_or_else(|e| { error!("Failed to write settings. {}", e.to_string())});
                Ok(ret)
            }
        }
    }

    pub fn ac_install_path(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.ac_install_path);
        if path.is_dir() {
            return Some(path);
        }
        None
    }

    pub fn set_ac_install_path(&mut self, new_path: &Path) {
        self.ac_install_path = new_path.to_string_lossy().into_owned();
    }

    pub fn beamng_mod_path(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.beamng_mod_path);
        if path.is_dir() {
            return Some(path);
        }
        None
    }

    pub fn set_beamng_mod_path(&mut self, new_path: &Path) {
        self.beamng_mod_path = new_path.to_string_lossy().into_owned();
    }

    pub fn crate_engine_path(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.crate_engine_path);
        if path.is_dir() {
            return Some(path);
        }
        None
    }

    pub fn set_crate_engine_pahth(&mut self, new_path: &Path) {
        self.crate_engine_path = new_path.to_string_lossy().into_owned();
    }

    pub fn legacy_automation_userdata_path(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.legacy_automation_userdata_path);
        if path.is_dir() {
            return Some(path);
        }
        None
    }

    pub fn set_legacy_automation_userdata_path(&mut self, new_path: &Path) {
        self.legacy_automation_userdata_path = new_path.to_string_lossy().into_owned();
    }

    pub fn automation_userdata_path(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.automation_userdata_path);
        if path.is_dir() {
            return Some(path);
        }
        None
    }

    pub fn set_automation_userdata_path(&mut self, new_path: &Path) {
        self.automation_userdata_path = new_path.to_string_lossy().into_owned();
    }

    pub fn write(&self) -> std::io::Result<()> {
        fs::write(format!("{}.toml", GlobalSettings::CONFIG_FILENAME), toml::to_string(&self).map_err(|_e|{
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to encode settings to toml")
        })?)
    }
}
