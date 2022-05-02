pub mod shift_lights;

use crate::assetto_corsa::car::Car;
use crate::assetto_corsa::ini_utils::Ini;
use crate::assetto_corsa::error::{Result, Error, ErrorKind};
use crate::assetto_corsa::traits::{CarDataFile, DataInterface};

#[derive(Debug)]
pub struct DigitalInstruments<'a> {
    car: &'a mut Car,
    ini_data: Ini,
}

impl<'a> DigitalInstruments<'a> {
    pub const INI_FILENAME: &'static str = "digital_instruments.ini";

    pub fn from_car(car: &'a mut Car) -> Result<Option<DigitalInstruments<'a>>> {
        match car.data_interface.get_file_data(DigitalInstruments::INI_FILENAME)? {
            None => Ok(None),
            Some(file_data) => {
                Ok(Some(DigitalInstruments {
                    car,
                    ini_data: Ini::load_from_string(String::from_utf8_lossy(file_data.as_slice()).into_owned())
                }))
            }
        }
    }

    pub fn write(&mut self) -> Result<()> {
        let data_interface = self.car.mut_data_interface();
        data_interface.update_file_data(DigitalInstruments::INI_FILENAME,
                                        self.ini_data.to_bytes());
        data_interface.write()?;
        Ok(())
    }
}

impl<'a> CarDataFile for DigitalInstruments<'a> {
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

