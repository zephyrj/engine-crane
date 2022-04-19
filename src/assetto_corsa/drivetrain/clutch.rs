use crate::assetto_corsa::drivetrain::get_mandatory_field;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::{Ini, IniUpdater};
use crate::assetto_corsa::traits::{CarDataFile, MandatoryDataSection};
use crate::assetto_corsa::error::Result;


#[derive(Debug)]
pub struct Clutch {
    pub max_torque: i32
}

impl MandatoryDataSection for Clutch {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> {
        Ok(Clutch{
            max_torque: get_mandatory_field(parent_data.ini_data(), "CLUTCH", "MAX_TORQUE")?
        })
    }
}

impl IniUpdater for Clutch {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_value(ini_data, "CLUTCH", "MAX_TORQUE", self.max_torque);
        Ok(())
    }
}
