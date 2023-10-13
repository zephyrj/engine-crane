/*
 * Copyright (c):
 * 2023 zephyrj
 * zephyrj@protonmail.com
 *
 * This file is part of engine-crane.
 *
 * engine-crane is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * engine-crane is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
 */



use crate::error::{Error, ErrorKind, Result};
use crate::ini_utils;
use crate::ini_utils::Ini;
use crate::traits::{CarDataFile, CarDataUpdater, OptionalDataSection};

#[derive(Debug)]
pub struct ShiftLights {
    shift_leds: Vec<Led>,
}

impl ShiftLights {
    pub fn count_shift_leds(ini_data: &Ini) -> usize {
        let mut count = 0;
        loop {
            if !ini_data.contains_section(&Led::get_ini_section_name(count)) {
                return count;
            }
            count += 1;
        }
    }

    pub fn update_limiter(&mut self, old_limiter: u32, new_limiter: u32) {
        for led in &mut self.shift_leds {
            let old_rpm_switch = led.rpm_switch();
            if old_rpm_switch == old_limiter {
                *led.mut_rpm_switch() = new_limiter;
            } else {
                let old_percentage = get_as_rounded_percentage_of(old_rpm_switch, old_limiter);
                *led.mut_rpm_switch() = round_to_nearest_hundred(get_percentage_of(old_percentage, new_limiter));
            }
            let old_blink_switch = led.blink_switch();
            if old_blink_switch > old_limiter && old_blink_switch < new_limiter {
                *led.mut_blink_switch() = new_limiter + 100;
            } else if old_blink_switch == old_limiter {
                *led.mut_blink_switch() = new_limiter;
            } else {
                let old_percentage = get_as_rounded_percentage_of(old_blink_switch, old_limiter);
                *led.mut_blink_switch() = round_to_nearest_hundred(get_percentage_of(old_percentage, new_limiter));
            }
        }
    }

    pub fn num_leds(&self) -> usize {
        self.shift_leds.len()
    }

    pub fn mut_shift_leds(&mut self) -> &mut Vec<Led> {
        &mut self.shift_leds
    }
}

impl OptionalDataSection for ShiftLights {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Option<Self>> where Self: Sized {
        let ini_data = parent_data.ini_data();
        let led_count = ShiftLights::count_shift_leds(ini_data);
        match led_count {
            0 => Ok(None),
            _ => {
                let mut shift_leds = Vec::new();
                for idx in 0..led_count {
                    shift_leds.push(Led::load_from_parent(idx, parent_data)?);
                }
                Ok(Some(ShiftLights{ shift_leds }))
            }
        }
    }
}

impl CarDataUpdater for ShiftLights {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        for led in &self.shift_leds {
            led.update_car_data(car_data)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Led {
    index: usize,
    object_name: String,
    rpm_switch: u32,
    emmisive: (f64, f64, f64),
    diffuse: f64,
    blink_switch: u32,
    blink_hz: u32
}

impl Led {
    pub fn get_ini_section_name(idx: usize) -> String {
        format!("LED_{}", idx)
    }

    pub fn load_from_parent(idx: usize, parent_data: &dyn CarDataFile) -> Result<Led> {
        let section_name = Led::get_ini_section_name(idx);
        let ini_data = parent_data.ini_data();
        let emmisive_string: String = ini_utils::get_mandatory_property(ini_data, &section_name, "EMISSIVE")?;
        let res: Result<Vec<f64>> = emmisive_string.split(",").map(|str| {
            match str.parse::<f64>() {
                Ok(num) => { Ok(num) }
                Err(e) => {
                    Err(Error::new(ErrorKind::InvalidCar,
                                   format!("Cannot parse emmisive elements as ints. {}",
                                                  e.to_string())))
                }
            }
        }).collect();
        let em_vec = res?;
        Ok(Led {
            index: idx,
            object_name: ini_utils::get_mandatory_property(ini_data, &section_name, "OBJECT_NAME")?,
            rpm_switch: ini_utils::get_mandatory_property(ini_data, &section_name, "RPM_SWITCH")?,
            emmisive: (em_vec[0], em_vec[1], em_vec[2]),
            diffuse: ini_utils::get_mandatory_property(ini_data, &section_name, "DIFFUSE")?,
            blink_switch: ini_utils::get_mandatory_property(ini_data, &section_name, "BLINK_SWITCH")?,
            blink_hz: ini_utils::get_mandatory_property(ini_data, &section_name,"BLINK_HZ")?
        })
    }

