use std::{fmt, io};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use csv::Terminator;

#[derive(Debug)]
pub struct LutFile<K, V>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    pub path: PathBuf,
    data: Vec<(K,V)>
}

impl<K, V> LutFile<K, V>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    pub fn new(path: &Path, data: Vec<(K, V)>) -> LutFile<K, V> {
        LutFile {
            path: path.to_path_buf(),
            data
        }
    }

    pub fn from_path(lut_path: &Path) -> Result<LutFile<K, V>, String> {
        Ok(LutFile {
            path: lut_path.to_path_buf(),
            data: load_lut_from_path::<K, V>(lut_path)?
        })
    }
    
    pub fn update(&mut self, data: Vec<(K,V)>) -> Vec<(K,V)> {
        std::mem::replace(&mut self.data, data)
    }

    pub fn write(&self) -> Result<(), String> {
        write_lut_to_path(&self.data, self.path.as_path())?;
        Ok(())
    }

    pub fn to_vec(&self) -> Vec<(K,V)> {
        self.data.clone()
    }
}

#[derive(Debug)]
pub struct InlineLut<K, V>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    data: Vec<(K,V)>
}

impl<K, V> InlineLut<K, V>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug,
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
}

impl<K, V> Display for InlineLut<K, V>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}",
               write_lut_to_property_value(&self.data, b'=', Terminator::Any(b'|')).map_err(
                   |err| { fmt::Error }
               )?)
    }
}


#[derive(Debug)]
pub enum LutType<K, V>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    File(LutFile<K,V>),
    Inline(InlineLut<K,V>),
    PathOnly(PathBuf)
}

impl<K, V> LutType<K, V>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug,
        (K, V): Clone
{
    pub fn path_only(path: PathBuf) -> LutType<K,V> {
        LutType::PathOnly(path)
    }

    pub fn load_from_property_value(property_value: String, data_dir: &Path) -> Result<LutType<K, V>, String>{
        return match property_value.starts_with("(") {
            true => {
                Ok(LutType::Inline(InlineLut::from_property_value(property_value)?))
            }
            false => {
                Ok(LutType::File(LutFile::from_path(data_dir.join(property_value.as_str()).as_path())?))
            }
        }
    }
}

pub fn load_lut_from_path<K, V>(lut_path: &Path) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug,
{
    let file = match File::open(lut_path) {
        Ok(file) => { file }
        Err(e) => {
            return Err(format!("Failed to open {}: {}", lut_path.display(), e.to_string()));
        }
    };
    load_lut_from_reader(&file, b'|', Terminator::CRLF)
}

pub fn load_lut_from_reader<K, V, R>(lut_reader: R, delimiter: u8, terminator: Terminator) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug,
        R: io::Read
{
    let mut lut_data: Vec<(K, V)> = Vec::new();
    let mut rdr = csv::ReaderBuilder::new().has_headers(false).delimiter(delimiter).terminator(terminator).from_reader(lut_reader);
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

pub fn load_lut_from_property_value<K, V>(property_value: String, data_dir: &Path) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr + Display, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr + Display, <V as FromStr>::Err: fmt::Debug
{
    return match property_value.starts_with("(") {
        true => {
            let data_slice = &property_value[1..(property_value.len() - 1)];
            load_lut_from_reader::<K, V, _>(data_slice.as_bytes(), b'=', Terminator::Any(b'|'))
        }
        false => {
            load_lut_from_path::<K, V>(data_dir.join(property_value.as_str()).as_path()) 
        }
    }
}

pub fn parse_lut_element<T>(record: &csv::StringRecord, index: usize) -> Result<T, String>
    where
        T: std::str::FromStr + Display, <T as FromStr>::Err: fmt::Debug
{
    match remove_whitespace(record.get(index).unwrap()).parse::<T>() {
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
fn load_lut_string() {
    let data = String::from("(0=0.12|0.97=13|1=0.40)");
    let vec: Vec<(f64, f64)> = load_lut_from_property_value(data, Path::new("")).unwrap();
    println!("{:?}", vec);
    let out = write_lut_to_property_value(&vec, b'=', Terminator::Any(b'|')).unwrap();
    println!("{}", out);
}
