/*
 * Copyright (c):
 * 2025 zephyrj
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

use std::fmt::Debug;
use std::io;
use crate::car::acd_utils::AcdError;
use crate::ini_utils::Ini;
use thiserror::Error;

pub trait CarDataFile
{
    fn ini_data(&self) -> &Ini;
    fn mut_ini_data(&mut self) -> &mut Ini;
    fn data_interface(&self) -> &dyn DataInterface;
    fn mut_data_interface(&mut self) -> &mut dyn DataInterface;
}

pub trait CarDataUpdater {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> crate::error::Result<()>;
}

pub trait MandatoryDataSection {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> crate::error::Result<Self> where Self: Sized;
}

pub trait OptionalDataSection {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> crate::error::Result<Option<Self>> where Self: Sized;
}

pub fn extract_mandatory_section<T: MandatoryDataSection>(car_data: &dyn CarDataFile) -> crate::error::Result<T> {
    T::load_from_parent(car_data)
}

pub fn extract_optional_section<T: OptionalDataSection>(car_data: &dyn CarDataFile) -> crate::error::Result<Option<T>> {
    T::load_from_parent(car_data)
}

pub fn update_car_data<T: CarDataFile, S: CarDataUpdater>(car_data: &mut T, car_data_updater: &S) -> crate::error::Result<()> {
    car_data_updater.update_car_data(car_data)
}

pub type DataInterfaceResult<T> = std::result::Result<T, DataInterfaceError>;

#[derive(Error, Debug)]
pub enum DataInterfaceError {
    #[error("io error")]
    IoError {
        #[from]
        source: io::Error
    },
    #[error("acd error")]
    AcdError {
        #[from]
        source: AcdError
    }
}

pub trait _DataInterfaceI {
    fn get_original_file_data(&self, filename: &str) -> DataInterfaceResult<Option<Vec<u8>>>;
    fn contains_file(&self, filename: &str) -> bool;
    fn update_file_data(&mut self, filename: &str, data: Vec<u8>);
    fn remove_file(&mut self, filename: &str);
    fn write(&mut self) -> DataInterfaceResult<()>;
}

pub trait DataInterface: _DataInterfaceI + Debug {}
