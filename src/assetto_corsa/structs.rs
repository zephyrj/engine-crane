use std::fmt;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use crate::assetto_corsa::ini_utils;
use crate::assetto_corsa::ini_utils::{Ini, IniUpdater};
use crate::assetto_corsa::lut_utils::LutType;

#[derive(Debug)]
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
                              data_dir: &Path) -> Result<LutProperty<K, V>, String>
    {
        let value = ini_utils::get_mandatory_property(ini_data,
                                                      section_name.as_str(),
                                                      property_name.as_str()).map_err(|err| {
            err.to_string()
        })?;
        let lut = LutType::load_from_property_value(value, data_dir)?;
        Ok(LutProperty{ lut, section_name, property_name })
    }

    pub fn optional_from_ini(section_name: String,
                             property_name: String,
                             ini_data: &Ini,
                             data_dir: &Path) -> Result<Option<LutProperty<K, V>>, String>
    {
        let value: String = match ini_utils::get_value::<String>(ini_data,
                                                                 section_name.as_str(),
                                                                 property_name.as_str()) {
            None => { return Ok(None); }
            Some(val) => {
                val
            }
        };
        let lut = LutType::load_from_property_value(value, data_dir)?;
        Ok(Some(LutProperty{ lut, section_name, property_name }))
    }
    
    pub fn update(&mut self, lut: Vec<(K, V)>) -> Vec<(K, V)> {
        match &mut self.lut {
            LutType::File(lut_file) => { lut_file.update(lut) }
            LutType::Inline(inline_lut) => { inline_lut.update(lut) }
            LutType::PathOnly(_) => { Vec::new() }
        }
    }

    pub fn to_vec(&self) -> Vec<(K, V)> {
        match &self.lut {
            LutType::File(lut_file) => { lut_file.to_vec() }
            LutType::Inline(inline_lut) => { inline_lut.to_vec() }
            LutType::PathOnly(_) => { Vec::new() }
        }
    }

    pub fn write(&self) -> std::result::Result<(), String> {
        return match &self.lut {
            LutType::File(lut_file) => { lut_file.write() },
            _ => { Ok(()) }
        }
    }
}

impl<K, V> IniUpdater for LutProperty<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug
{
    fn update_ini(&self, ini_data: &mut Ini) -> Result<(), String> {
        match &self.lut {
            LutType::File(lut_file) => {
                ini_utils::set_value(ini_data,
                                     self.section_name.as_str(),
                                     self.property_name.as_str(),
                                     lut_file.path.file_name().unwrap().to_str().unwrap());
            }
            LutType::Inline(lut) => {
                ini_utils::set_value(ini_data,
                                     self.section_name.as_str(),
                                     self.property_name.as_str(),
                                     lut.to_string());
            }
            LutType::PathOnly(path) => {
                ini_utils::set_value(ini_data,
                                     self.section_name.as_str(),
                                     self.property_name.as_str(),
                                     format!("{}", path.display()));
            }
        }
        Ok(())
    }
}