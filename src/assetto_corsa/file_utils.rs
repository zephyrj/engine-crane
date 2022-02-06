use std::cell::RefCell;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::ops::Add;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
use configparser::ini::Ini;
use crate::assetto_corsa::lut_utils::{parse_lut_element, load_lut_from_reader};


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
