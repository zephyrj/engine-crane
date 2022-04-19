use crate::assetto_corsa::car::data::drivetrain::get_mandatory_field;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::{Ini, IniUpdater};
use crate::assetto_corsa::traits::{CarDataFile, MandatoryDataSection};
use crate::assetto_corsa::error::Result;


#[derive(Debug)]
pub struct DownshiftProtection {
    pub active: i32,
    pub debug: i32,
    pub overrev: i32,
    pub lock_n: i32
}

impl MandatoryDataSection for DownshiftProtection {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        Ok(DownshiftProtection{
            active: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "ACTIVE")?,
            debug: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "DEBUG")?,
            overrev: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "OVERREV")?,
            lock_n: get_mandatory_field(ini_data, "DOWNSHIFT_PROTECTION", "LOCK_N")?,
        })
    }
}

impl IniUpdater for DownshiftProtection {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "ACTIVE", self.active);
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "DEBUG", self.debug);
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "OVERREV", self.overrev);
        ini_utils::set_value(ini_data, "DOWNSHIFT_PROTECTION", "LOCK_N", self.lock_n);
        Ok(())
    }
}
