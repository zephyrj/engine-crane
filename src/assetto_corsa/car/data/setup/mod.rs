use std::fmt::{Display, Formatter};
use std::str::FromStr;
use crate::assetto_corsa::Car;
use crate::assetto_corsa::error::{Result, Error, ErrorKind};
use crate::assetto_corsa::ini_utils::Ini;
use crate::assetto_corsa::traits::{CarDataFile, DataInterface};

pub mod gears;

#[derive(Debug)]
pub enum HelpData {
    Empty,
    Id(String),
    Text(String)
}

impl FromStr for HelpData {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(HelpData::Empty);
        }
        if s.starts_with("\"") || s.starts_with("\'") {
            return Ok(HelpData::Text(String::from(&s[1..s.len()-1])))
        }
        return Ok(HelpData::Id(String::from(s)));
    }
}

impl Display for HelpData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HelpData::Empty => write!(f, ""),
            HelpData::Id(id_str) => write!(f, "{}", id_str),
            HelpData::Text(text) => write!(f, "\"{}\"", text)
        }
    }
}


pub const INI_FILENAME: &'static str = "setup.ini";

#[derive(Debug)]
pub struct Setup<'a> {
    car: &'a mut Car,
    ini_data: Ini,
}

impl<'a> Setup<'a> {
    pub const INI_FILENAME: &'static str = INI_FILENAME;

    pub fn from_car(car: &'a mut Car) -> Result<Option<Setup<'a>>> {
        match car.data_interface.get_original_file_data(Setup::INI_FILENAME)? {
            None => Ok(None),
            Some(file_data) => {
                Ok(Some(Setup {
                    car,
                    ini_data: Ini::load_from_string(String::from_utf8_lossy(file_data.as_slice()).into_owned())
                }))
            }
        }
    }

    pub fn write(&mut self) -> Result<()> {
        let data_interface = self.car.mut_data_interface();
        data_interface.update_file_data(Setup::INI_FILENAME,
                                        self.ini_data.to_bytes());
        data_interface.write()?;
        Ok(())
    }
}

impl<'a> CarDataFile for Setup<'a> {
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

fn mandatory_field_error(section: &str, key: &str) -> Error {
    return Error::new(
        ErrorKind::InvalidCar,
        format!("Missing {}.{} in {}", section, key, Setup::INI_FILENAME)
    )
}
