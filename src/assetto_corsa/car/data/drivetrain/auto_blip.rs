use crate::assetto_corsa::car::data::drivetrain::get_mandatory_field;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::{Ini, IniUpdater};
use crate::assetto_corsa::traits::{CarDataFile, MandatoryDataSection};
use crate::assetto_corsa::error::Result;


#[derive(Debug)]
pub struct AutoBlip {
    pub electronic: i32,
    pub points: Vec<i32>,
    pub level: f64
}

impl AutoBlip {
    fn get_point_key<T: std::fmt::Display>(idx: T) -> String {
        format!("POINT_{}", idx)
    }
}

impl MandatoryDataSection for AutoBlip {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let electronic = get_mandatory_field(ini_data, "AUTOBLIP", "ELECTRONIC")?;
        let level = get_mandatory_field(ini_data, "AUTOBLIP", "LEVEL")?;
        let mut points = Vec::new();
        for idx in 0..3 {
            points.push(get_mandatory_field(ini_data,
                                            "AUTOBLIP",
                                            AutoBlip::get_point_key(idx).as_str())?);
        }
        Ok(AutoBlip{ electronic, points, level })
    }
}

impl IniUpdater for AutoBlip {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String> {
        ini_utils::set_value(ini_data, "AUTOBLIP", "ELECTRONIC", self.electronic);
        ini_utils::set_float(ini_data, "AUTOBLIP", "LEVEL", self.level, 2);
        for (idx, point) in self.points.iter().enumerate() {
            ini_utils::set_value(ini_data,
                                 "AUTOBLIP",
                                 AutoBlip::get_point_key(idx).as_str(),
                                 point);
        }
        Ok(())
    }
}
