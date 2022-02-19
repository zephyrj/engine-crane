use std::cell::RefCell;
use std::io;
use std::path::Path;
use std::rc::Rc;
use crate::assetto_corsa::ini_utils::Ini;


pub fn load_ini_file(ini_path: &Path) -> io::Result<Ini> {
    Ini::load_from_file(ini_path)
}

pub fn load_ini_file_rc(ini_path: &Path) -> io::Result<Rc<RefCell<Ini>>> {
    Ok(Rc::new(RefCell::new(load_ini_file(ini_path)?)))
}
