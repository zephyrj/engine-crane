use crate::assetto_corsa::car::data::drivetrain::get_mandatory_field;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
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

impl CarDataUpdater for Clutch {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        ini_utils::set_value(car_data.mut_ini_data(), "CLUTCH", "MAX_TORQUE", self.max_torque);
        Ok(())
    }
}
