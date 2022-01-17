use std::cell::RefCell;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::ops::Add;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
use configparser::ini::Ini;


pub fn load_ini_file(ini_path: &Path) -> Result<Ini, String> {
    let mut ini = Ini::new();
    match ini.load(ini_path) {
        Err(err_str) =>  {
            Err(format!("Failed to decode {}: {}", ini_path.display(), err_str))
        },
        _ => {
            Ok(ini)
        }
    }
}

pub fn load_ini_file_rc(ini_path: &Path) -> Result<Rc<RefCell<Ini>>, String> {
    let mut ini = Rc::new(RefCell::new(Ini::new()));
    match ini.borrow_mut().load(ini_path) {
        Err(err_str) =>  {
            return Err(format!("Failed to decode {}: {}", ini_path.display(), err_str));
        },
        _ => {}
    };
    Ok(ini)
}


pub fn load_lut<K, V>(lut_path: &Path) -> Result<Vec<(K, V)>, String>
where
    K: std::str::FromStr, <K as FromStr>::Err: fmt::Debug,
    V: std::str::FromStr, <V as FromStr>::Err: fmt::Debug
{
    let file = match File::open(lut_path) {
        Ok(file) => { file }
        Err(e) => {
            return Err(format!("Failed to open {}: {}", lut_path.display(), e.to_string()));
        }
    };

    let mut lut_data: Vec<(K, V)> = Vec::new();
    let mut rdr = csv::ReaderBuilder::new().has_headers(false).delimiter(b'|').from_reader(file);
    for result in rdr.records() {
        match result {
            Ok(record) => {
                lut_data.push((parse_lut_element(&record, 0)?,
                               parse_lut_element(&record, 1)?));
            },
            _ => {}
        }
    }
    Ok(lut_data)
}

fn parse_lut_element<T>(record: &csv::StringRecord, index: usize) -> Result<T, String>
where
    T: std::str::FromStr, <T as FromStr>::Err: fmt::Debug
{
    match record.get(index).unwrap().parse::<T>() {
        Ok(s) => { Ok(s) },
        Err(e) => {
            let mut err_str = String::from("Invalid lut types, Cannot convert first item");
            if let Some(pos) = record.position() {
                err_str.push_str(&format!(" at line {}", pos.line()));
            }
            return Err(format!("{} to {}",
                               err_str, std::any::type_name::<T>()))
        }
    }
}