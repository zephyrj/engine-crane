use crate::assetto_corsa::drivetrain::{get_mandatory_field, mandatory_field_error};
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::{Ini, IniUpdater};
use crate::assetto_corsa::traits::{CarDataFile, MandatoryDataSection};
use crate::assetto_corsa::error::Result;


#[derive(Debug)]
pub struct AutoShifter {
    pub up: i32,
    pub down: i32,
    pub slip_threshold: f64,
    pub gas_cutoff_time: f64
}

impl MandatoryDataSection for AutoShifter {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let up = get_mandatory_field(ini_data, "AUTO_SHIFTER", "UP")?;
        let down = get_mandatory_field(ini_data, "AUTO_SHIFTER", "DOWN")?;
        let slip_threshold = get_mandatory_field(ini_data, "AUTO_SHIFTER", "SLIP_THRESHOLD")?;
        let gas_cutoff_time = get_mandatory_field(ini_data, "AUTO_SHIFTER", "GAS_CUTOFF_TIME")?;
        Ok(AutoShifter{ up, down, slip_threshold, gas_cutoff_time })
    }
}

impl IniUpdater for AutoShifter {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_value(ini_data, "AUTO_SHIFTER", "UP", self.up);
        ini_utils::set_value(ini_data, "AUTO_SHIFTER", "DOWN", self.down);
        ini_utils::set_float(ini_data, "AUTO_SHIFTER", "SLIP_THRESHOLD", self.slip_threshold, 2);
        ini_utils::set_float(ini_data, "AUTO_SHIFTER", "GAS_CUTOFF_TIME", self.gas_cutoff_time, 2);
        Ok(())
    }
}
