use std::fmt::Debug;
use std::io;
use std::path::Path;
use crate::assetto_corsa::ini_utils::Ini;

pub trait CarDataFile
{
    fn ini_data(&self) -> &Ini;
    fn data_interface(&self) -> &dyn DataInterface;
}

pub trait MandatoryDataSection {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> crate::assetto_corsa::error::Result<Self> where Self: Sized;
}

pub trait OptionalDataSection {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> crate::assetto_corsa::error::Result<Option<Self>> where Self: Sized;
}

pub fn extract_mandatory_section<T: MandatoryDataSection>(car_data: &dyn CarDataFile) -> crate::assetto_corsa::error::Result<T> {
    T::load_from_parent(car_data)
}

pub fn extract_optional_section<T: OptionalDataSection>(car_data: &dyn CarDataFile) -> crate::assetto_corsa::error::Result<Option<T>> {
    T::load_from_parent(car_data)
}

pub trait _DataInterfaceI {
    fn load(&self);
    fn get_file_data(&self, filename: &str) -> io::Result<Vec<u8>>;
    fn write_file_data(&mut self, filename: &str, data: Vec<u8>) -> io::Result<()>;
    fn delete_file(&mut self, filename: &str) -> io::Result<()>;
}

pub trait DataInterface: _DataInterfaceI + Debug {}
