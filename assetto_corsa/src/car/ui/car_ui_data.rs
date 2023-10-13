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

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use serde_json::{json, Map, Value};

use crate::car::Car;
use crate::error::{Error, ErrorKind, Result};


#[derive(Debug)]
pub struct CarUiData<'a> {
    car: &'a mut Car,
    pub ui_info: UiInfo
}

impl<'a> CarUiData<'a> {
    pub fn from_car(car: &'a mut Car) -> Result<CarUiData<'a>> {
        let ui_info_path = car.root_path.join(["ui", "ui_car.json"].iter().collect::<PathBuf>());
        let ui_info = UiInfo::load(ui_info_path.as_path())?;
        Ok(CarUiData{
            car,
            ui_info
        })
    }

    pub fn write(&'a mut self) -> Result<()> {
        self.ui_info.write()
    }
}


#[derive(Debug, Default)]
pub struct UiInfo {
    ui_info_path: PathBuf,
    json_config: serde_json::Value
}

impl UiInfo {
    fn load(ui_json_path: &Path) -> Result<UiInfo> {
        let ui_info_string = fs::read_to_string(ui_json_path)?;
        let json_config: serde_json::Value = serde_json::from_str(ui_info_string
            .replace("\r\n", "\n")
            .replace("\n", " ")
            .replace("\t", "  ")
            .as_str())?;
        let ui_info = UiInfo {
            ui_info_path: ui_json_path.to_path_buf(),
            json_config
        };
        Ok(ui_info)
    }

    pub fn write(&self) -> Result<()> {
        let writer = BufWriter::new(File::create(&self.ui_info_path)?);
        serde_json::to_writer_pretty(writer, &self.json_config)?;
        Ok(())
    }

    pub fn name(&self) -> Option<&str> {
        self.get_json_string("name")
    }

    pub fn set_name(&mut self, name: String) {
        self.set_json_string("name", name);
    }

    pub fn parent(&self) -> Option<&str> {
        self.get_json_string("parent")
    }

    pub fn set_parent(&mut self, parent: String) {
        self.set_json_string("parent", parent);
    }

    pub fn brand(&self) -> Option<&str> {
        self.get_json_string("brand")
    }

    pub fn description(&self) -> Option<&str> {
        self.get_json_string("description")
    }

    pub fn class(&self) -> Option<&str> {
        self.get_json_string("class")
    }

    pub fn tags(&self) -> Option<Vec<&str>> {
        let mut return_vec: Vec<&str> = Vec::new();
        if let Some(value) = self.json_config.get("tags") {
            if let Some(list) = value.as_array() {
                for val in list {
                    if let Some(v) = val.as_str() {
                        return_vec.push(v);
                    }
                }
                return Some(return_vec);
            }
        }
        None
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        if let Some(value) = self.json_config.get("tags") {
            if let Some(list) = value.as_array() {
                return list.contains(&serde_json::Value::String(tag.to_string()));
            }
        }
        false
    }

    pub fn add_tag(&mut self, new_tag: String) -> Result<()> {
        if let Some(obj) = self.json_config.as_object_mut() {
            if !obj.contains_key("tags") {
                obj.insert("tags".to_string(), serde_json::Value::Array(Vec::new()));
            }
            if let Some(value) = obj.get_mut("tags") {
                if let Some(list) = value.as_array_mut() {
                    list.push(serde_json::Value::String(new_tag));
                    return Ok(())
                }
            }
        }
        Err(Error::new(ErrorKind::UpdateError,
                       "'tags' element of ui data couldn't be accessed".to_string()))
    }

