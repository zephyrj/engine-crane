/*
 * Copyright (c):
 * 2024 zephyrj
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

use std::{fmt, io};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use csv::Terminator;

use tracing::error;
use crate::traits::DataInterface;


#[derive(Debug, Clone)]
pub struct LutFile<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    pub filename: String,
    data: Vec<(K,V)>
}

impl<K, V> LutFile<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    pub fn new(filename: String, data: Vec<(K, V)>) -> LutFile<K, V> {
        LutFile { filename, data }
    }

    pub fn from_path(lut_path: &Path) -> Result<LutFile<K, V>, String> {
        let filename = match lut_path.file_name() {
            None => { return Err(format!("Failed to get filename for {}", lut_path.display()))}
            Some(n) => { n.to_string_lossy().to_string() }
        };
        Ok(LutFile {
            filename,
            data: load_lut_from_path::<K, V>(lut_path)?
        })
    }

    pub fn delete(&self, data_interface: &mut dyn DataInterface) {
        data_interface.remove_file(&self.filename)
    }
    
    pub fn update(&mut self, data: Vec<(K,V)>) -> Vec<(K,V)> {
        std::mem::replace(&mut self.data, data)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        write_lut_to_bytes(&self.data).unwrap_or_else(|e|{
            error!("Couldn't write lut as bytes. {}", e.to_string());
            Vec::new()
        })
    }

    pub fn write_to_dir(&self, dir: &Path) -> Result<(), String> {
        write_lut_to_path(&self.data, &dir.join(Path::new(&self.filename)))?;
        Ok(())
    }

    pub fn to_vec(&self) -> Vec<(K,V)> {
        self.data.clone()
    }

    pub fn clone_values(&self) -> Vec<V> {
        self.data.iter().map(|(_, val)|{(*val).clone()}).collect()
    }

    pub fn num_entries(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Clone)]
pub struct InlineLut<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    data: Vec<(K,V)>
}

impl<K, V> InlineLut<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    pub fn new() -> InlineLut<K, V> {
        InlineLut { data: Vec::new() }
    }

    pub fn from_vec(data: Vec<(K,V)>) -> InlineLut<K,V> {
        InlineLut { data }
    }

    pub fn from_property_value(property_value: String) -> Result<InlineLut<K, V>, String> {
        let data_slice = &property_value[1..(property_value.len() - 1)];
        let data = load_lut_from_reader::<K, V, _>(data_slice.as_bytes(), b'=', Terminator::Any(b'|'))?;
        Ok(InlineLut { data })
    }

    pub fn update(&mut self, data: Vec<(K,V)>) -> Vec<(K,V)> {
        std::mem::replace(&mut self.data, data)
    }

    pub fn to_vec(&self) -> Vec<(K,V)> {
        self.data.clone()
    }

    pub fn clone_values(&self) -> Vec<V> {
        self.data.iter().map(|(_, val)|{(*val).clone()}).collect()
    }

    pub fn num_entries(&self) -> usize {
        self.data.len()
    }
}

impl<K, V> Display for InlineLut<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}",
               write_lut_to_property_value(&self.data, b'=', Terminator::Any(b'|')).map_err(
                   |_err| { fmt::Error }
               )?)
    }
}

#[derive(Debug, Clone)]
pub enum LutType<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    File(LutFile<K,V>),
    Inline(InlineLut<K,V>),
    PathOnly(PathBuf)
}

impl<K, V> LutType<K, V>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    pub fn path_only(path: PathBuf) -> LutType<K,V> {
        LutType::PathOnly(path)
    }

    pub fn load_from_property_value(property_value: String, data_source: &dyn DataInterface) -> Result<LutType<K, V>, String>{
        return match property_value.starts_with("(") {
            true => {
                Ok(LutType::Inline(InlineLut::from_property_value(property_value)?))
            }
            false => {
                let data = match data_source.get_original_file_data(&property_value) {
                    Ok(data_option) => {
                        match data_option {
                            None => Err(format!("Failed to load {} from data source. No such file", property_value)),
                            Some(data) => Ok(data)
                        }
                    }
                    Err(e) => {
                        Err(format!("Failed to load {} from data source. {}", property_value, e.to_string()))
                    }
                }?;
                let lut_vec = load_lut_from_bytes(&data).map_err(|err| {
                    format!("Failed to parse lut from {} in data source. {}", property_value, err.to_string())
                })?;
                Ok(LutType::File(LutFile::new(property_value, lut_vec)))
            }
        }
    }
}

#[derive(Debug)]
pub struct LutInterpolator<K, V> {
    data: Vec<(K, V)>,
}

impl<K, V> LutInterpolator<K, V>
where
    K: Into<f64> + PartialOrd + Copy + FromStr + Display + std::ops::Sub<Output=K> + std::ops::Div<Output=K>, <K as FromStr>::Err: fmt::Debug,
    V: Into<f64> + Copy + FromStr + Display, <V as FromStr>::Err: fmt::Debug,
{
    pub fn from_lut(lut: &LutType<K, V>) -> LutInterpolator<K,V> {
        let data = match lut {
            LutType::File(l) => { l.to_vec() }
            LutType::Inline(l) => { l.to_vec() }
            LutType::PathOnly(_) => { Vec::new() }
        };
        LutInterpolator { data }
    }

    pub fn from_vec(data: Vec<(K, V)>) -> LutInterpolator<K,V> {
        LutInterpolator { data }
    }

    pub fn get_value(&self, key: K) -> Option<f64> {
        if self.data.is_empty() {
            return None;
        }

        if key < self.data[0].0 || key > self.data[self.data.len() - 1].0 {
            return None;
        }

        // Binary search for the correct position
        let mut low = 0;
        let mut high = self.data.len() - 1;

        while low <= high {
            let mid = low + (high - low) / 2;

            match self.data[mid].0.partial_cmp(&key) {
                Some(Ordering::Less) => low = mid + 1,
                Some(Ordering::Greater) => {
                    if mid == 0 {
                        break;
                    }
                    high = mid - 1;
                }
                Some(Ordering::Equal) => return Some(self.data[mid].1.into()),
                None => return None, // Handle NaN or other invalid comparisons
            }
        }

        // At this point, low is the index where the key would be inserted
        let (k1, v1) = self.data[low - 1];
        let (k2, v2) = self.data[low];

        // Linear interpolation formula
        let fraction :f64 = (key - k1).into() / (k2 - k1).into();
        let interpolated_value = v1.into() + fraction * (v2.into() - v1.into());

        Some(interpolated_value)
    }
}

pub fn load_lut_from_path<K, V>(lut_path: &Path) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
{
    let file = match File::open(lut_path) {
        Ok(file) => { file }
        Err(e) => {
            return Err(format!("Failed to open {}: {}", lut_path.display(), e.to_string()));
        }
    };
    load_lut_from_reader(&file, b'|', Terminator::CRLF)
}

pub fn load_lut_from_bytes<K, V>(lut_bytes: &Vec<u8>) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
{
    load_lut_from_reader(Cursor::new(lut_bytes), b'|', Terminator::CRLF)
}


pub fn load_lut_from_reader<K, V, R>(lut_reader: R, delimiter: u8, terminator: Terminator) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug,
        R: io::Read
{
    let mut lut_data: Vec<(K, V)> = Vec::new();
    let mut rdr = csv::ReaderBuilder::new().has_headers(false).delimiter(delimiter).terminator(terminator).comment(Some(b';')).from_reader(lut_reader);
    for result in rdr.records() {
        match result {
            Ok(record) => {
                let key: K = parse_lut_element(&record, 0)?;
                let value: V = parse_lut_element(&record, 1)?;
                lut_data.push((key, value));
            },
            _ => {}
        }
    }
    Ok(lut_data)
}

fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

pub fn load_lut_from_property_value<K, V>(property_value: String, data_source: &dyn DataInterface) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr + Display + Clone, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display + Clone, <V as FromStr>::Err: fmt::Debug
{
    return match property_value.starts_with("(") {
        true => {
            let data_slice = &property_value[1..(property_value.len() - 1)];
            load_lut_from_reader::<K, V, _>(data_slice.as_bytes(), b'=', Terminator::Any(b'|'))
        }
        false => {
            let file_data = match data_source.get_original_file_data(&property_value) {
                Ok(data_option) => match data_option {
                    None => Err(format!("Failed to load {} from data source. No such file", property_value)),
                    Some(data) => Ok(data)
                }
                Err(e) => {
                    Err(format!("Failed to load {} from data source. {}", property_value, e.to_string()))
                }
            }?;
            load_lut_from_bytes::<K, V>(&file_data)
        }
    }
}

pub fn parse_lut_element<T>(record: &csv::StringRecord, index: usize) -> Result<T, String>
    where
        T: std::str::FromStr + Display, <T as FromStr>::Err: fmt::Debug
{
    let record_opt = record.get(index);
    if record_opt.is_none() {
        return Err(format!("Cannot access index {} of lut", index));
    }
    match remove_whitespace(record_opt.unwrap()).parse::<T>() {
        Ok(s) => { Ok(s) },
        Err(e) => {
            let mut err_str = String::from("Invalid lut types, Cannot convert first item");
            if let Some(pos) = record.position() {
                err_str.push_str(&format!(" at line {}", pos.line()));
            }
            return Err(format!("{} to {}. {:?}", err_str, std::any::type_name::<T>(), e))
        }
    }
}

pub fn write_lut_to_bytes<K,V>(data: &Vec<(K,V)>) -> Result<Vec<u8>, String>
    where
        K: std::fmt::Display,
        V: std::fmt::Display
{
    let mut out : Vec<u8> = Vec::new();
    {
        let mut writer = csv::WriterBuilder::new().has_headers(false).delimiter(b'|').from_writer(&mut out);
        for (key, val) in data {
            writer.write_record(&[key.to_string(), val.to_string()]).map_err(|err| {
                format!("Couldn't write lut to buffer. {}", err.to_string())
            })?;
        }
    }
    Ok(out)
}

pub fn write_lut_to_path<K, V>(data: &Vec<(K,V)>, path: &Path) -> Result<(), String>
    where
      K: std::fmt::Display,
      V: std::fmt::Display
{
    let mut writer = csv::WriterBuilder::new().has_headers(false).delimiter(b'|').from_path(path).map_err(
        |err| { format!("Couldn't write {}. {}", path.to_path_buf().display(), err.to_string()) }
    )?;
    for (key, val) in data {
        writer.write_record(&[key.to_string(), val.to_string()]).map_err(|err| {
            format!("Couldn't write {}. {}", path.to_path_buf().display(), err.to_string())
        })?;
    }
    writer.flush().map_err(
        |err| { format!("Couldn't write {}. {}", path.to_path_buf().display(), err.to_string()) }
    )?;
    Ok(())
}

pub fn write_lut_to_property_value<K, V>(data: &Vec<(K,V)>, delimiter: u8, terminator: Terminator) -> Result<String, String>
    where
      K: std::fmt::Display,
      V: std::fmt::Display
{
    let mut out : Vec<u8> = Vec::new();
    out.push(b'(');
    {
        let mut writer = csv::WriterBuilder::new().has_headers(false).delimiter(delimiter).terminator(terminator).flexible(true).from_writer(&mut out);
        for (key, val) in data {
            writer.write_record(&[key.to_string(), val.to_string()]).map_err(|err| {
                err.to_string()
            })?;
        }
    }
    out.pop();
    out.push(b')');
    return match std::str::from_utf8(&out) {
        Ok(v) => Ok(v.to_owned()),
        Err(e) => Err(e.to_string()),
    };
}

#[test]
fn interpolator_test() {
    let data = vec![
        (1.0, 2.0),
        (2.0, 4.0),
        (3.0, 6.0),
        (4.0, 8.0),
        (5.0, 10.0),
    ];

    let interpolator = LutInterpolator {data};

    // Test interpolation
    let test_keys =
        vec![0.5,  1.0,       1.5,       2.5,       3.5,       4.5,       5.0,        5.5];
    let expected_vals =
        vec![None, Some(2.0), Some(3.0), Some(5.0), Some(7.0), Some(9.0), Some(10.0), None];
    for (idx, key) in test_keys.iter().enumerate() {
        assert_eq!(interpolator.get_value(*key), expected_vals[idx]);
    }
}
// fn load_lut_string() {
//     let data = String::from("(0=0.12|0.97=13|1=0.40)");
//     let vec: Vec<(f64, f64)> = load_lut_from_property_value(data, Path::new("")).unwrap();
//     println!("{:?}", vec);
//     let out = write_lut_to_property_value(&vec, b'=', Terminator::Any(b'|')).unwrap();
//     println!("{}", out);
// }
