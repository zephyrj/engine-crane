use crate::assetto_corsa::car::data::drivetrain::{get_mandatory_field, mandatory_field_error};
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::Ini;
use crate::assetto_corsa::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::assetto_corsa::error::Result;


#[derive(Debug)]
pub struct ShiftProfile {
    name: String,
    points: Vec<i32>
}

impl ShiftProfile {
    pub fn load_from_ini(ini_data: &Ini, name: &str) -> Result<ShiftProfile> {
        let name = String::from(name);
        let mut points = Vec::new();
        for idx in 0..3 {
            points.push(get_mandatory_field(ini_data, &name, &format!("POINT_{}", idx))?);
        }
        Ok(ShiftProfile { name, points })
    }
}

#[derive(Debug)]
pub struct AutoClutch {
    upshift_profile: Option<ShiftProfile>,
    downshift_profile: Option<ShiftProfile>,
    pub use_on_changes: i32,
    pub min_rpm: i32,
    pub max_rpm: i32,
    pub forced_on: i32
}

impl AutoClutch {
    fn load_shift_profile(ini_data: &Ini, key_name: &str) -> Result<Option<ShiftProfile>> {
        if let Some(profile_name) = ini_utils::get_value(ini_data, "AUTOCLUTCH", key_name) {
            let section_name: String = profile_name;
            if section_name.to_lowercase() != "none" {
                return match ShiftProfile::load_from_ini(ini_data, &section_name) {
                    Ok(prof) => { Ok(Some(prof)) },
                    Err(_) => { return Err(mandatory_field_error(key_name, &section_name)); }
                }
            }
        }
        Ok(None)
    }
}

impl MandatoryDataSection for AutoClutch {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let upshift_profile = AutoClutch::load_shift_profile(ini_data, "UPSHIFT_PROFILE")?;
        let downshift_profile = AutoClutch::load_shift_profile(ini_data, "DOWNSHIFT_PROFILE")?;
        let use_on_changes = get_mandatory_field(ini_data, "AUTOCLUTCH", "USE_ON_CHANGES")?;
        let min_rpm = get_mandatory_field(ini_data, "AUTOCLUTCH", "MIN_RPM")?;
        let max_rpm = get_mandatory_field(ini_data, "AUTOCLUTCH", "MAX_RPM")?;
        let forced_on = get_mandatory_field(ini_data, "AUTOCLUTCH", "FORCED_ON")?;

        Ok(AutoClutch {
            upshift_profile,
            downshift_profile,
            use_on_changes,
            min_rpm,
            max_rpm,
            forced_on
        })
    }
}

impl CarDataUpdater for AutoClutch {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "USE_ON_CHANGES", self.use_on_changes);
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "MIN_RPM", self.min_rpm);
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "MAX_RPM", self.max_rpm);
        ini_utils::set_value(ini_data, "AUTOCLUTCH", "FORCED_ON", self.forced_on);
        Ok(())
    }
}
