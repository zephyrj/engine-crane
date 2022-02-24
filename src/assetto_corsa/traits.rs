use std::path::Path;
use crate::assetto_corsa::ini_utils::Ini;

pub trait CarIniData
{
    fn ini_data(&self) -> &Ini;
    fn data_dir(&self) -> &Path;
}

pub trait MandatoryCarData: CarIniData {
    fn load_from_path(data_dir: &Path) -> crate::assetto_corsa::error::Result<Self> where Self: Sized;
}

pub trait OptionalCarData: CarIniData {
    fn load_from_path(data_dir: &Path) -> crate::assetto_corsa::error::Result<Option<Self>> where Self: Sized;
}

pub trait MandatoryDataComponent {
    fn load_from_parent(parent_data: &dyn CarIniData) -> crate::assetto_corsa::error::Result<Self> where Self: Sized;
}

pub trait OptionalDataComponent {
    fn load_from_parent(parent_data: &dyn CarIniData) -> crate::assetto_corsa::error::Result<Option<Self>> where Self: Sized;
}

pub fn extract_mandatory_component<T: MandatoryDataComponent>(car_data: &dyn CarIniData) -> crate::assetto_corsa::error::Result<T> {
    T::load_from_parent(car_data)
}

pub fn extract_optional_component<T: OptionalDataComponent>(car_data: &dyn CarIniData) -> crate::assetto_corsa::error::Result<Option<T>> {
    T::load_from_parent(car_data)
}
