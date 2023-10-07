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

use std::{fmt};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use crate::ini_utils;
use crate::ini_utils::Ini;
use crate::car::lut_utils::LutType;
use crate::traits::{CarDataFile, CarDataUpdater, DataInterface};

#[derive(Debug, Clone)]
pub struct LutProperty<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug
{
    lut: LutType<K, V>,
    section_name: String,
    property_name: String
}

impl<K,V> LutProperty<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug
{
    pub fn new(lut: LutType<K,V>, section_name: String, property_name: String) -> LutProperty<K,V> {
        LutProperty{ lut, section_name, property_name }
    }

    pub fn path_only(section_name: String, property_name: String, ini_data: &Ini) -> Result<LutProperty<K,V>, String> {
        let property_val: String = ini_utils::get_mandatory_property(&ini_data,
                                                                     section_name.as_str(),
                                                                     property_name.as_str()).map_err(|err|{
            err.to_string()
        })?;
        Ok(LutProperty{
            lut: LutType::PathOnly(PathBuf::from(property_val)),
            section_name: String::new(),
            property_name: String::new()
        })
    }

    pub fn mandatory_from_ini(section_name: String,
                              property_name: String,
                              ini_data: &Ini,
                              data_source: &dyn DataInterface) -> Result<LutProperty<K, V>, String>
    {
        let value = ini_utils::get_mandatory_property(ini_data,
                                                      section_name.as_str(),
                                                      property_name.as_str()).map_err(|err| {
            err.to_string()
        })?;
        let lut = LutType::load_from_property_value(value, data_source)?;
        Ok(LutProperty{ lut, section_name, property_name })
    }

    pub fn optional_from_ini(section_name: String,
                             property_name: String,
                             ini_data: &Ini,
                             data_source: &dyn DataInterface) -> Result<Option<LutProperty<K, V>>, String>
    {
        let value: String = match ini_utils::get_value::<String>(ini_data,
                                                                 section_name.as_str(),
                                                                 property_name.as_str()) {
            None => { return Ok(None); }
            Some(val) => {
                val
            }
        };
        let lut = LutType::load_from_property_value(value, data_source)?;
        Ok(Some(LutProperty{ lut, section_name, property_name }))
    }

    pub fn update(&mut self, lut: Vec<(K, V)>) -> Vec<(K, V)> {
        match &mut self.lut {
            LutType::File(lut_file) => { lut_file.update(lut) }
            LutType::Inline(inline_lut) => { inline_lut.update(lut) }
            LutType::PathOnly(_) => { Vec::new() }
        }
    }

    pub fn delete_from_car_data(&self, car_data: &mut dyn CarDataFile) {
        car_data.mut_ini_data().remove_value(&self.section_name, &self.property_name);
        match &self.lut {
            LutType::File(lut_file) => {
                lut_file.delete(car_data.mut_data_interface())
            },
            _ => ()
        }
    }

    pub fn to_vec(&self) -> Vec<(K, V)> {
        match &self.lut {
            LutType::File(lut_file) => { lut_file.to_vec() }
            LutType::Inline(inline_lut) => { inline_lut.to_vec() }
            LutType::PathOnly(_) => { Vec::new() }
        }
    }

    pub fn clone_values(&self) -> Vec<V> {
        match &self.lut {
            LutType::File(lut_file) => { lut_file.clone_values() }
            LutType::Inline(inline_lut) => { inline_lut.clone_values() }
            LutType::PathOnly(_) => { Vec::new() }
        }
    }

    pub fn get_type(&self) -> &LutType<K, V> {
        &self.lut
    }

    pub fn get_mut_type(&mut self) -> &mut LutType<K, V> {
        &mut self.lut
    }

    pub fn num_entries(&self) -> usize {
        match &self.lut {
            LutType::PathOnly(_) => 0,
            LutType::File(lut_file) => lut_file.num_entries(),
            LutType::Inline(inline_lut) => inline_lut.num_entries()
        }
    }

}

impl <K,V> CarDataUpdater for LutProperty<K, V>
    where  K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
           V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug
{
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> crate::error::Result<()> {
        match &self.lut {
            LutType::File(lut_file) => {
                ini_utils::set_value(car_data.mut_ini_data(),
                                     self.section_name.as_str(),
                                     self.property_name.as_str(),
                                     &lut_file.filename);
                car_data.mut_data_interface().update_file_data(&lut_file.filename,
                                                               lut_file.to_bytes());
            }
            LutType::Inline(lut) => {
                ini_utils::set_value(car_data.mut_ini_data(),
                                     self.section_name.as_str(),
                                     self.property_name.as_str(),
                                     lut.to_string());
            }
            LutType::PathOnly(path) => {
                ini_utils::set_value(car_data.mut_ini_data(),
                                     self.section_name.as_str(),
                                     self.property_name.as_str(),
                                     format!("{}", path.display()));
            }
        }
        Ok(())
    }
}
