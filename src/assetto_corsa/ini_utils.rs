use std::cell::RefCell;
use std::error;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::rc::Weak;
use configparser::ini::Ini;

#[derive(Debug)]
pub struct FieldTypeError {
    section_name: String,
    field_name: String,
    expected_type: String
}

impl FieldTypeError {
    pub fn new(section_name: &str, field_name: &str, expected_type: &str) -> FieldTypeError {
        FieldTypeError {
            section_name: String::from(section_name),
            field_name: String::from(field_name),
            expected_type: String::from(expected_type)
        }
    }
}

impl Display for FieldTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expected {}-{} to be {} type",
               &self.section_name,
               &self.field_name,
               &self.expected_type)
    }
}

impl error::Error for FieldTypeError {}

pub fn get_value_from_weak_ref<T: std::str::FromStr>(ini_data: &Weak<RefCell<Ini>>,
                                                     section: &str,
                                                     key: &str) -> Option<T> {
    let ini = ini_data.upgrade()?;
    let ini_ref = ini.borrow();
    get_value(ini_ref.deref(), section, key)
}

pub fn get_value<T: std::str::FromStr>(ini: &Ini,
                                       section: &str,
                                       key: &str) -> Option<T> {
    let item = ini.get(section, key)?;
    match item.parse::<T>() {
        Ok(val) => { Some(val) }
        Err(_) => { None }
    }
}
