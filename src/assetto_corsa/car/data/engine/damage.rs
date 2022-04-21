use crate::assetto_corsa::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::assetto_corsa::error::Result;
use crate::assetto_corsa::ini_utils;


#[derive(Clone, Debug, PartialEq)]
pub struct Damage {
    rpm_threshold: i32,
    rpm_damage_k: i32,
    turbo_boost_threshold: Option<f64>,
    turbo_damage_k: Option<i32>
}

impl Damage {
    const SECTION_NAME: &'static str = "DAMAGE";

    pub fn new(rpm_threshold: i32,
               rpm_damage_k: i32,
               turbo_boost_threshold: Option<f64>,
               turbo_damage_k: Option<i32>) -> Damage {
        Damage{rpm_threshold, rpm_damage_k, turbo_boost_threshold, turbo_damage_k, }
    }
}

impl MandatoryDataSection for Damage {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        Ok(Damage{
            rpm_threshold: ini_utils::get_mandatory_property(ini_data, Self::SECTION_NAME, "RPM_THRESHOLD")?,
            rpm_damage_k: ini_utils::get_mandatory_property(ini_data, Self::SECTION_NAME, "RPM_DAMAGE_K")?,
            turbo_boost_threshold: ini_utils::get_value(ini_data, Self::SECTION_NAME, "TURBO_BOOST_THRESHOLD"),
            turbo_damage_k: ini_utils::get_value(ini_data, Self::SECTION_NAME, "TURBO_DAMAGE_K"),
        })
    }
}

impl CarDataUpdater for Damage {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "RPM_THRESHOLD", self.rpm_threshold);
        ini_utils::set_value(ini_data, Self::SECTION_NAME, "RPM_DAMAGE_K", self.rpm_damage_k);
        match self.turbo_boost_threshold {
            None => {
                ini_data.remove_value(Self::SECTION_NAME, "TURBO_BOOST_THRESHOLD");
            }
            Some(turbo_boost_threshold) => {
                ini_utils::set_float(ini_data, Self::SECTION_NAME, "TURBO_BOOST_THRESHOLD", turbo_boost_threshold, 2);
            }
        }
        match self.turbo_damage_k {
            None => {
                ini_data.remove_value(Self::SECTION_NAME, "TURBO_DAMAGE_K");
            }
            Some(turbo_damage_k) => {
                ini_utils::set_value(ini_data, Self::SECTION_NAME, "TURBO_DAMAGE_K", turbo_damage_k);
            }
        }
        Ok(())
    }
}
