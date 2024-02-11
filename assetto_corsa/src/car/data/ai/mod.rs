/*
Copyright (c):
2023 zephyrj
zephyrj@protonmail.com

This file is part of engine-crane.

engine-crane is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

engine-crane is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
*/

pub mod gears;
pub use gears::Gears;

use crate::car::Car;
use crate::ini_utils::Ini;
use crate::error::Result;
use crate::traits::{CarDataFile, DataInterface};


pub const INI_FILENAME: &'static str = "ai.ini";

#[derive(Debug)]
pub struct Ai<'a> {
    car: &'a mut Car,
    ini_data: Ini,
}

impl<'a> Ai<'a> {
    pub const INI_FILENAME: &'static str = INI_FILENAME;

    pub fn from_car(car: &'a mut Car) -> Result<Option<Ai<'a>>> {
        match car.data_interface.get_original_file_data(Ai::INI_FILENAME)? {
            None => Ok(None),
            Some(file_data) => {
                Ok(Some(Ai {
                    car,
                    ini_data: Ini::load_from_string(String::from_utf8_lossy(file_data.as_slice()).into_owned())
                }))
            }
        }
    }

    pub fn write(&mut self) -> Result<()> {
        let data_interface = self.car.mut_data_interface();
        data_interface.update_file_data(Ai::INI_FILENAME,
                                        self.ini_data.to_bytes());
        data_interface.write()?;
        Ok(())
    }
}

impl<'a> CarDataFile for Ai<'a> {
    fn ini_data(&self) -> &Ini {
        &self.ini_data
    }
    fn mut_ini_data(&mut self) -> &mut Ini {
        &mut self.ini_data
    }
    fn data_interface(&self) -> &dyn DataInterface {
        self.car.data_interface()
    }
    fn mut_data_interface(&mut self) -> &mut dyn DataInterface {
        self.car.mut_data_interface()
    }
}
