use crate::assetto_corsa::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::assetto_corsa::error::Result;
use crate::assetto_corsa::ini_utils;


#[derive(Clone, Debug, PartialEq)]
pub struct EngineData {
    pub altitude_sensitivity: f64,
    pub inertia: f64,
    pub limiter: i32,
    pub limiter_hz: i32,
    pub minimum: i32
}

impl EngineData {
    const SECTION_NAME: &'static str = "ENGINE_DATA";
}

impl MandatoryDataSection for EngineData {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        Ok(EngineData{
            altitude_sensitivity: ini_utils::get_mandatory_property(ini_data, Self::SECTION_NAME, "ALTITUDE_SENSITIVITY")?,
            inertia: ini_utils::get_mandatory_property(ini_data, Self::SECTION_NAME, "INERTIA")?,
            limiter: ini_utils::get_mandatory_property(ini_data, Self::SECTION_NAME, "LIMITER")?,
            limiter_hz: ini_utils::get_mandatory_property(ini_data, Self::SECTION_NAME, "LIMITER_HZ")?,
            minimum: ini_utils::get_mandatory_property(ini_data, Self::SECTION_NAME, "MINIMUM")?
        })
    }
}

impl CarDataUpdater for EngineData {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        ini_utils::set_float(ini_data, Self::SECTION_NAME, "ALTITUDE_SENSITIVITY", self.altitude_sensitivity, 2);
        ini_utils::set_float(ini_data, Self::SECTION_NAME, "INERTIA", self.inertia, 3);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "LIMITER", self.limiter);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "LIMITER_HZ", self.limiter_hz);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "MINIMUM", self.minimum);
        Ok(())
    }
}
