/*
 * Copyright (c):
 * 2023 zephyrj
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

pub mod tyre_sets;

use crate::assetto_corsa::{Car, ini_utils};
use crate::assetto_corsa::error::{Result, Error, ErrorKind};
use crate::assetto_corsa::ini_utils::Ini;
use crate::assetto_corsa::traits::{CarDataFile, DataInterface};

#[derive(Debug)]
pub struct Tyres<'a> {
    car: &'a mut Car,
    ini_data: Ini,
}

impl<'a> Tyres<'a> {
    const INI_FILENAME: &'static str = "tyres.ini";

    pub fn from_car(car: &'a mut Car) -> Result<Tyres<'a>> {
        let file_data = match car.data_interface.get_original_file_data(Tyres::INI_FILENAME) {
            Ok(data_option) => {
                match data_option {
                    None => Err(Error::new(ErrorKind::InvalidCar, format!("missing {} data", Self::INI_FILENAME))),
                    Some(data) => Ok(data)
                }
            }
            Err(e) => {
                Err(Error::new(ErrorKind::InvalidCar, format!("error reading {} data. {}", Self::INI_FILENAME, e.to_string())))
            }
        }?;
        Ok(Tyres {
            car,
            ini_data: Ini::load_from_string(String::from_utf8_lossy(file_data.as_slice()).into_owned())
        })
    }

    pub fn write(&mut self) -> Result<()> {
        let data_interface = self.car.mut_data_interface();
        data_interface.update_file_data(Tyres::INI_FILENAME,
                                        self.ini_data.to_bytes());
        data_interface.write()?;
        Ok(())
    }
}

impl<'a> CarDataFile for Tyres<'a> {
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

fn get_mandatory_field<T: std::str::FromStr>(ini_data: &Ini, section_name: &str, key: &str) -> Result<T> {
    let res: T = match ini_utils::get_value(ini_data, section_name, key) {
        Some(val) => val,
        None => { return Err(mandatory_field_error(section_name, key)); }
    };
    Ok(res)
}

fn mandatory_field_error(section: &str, key: &str) -> Error {
    return Error::new(
        ErrorKind::InvalidCar,
        format!("Missing {}.{} in {}", section, key, Tyres::INI_FILENAME)
    )
}
