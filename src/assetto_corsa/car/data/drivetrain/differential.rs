use crate::assetto_corsa::car::data::drivetrain::get_mandatory_field;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::{Ini, IniUpdater};
use crate::assetto_corsa::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::assetto_corsa::error::Result;


#[derive(Debug)]
pub struct Differential {
    pub power: f64,
    pub coast: f64,
    pub preload: i32
}

impl MandatoryDataSection for Differential {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let power = get_mandatory_field(ini_data, "DIFFERENTIAL", "POWER")?;
        let coast = get_mandatory_field(ini_data, "DIFFERENTIAL", "COAST")?;
        let preload = get_mandatory_field(ini_data, "DIFFERENTIAL", "PRELOAD")?;
        Ok(Differential { power, coast, preload })
    }
}

impl CarDataUpdater for Differential {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        ini_utils::set_float(ini_data, "DIFFERENTIAL", "POWER", self.power, 2);
        ini_utils::set_float(ini_data, "DIFFERENTIAL", "COAST", self.coast, 2);
        ini_utils::set_value(ini_data, "DIFFERENTIAL", "PRELOAD", self.preload);
        Ok(())
    }
}