    pub fn add_tag_if_unique(&mut self, new_tag: String) -> Result<bool> {
        if let Some(obj) = self.json_config.as_object_mut() {
            if !obj.contains_key("tags") {
                obj.insert("tags".to_string(), serde_json::Value::Array(Vec::new()));
            }
            if let Some(value) = obj.get_mut("tags") {
                if let Some(list) = value.as_array_mut() {
                    let new_tag = serde_json::Value::String(new_tag);
                    return if !list.contains(&new_tag) {
                        list.push(new_tag);
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
            }
        }
        Err(Error::new(ErrorKind::UpdateError,
                       "'tags' element of ui data couldn't be accessed".to_string()))
    }

    pub fn specs(&self) -> Option<HashMap<&str, SpecValue>> {
        let mut return_map: HashMap<&str, SpecValue> = HashMap::new();
        if let Some(value) = self.json_config.get("specs") {
            if let Some(map) = value.as_object() {
                map.iter().for_each(|(k, v)| {
                    if let Some(val) = SpecValue::parse(k.as_str(), v) {
                        return_map.insert(k.as_str(), val);
                    }
                });
                return Some(return_map);
            }
        }
        None
    }

    pub fn update_spec(&mut self, spec_key: &str, val: String) -> Result<()> {
        if let Some(obj) = self.json_config.as_object_mut() {
            if !obj.contains_key("specs") {
                obj.insert("specs".to_owned(), serde_json::Value::Object(Map::new()));
            }
            if let Some(value) = obj.get_mut("specs") {
                if let Some(map) = value.as_object_mut() {
                    map.remove(spec_key);
                    map.insert(String::from(spec_key), serde_json::Value::String(val));
                    return Ok(());
                }
            }
        }
        Err(Error::new(ErrorKind::UpdateError,
                       "'specs' element of ui data couldn't be accessed".to_string()))
    }

    pub fn torque_curve(&self) -> Option<Vec<Vec<&str>>> {
        self.load_curve_data("torqueCurve")
    }

    pub fn update_torque_curve(&mut self, new_curve_data: Vec<(i32, i32)>) -> Result<()> {
        self.update_curve_data("torqueCurve", new_curve_data)
    }

    pub fn power_curve(&self) -> Option<Vec<Vec<&str>>> {
        self.load_curve_data("powerCurve")
    }

    pub fn update_power_curve(&mut self, new_curve_data: Vec<(i32, i32)>) -> Result<()>  {
        self.update_curve_data("powerCurve", new_curve_data)
    }

    fn get_json_string(&self, key: &str) -> Option<&str> {
        if let Some(value) = self.json_config.get(key) {
            value.as_str()
        } else {
            None
        }
    }

    fn set_json_string(&mut self, key: &str, value: String) {
        match self.json_config.get_mut(key) {
            None => {
                if let Some(obj) = self.json_config.as_object_mut() {
                    obj.insert(String::from(key), serde_json::Value::String(value));
                }
            }
            Some(val) => {
                match val {
                    Value::String(str) => {
                        let _ = std::mem::replace(str, value);
                    }
                    _ => {}
                }
            }
        }
    }

    fn load_curve_data(&self, key: &str) -> Option<Vec<Vec<&str>>> {
        let mut outer_vec: Vec<Vec<&str>> = Vec::new();
        if let Some(value) = self.json_config.get(key) {
            if let Some(out_vec) = value.as_array() {
                out_vec.iter().for_each(|x: &Value| {
                    let mut inner_vec: Vec<&str> = Vec::new();
                    if let Some(v2) = x.as_array() {
                        v2.iter().for_each(|y: &Value| {
                            if let Some(val) = y.as_str() {
                                inner_vec.push(val);
                            }
                        });
                        outer_vec.push(inner_vec);
                    }
                });
                return Some(outer_vec);
            }
        }
        None
    }

    fn update_curve_data(&mut self, key: &str, new_curve_data: Vec<(i32, i32)>) -> Result<()> {
        let mut data_vec: Vec<serde_json::Value> = Vec::new();
        for (rpm, power_bhp) in new_curve_data {
            data_vec.push(json!([format!("{}", rpm), format!("{}", power_bhp)]));
        }
        return match self.json_config.get_mut(key) {
            None => {
                if let Some(map) = self.json_config.as_object_mut() {
                    map.insert(String::from(key),
                               serde_json::Value::Array(data_vec));
                    Ok(())
                } else {
                    Err(Error::new(ErrorKind::UpdateError,
                                   format!("Couldn't access json ui data to update {} curve", key)))
                }
            }
            Some(val) => {
                if let Some(torque_array) = val.as_array_mut() {
                    torque_array.clear();
                    for val in data_vec {
                        torque_array.push(val);
                    }
                    Ok(())
                } else {
                    Err(Error::new(ErrorKind::UpdateError,
                                   format!("Couldn't access {} curve data", key)))
                }
            }
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum SpecValue<'a> {
    Bhp(&'a str),
    Torque(&'a str),
    Weight(&'a str),
    TopSpeed(&'a str),
    Acceleration(&'a str),
    PWRatio(&'a str),
    Range(i32)
}

impl<'a> SpecValue<'a> {
    fn parse(key: &str, value: &'a Value) -> Option<SpecValue<'a>> {
        match key {
            "bhp" => if let Some(val) = value.as_str() { return Some(SpecValue::Bhp(val)); },
            "torque" => if let Some(val) = value.as_str() { return Some(SpecValue::Torque(val)); },
            "weight" => if let Some(val) = value.as_str() { return Some(SpecValue::Weight(val)); },
            "topspeed" => if let Some(val) = value.as_str() { return Some(SpecValue::TopSpeed(val)); },
            "acceleration" => if let Some(val) = value.as_str() { return Some(SpecValue::Acceleration(val)); },
            "pwratio" => if let Some(val) = value.as_str() { return Some(SpecValue::PWRatio(val)); },
            "range" => if let Some(val) = value.as_i64() { return Some(SpecValue::Range(val as i32)); },
            _ => {}
        }
        None
    }
}
