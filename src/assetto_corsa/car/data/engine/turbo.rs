use crate::assetto_corsa::traits::{CarDataFile, CarDataUpdater, OptionalDataSection};
use crate::assetto_corsa::error::Result;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::{Ini, IniUpdater};


#[derive(Debug)]
pub struct Turbo {
    pub bov_pressure_threshold: Option<f64>,
    sections: Vec<TurboSection>
}

impl OptionalDataSection for Turbo {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Option<Self>> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let turbo_count: isize = Turbo::count_turbo_sections(ini_data);
        if turbo_count == 0 {
            return Ok(None);
        }

        let pressure_threshold = ini_utils::get_value(ini_data, "BOV", "PRESSURE_THRESHOLD");
        let mut section_vec: Vec<TurboSection> = Vec::new();
        for idx in 0..turbo_count {
            section_vec.push(TurboSection::load_from_ini( idx, ini_data)?);
        }
        Ok(Some(Turbo{
            bov_pressure_threshold: pressure_threshold,
            sections: section_vec
        }))
    }
}

impl Turbo {
    pub fn new() -> Turbo {
        Turbo {
            bov_pressure_threshold: None,
            sections: Vec::new()
        }
    }

    pub fn add_section(&mut self, section: TurboSection) {
        self.sections.push(section)
    }

    pub fn clear_sections(&mut self) {
        self.sections.clear()
    }

    pub fn count_turbo_sections(ini: &Ini) -> isize {
        let mut count = 0;
        loop {
            if !ini.contains_section(TurboSection::get_ini_section_name(count).as_str()) {
                return count;
            }
            count += 1;
        }
    }
}

impl CarDataUpdater for Turbo {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        {
            let ini_data = car_data.mut_ini_data();
            if let Some(bov_pressure_threshold) = self.bov_pressure_threshold {
                ini_utils::set_float(ini_data, "BOV", "PRESSURE_THRESHOLD", bov_pressure_threshold, 2);
            } else {
                ini_data.remove_section("BOV");
            }
            for idx in 0..Turbo::count_turbo_sections(ini_data) {
                ini_data.remove_section(TurboSection::get_ini_section_name(idx).as_str())
            }
        }
        for section in &self.sections {
            section.update_car_data(car_data)?;
        }
        Ok(())
    }
}


#[derive(Debug)]
pub struct TurboSection {
    index: isize,
    lag_dn: f64,
    lag_up: f64,
    max_boost: f64,
    wastegate: f64,
    display_max_boost: f64,
    reference_rpm: i32,
    gamma: f64,
    cockpit_adjustable: i32,
}

impl TurboSection {
    pub fn from_defaults(index: isize) -> TurboSection {
        TurboSection {
            index,
            lag_dn: 0.99,
            lag_up: 0.965,
            max_boost: 1.0,
            wastegate: 1.0,
            display_max_boost: 1.0,
            reference_rpm: 3000,
            gamma: 1.0,
            cockpit_adjustable: 0
        }
    }

    pub fn new(index: isize,
               lag_dn: f64,
               lag_up: f64,
               max_boost: f64,
               wastegate: f64,
               display_max_boost: f64,
               reference_rpm: i32,
               gamma: f64,
               cockpit_adjustable: i32) -> TurboSection
    {
        TurboSection {
            index,
            lag_dn,
            lag_up,
            max_boost,
            wastegate,
            display_max_boost,
            reference_rpm,
            gamma,
            cockpit_adjustable
        }
    }

    pub fn load_from_ini(idx: isize,
                         ini: &Ini) -> Result<TurboSection> {
        let section_name = TurboSection::get_ini_section_name(idx);
        Ok(TurboSection {
            index: idx,
            lag_dn: ini_utils::get_mandatory_property(ini, &section_name, "LAG_DN")?,
            lag_up: ini_utils::get_mandatory_property(ini, &section_name, "LAG_UP")?,
            max_boost: ini_utils::get_mandatory_property(ini, &section_name, "MAX_BOOST")?,
            wastegate: ini_utils::get_mandatory_property(ini, &section_name, "WASTEGATE")?,
            display_max_boost: ini_utils::get_mandatory_property(ini, &section_name, "DISPLAY_MAX_BOOST")?,
            reference_rpm: ini_utils::get_mandatory_property(ini, &section_name, "REFERENCE_RPM")?,
            gamma: ini_utils::get_mandatory_property(ini, &section_name, "GAMMA")?,
            cockpit_adjustable: ini_utils::get_mandatory_property(ini, &section_name, "COCKPIT_ADJUSTABLE")?
        })
    }

    pub fn get_ini_section_name(idx: isize) -> String {
        format!("TURBO_{}", idx)
    }
}

impl CarDataUpdater for TurboSection {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        let section_name = TurboSection::get_ini_section_name(self.index);
        ini_utils::set_float(ini_data, &section_name, "LAG_DN", self.lag_dn, 3);
        ini_utils::set_float(ini_data, &section_name, "LAG_UP", self.lag_up, 3);
        ini_utils::set_float(ini_data, &section_name, "MAX_BOOST", self.max_boost, 2);
        ini_utils::set_float(ini_data, &section_name, "WASTEGATE", self.wastegate, 2);
        ini_utils::set_float(ini_data, &section_name, "DISPLAY_MAX_BOOST", self.display_max_boost, 2);
        ini_utils::set_value(ini_data, &section_name, "REFERENCE_RPM", self.reference_rpm);
        ini_utils::set_float(ini_data, &section_name, "GAMMA", self.gamma, 2);
        ini_utils::set_value(ini_data, &section_name, "COCKPIT_ADJUSTABLE", self.cockpit_adjustable);
        Ok(())
    }
}