    pub fn section_name(&self) -> String {
        Led::get_ini_section_name(self.index)
    }

    pub fn rpm_switch(&self) -> u32 {
        self.rpm_switch
    }

    pub fn mut_rpm_switch(&mut self) -> &mut u32 {
        &mut self.rpm_switch
    }

    pub fn blink_switch(&self) -> u32 {
        self.blink_switch
    }

    pub fn mut_blink_switch(&mut self) -> &mut u32 {
        &mut self.blink_switch
    }
}

impl CarDataUpdater for Led {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        let ini_data = car_data.mut_ini_data();
        let section_name = self.section_name();
        ini_utils::set_value(ini_data, &section_name, "OBJECT_NAME", self.object_name.clone());
        ini_utils::set_value(ini_data, &section_name, "RPM_SWITCH", self.rpm_switch);
        ini_utils::set_value(
            ini_data,
            &section_name,
            "EMISSIVE",
            format!("{},{},{}", self.emmisive.0, self.emmisive.1, self.emmisive.2));
        ini_utils::set_float(ini_data, &section_name, "DIFFUSE", self.diffuse, 2);
        ini_utils::set_value(ini_data, &section_name, "BLINK_SWITCH", self.blink_switch);
        ini_utils::set_value(ini_data, &section_name, "BLINK_HZ", self.blink_hz);
        Ok(())
    }
}

fn get_as_rounded_percentage_of(num: u32, of: u32) -> u32 {
    ((num as f64 / of as f64) * 100_f64).round() as u32
}

fn get_percentage_of(percentage: u32, of: u32) -> f64 {
    (of as f64) * (percentage as f64 / 100_f64)
}

fn round_to_nearest_hundred(val: f64) -> u32 {
    if val as u32 == 0 {
        return 0;
    }
    ((val / 100_f64).round() as u32) * 100
}


#[cfg(test)]
mod tests {
    use crate::car::data::digital_instruments::shift_lights::{get_as_rounded_percentage_of, get_percentage_of, round_to_nearest_hundred};

    #[test]
    fn get_as_rounded_percentage_of_check() {
        assert_eq!(get_as_rounded_percentage_of(0, 1000), 0);
        assert_eq!(get_as_rounded_percentage_of(100, 1000), 10);
        assert_eq!(get_as_rounded_percentage_of(101, 1000), 10);
        assert_eq!(get_as_rounded_percentage_of(189, 1000), 19);
        assert_eq!(get_as_rounded_percentage_of(999, 1000), 100);
        assert_eq!(get_as_rounded_percentage_of(1000, 1000), 100);
    }

    #[test]
    fn get_percentage_of_check() {
        assert_eq!(get_percentage_of(0, 1000), 0_f64);
        assert_eq!(get_percentage_of(1, 1000), 10_f64);
        assert_eq!(get_percentage_of(100, 1000), 1000_f64);
        assert_eq!(get_percentage_of(10, 1000), 100_f64);
        assert_eq!(get_percentage_of(50, 1000), 500_f64);
        assert_eq!(get_percentage_of(90, 1000), 900_f64);
    }

    #[test]
    fn round_to_nearest_hundred_check() {
        assert_eq!(round_to_nearest_hundred(0_f64), 0);
        assert_eq!(round_to_nearest_hundred(49_f64), 0);
        assert_eq!(round_to_nearest_hundred(50_f64), 100);
        assert_eq!(round_to_nearest_hundred(51_f64), 100);
        assert_eq!(round_to_nearest_hundred(99_f64), 100);
        assert_eq!(round_to_nearest_hundred(100_f64), 100);
        assert_eq!(round_to_nearest_hundred(101_f64), 100);
        assert_eq!(round_to_nearest_hundred(149_f64), 100);
        assert_eq!(round_to_nearest_hundred(150_f64), 200);
        assert_eq!(round_to_nearest_hundred(151_f64), 200);
    }
}
