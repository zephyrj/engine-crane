use crate::assetto_corsa::car::structs::LutProperty;
use crate::assetto_corsa::traits::{CarDataFile, CarDataUpdater, MandatoryDataSection};
use crate::assetto_corsa::error::{Result, Error, ErrorKind};

#[derive(Debug)]
pub struct PowerCurve {
    power_lut: LutProperty<i32, i32>,
}

impl PowerCurve {
    pub fn update(&mut self, power_vec: Vec<(i32, i32)>) -> Result<Vec<(i32, i32)>> {
        Ok(self.power_lut.update(power_vec))
    }
}

impl MandatoryDataSection for PowerCurve {
    fn load_from_parent(parent_data: &dyn CarDataFile) -> Result<Self> where Self: Sized {
        Ok(PowerCurve{
            power_lut: LutProperty::mandatory_from_ini(
                String::from("HEADER"),
                String::from("POWER_CURVE"),
                parent_data.ini_data(),
                parent_data.data_interface()).map_err(|err|{
                Error::new(ErrorKind::InvalidCar, format!("Cannot find a lut for power curve. {}", err.to_string()))
            })?
        })
    }
}

impl CarDataUpdater for PowerCurve {
    fn update_car_data(&self, car_data: &mut dyn CarDataFile) -> Result<()> {
        self.power_lut.update_car_data(car_data)?;
        Ok(())
    }
}
