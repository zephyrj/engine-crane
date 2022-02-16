use std::cell::RefCell;
use std::ffi::OsStr;
use std::{fmt, io};
use std::fs::File;
use std::ops::Add;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
use crate::assetto_corsa::ini_utils::Ini;
use crate::assetto_corsa::lut_utils::{parse_lut_element, load_lut_from_reader};


pub fn load_ini_file(ini_path: &Path) -> io::Result<Ini> {
    Ini::load_from_file(ini_path)
}

pub fn load_ini_file_rc(ini_path: &Path) -> io::Result<Rc<RefCell<Ini>>> {
    Ok(Rc::new(RefCell::new(load_ini_file(ini_path)?)))
}
