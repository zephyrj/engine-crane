use std::cell::RefCell;
use std::ffi::OsString;
use std::path::Path;
use std::rc::Rc;
use configparser::ini::Ini;
use crate::assetto_corsa::error::{Result, Error, ErrorKind};
use crate::assetto_corsa::file_utils::load_ini_file_rc;


#[derive(Debug)]
pub struct Drivetrain {
    data_dir: OsString,
    ini_data: Rc<RefCell<Ini>>
}

impl Drivetrain {
    const INI_FILENAME: &'static str = "drivetrain.ini";

    pub fn load_from_path(data_dir: &Path) -> Result<Drivetrain> {
        let ini_data = match load_ini_file_rc(data_dir.join(Drivetrain::INI_FILENAME).as_path()) {
            Ok(ini_object) => { ini_object }
            Err(err_str) => {
                return Err(Error::new(ErrorKind::InvalidCar, err_str ));
            }
        };
        Ok(Drivetrain {
            data_dir: OsString::from(data_dir),
            ini_data
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use crate::assetto_corsa::drivetrain::Drivetrain;
    use crate::assetto_corsa::engine::Engine;

    #[test]
    fn load_drivetrain() -> Result<(), String> {
        let path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/a1_science_car/data");
        match Drivetrain::load_from_path(&path) {
            Ok(_) => {
                Ok(())
            }
            Err(e) => { Err(e.to_string()) }
        }
    }
}