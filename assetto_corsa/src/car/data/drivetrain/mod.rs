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

pub mod traction;
pub mod gearbox;
pub mod clutch;
pub mod differential;
pub mod auto_clutch;
pub mod auto_blip;
pub mod auto_shifter;
pub mod downshift_protection;

pub use traction::Traction;
pub use gearbox::Gearbox;
pub use clutch::Clutch;
pub use differential::Differential;
pub use auto_clutch::AutoClutch;
pub use auto_blip::AutoBlip;
pub use auto_shifter::AutoShifter;
pub use downshift_protection::DownshiftProtection;

use crate::error::{Result, Error, ErrorKind};
use crate::{ini_utils};
use crate::car::Car;
use crate::ini_utils::Ini;
use crate::traits::{CarDataFile, DataInterface};


#[derive(Debug)]
pub struct Drivetrain<'a> {
    car: &'a mut Car,
    ini_data: Ini,
}

impl<'a> Drivetrain<'a> {
    pub const INI_FILENAME: &'static str = "drivetrain.ini";

    pub fn from_car(car: &'a mut Car) -> Result<Drivetrain<'a>> {
        let file_data = match car.data_interface.get_original_file_data(Drivetrain::INI_FILENAME) {
            Ok(data_option) => {
                match data_option {
                    None => Err(Error::new(ErrorKind::InvalidCar, format!("missing {} data", Drivetrain::INI_FILENAME))),
                    Some(data) => Ok(data)
                }
            }
            Err(e) => {
                Err(Error::new(ErrorKind::InvalidCar, format!("error reading {} data. {}", Drivetrain::INI_FILENAME, e.to_string())))
            }
        }?;
        Ok(Drivetrain {
            car,
            ini_data: Ini::load_from_string(String::from_utf8_lossy(file_data.as_slice()).into_owned())
        })
    }

    // pub fn load_from_data(data: &(dyn DebuggableDataInterface)) -> Result<Self> where Self: Sized {
    //     let file_data = data.get_file_data(Drivetrain::INI_FILENAME)?;
    //     Ok(Drivetrain {
    //         ini_data: Ini::load_from_string(String::from_utf8_lossy(file_data.as_slice()).into_owned())
    //     })
    // }
    //
    // pub fn load_from_ini_string(ini_data: String) -> Drivetrain {
    //     Drivetrain {
    //         ini_data: Ini::load_from_string(ini_data)
    //     }
    // }
    //
    // pub fn load_from_file(ini_path: &Path) -> Result<Drivetrain> {
    //     let ini_data = match load_ini_file(ini_path) {
    //         Ok(ini_object) => { ini_object }
    //         Err(err) => {
    //             return Err(Error::new(ErrorKind::InvalidCar, err.to_string() ));
    //         }
    //     };
    //     Ok(Drivetrain {
    //         ini_data
    //     })
    // }

    pub fn write(&mut self) -> Result<()> {
        let data_interface = self.car.mut_data_interface();
        data_interface.update_file_data(Drivetrain::INI_FILENAME,
                                        self.ini_data.to_bytes());
        data_interface.write()?;
        Ok(())
    }
}

impl<'a> CarDataFile for Drivetrain<'a> {
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
        format!("Missing {}.{} in {}", section, key, Drivetrain::INI_FILENAME)
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use crate::Car;
    use crate::car::data::drivetrain::{Drivetrain, Gearbox, Traction};
    use crate::car::data::drivetrain::traction::DriveType;
    use crate::traits::{CarDataUpdater, MandatoryDataSection};

    const TEST_DATA_PATH: &'static str = "test_data";
    const TEMP_TEST_CAR_NAME_PREFIX: &'static str = "tmp_car";

    struct TestFileHelper {
        test_name: String
    }

    impl TestFileHelper {
        pub fn new(test_name: String) -> TestFileHelper {
            TestFileHelper{test_name}
        }

        fn get_tmp_car_folder(&self) -> String {
            format!("{}_{}", TEMP_TEST_CAR_NAME_PREFIX, self.test_name)
        }

        fn get_test_car_path(&self, car_name: &str) -> PathBuf {
            let mut test_folder_path = PathBuf::from(Path::new(file!()).parent().unwrap());
            test_folder_path.push(format!("{}/{}", TEST_DATA_PATH, car_name));
            test_folder_path
        }

        fn load_test_car(&self, test_car_name: &str) -> Car {
            Car::load_from_path(&self.get_test_car_path(test_car_name)).unwrap()
        }

        fn load_tmp_car(&self) -> Car {
            self.load_test_car(&self.get_tmp_car_folder())
        }

        fn delete_tmp_car(&self) {
            let tmp_car_path = self.get_test_car_path(&self.get_tmp_car_folder());
            if tmp_car_path.exists() {
                std::fs::remove_dir_all(tmp_car_path).unwrap();
            }
        }

        fn create_tmp_car(&self) -> Car {
            self.delete_tmp_car();
            Car::new(self.get_test_car_path(&self.get_tmp_car_folder())).unwrap()
        }

        fn setup_tmp_car_as(&self, test_car_name: &str) -> Car {
            self.create_tmp_car();
            let mut copy_options = fs_extra::dir::CopyOptions::new();
            copy_options.content_only = true;
            fs_extra::dir::copy(self.get_test_car_path(test_car_name),
                                self.get_test_car_path(&self.get_tmp_car_folder()),
                                &copy_options).unwrap();
            Car::load_from_path(&self.get_test_car_path(&self.get_tmp_car_folder())).unwrap()
        }
    }


    // fn subcomponent_update_test<T: IniUpdater + MandatoryDataSection, F: FnOnce(&mut T)>(component_update_fn: F) -> Result<String, String> {
    //     let mut drivetrain = Drivetrain::load_from_ini_string(String::from(TEST_DATA));
    //     let mut component = extract_mandatory_section::<T>(&drivetrain).unwrap();
    //     component_update_fn(&mut component);
    //     drivetrain.update_subcomponent(&component).map_err(|err| format!("{}", err.to_string()))?;
    //     Ok(drivetrain.ini_data.to_string())
    // }
    //
    // fn validate_subcomponent<T, F>(ini_string: String, component_validation_fn: F) -> Result<(), String>
    //     where T: MandatoryDataSection,
    //           F: FnOnce(&T)
    // {
    //     let drivetrain = Drivetrain::load_from_ini_string(ini_string);
    //     let component = extract_mandatory_section::<T>(&drivetrain).map_err(|err| format!("{}", err.to_string()))?;
    //     component_validation_fn(&component);
    //     Ok(())
    // }

    #[test]
    fn update_traction() -> Result<(), String> {
        let update_drive_type = DriveType::FWD;
        let helper = TestFileHelper::new("update_traction".to_string());
        {
            let mut car = helper.setup_tmp_car_as("six-gears");
            let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
            let mut traction_data = Traction::load_from_parent(&mut drivetrain).unwrap();
            assert_eq!(traction_data.drive_type, DriveType::RWD);

            traction_data.drive_type = update_drive_type;
            assert!(traction_data.update_car_data(&mut drivetrain).is_ok());
            assert!(drivetrain.write().is_ok());
        }

        let mut car = helper.load_tmp_car();
        let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
        let traction_data = Traction::load_from_parent(&mut drivetrain).unwrap();
        assert_eq!(traction_data.drive_type, update_drive_type);
        helper.delete_tmp_car();
        Ok(())
    }

    #[test]
    fn update_same_number_gear_ratios() -> Result<(), String> {
        let original_ratios: Vec<f64> = vec![2.36, 1.94, 1.56, 1.29, 1.10, 0.92];
        let updated_ratios: Vec<f64> = vec![2.36, 1.98, 1.66, 1.33, 1.10, 0.88];
        let helper = TestFileHelper::new("update_same_number_gear_ratios".to_string());
        {
            let mut car = helper.setup_tmp_car_as("six-gears");
            let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
            let mut gearbox_data = Gearbox::load_from_parent(&mut drivetrain).unwrap();
            assert_eq!(gearbox_data.num_gears(), 6);
            assert_eq!(gearbox_data.final_drive(), 3.10);
            for (actual, expected, ) in gearbox_data.gear_ratios().iter().zip(original_ratios) {
                assert_eq!(*actual, expected);
            }

            let _ = gearbox_data.update_gears(updated_ratios.clone());
            assert!(gearbox_data.update_car_data(&mut drivetrain).is_ok());
            assert!(drivetrain.write().is_ok());
        }

        let mut car = helper.load_tmp_car();
        let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
        let gearbox_data = Gearbox::load_from_parent(&mut drivetrain).unwrap();
        assert_eq!(gearbox_data.num_gears(), updated_ratios.len());
        assert_eq!(gearbox_data.final_drive(), 3.10);
        for (actual, expected, ) in gearbox_data.gear_ratios().iter().zip(updated_ratios) {
            assert_eq!(expected, *actual);
        }
        helper.delete_tmp_car();
        Ok(())
    }

    #[test]
    fn update_more_gear_ratios() -> Result<(), String> {
        let original_ratios: Vec<f64> = vec![2.36, 1.94, 1.56, 1.29, 1.10, 0.92];
        let updated_ratios: Vec<f64> = vec![2.36, 1.98, 1.66, 1.33, 1.10, 0.88, 0.72];
        let helper = TestFileHelper::new("update_more_gear_ratios".to_string());
        {
            let mut car = helper.setup_tmp_car_as("six-gears");
            let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
            let mut gearbox_data = Gearbox::load_from_parent(&mut drivetrain).unwrap();
            assert_eq!(gearbox_data.num_gears(), 6);
            assert_eq!(gearbox_data.final_drive(), 3.10);
            for (actual, expected, ) in gearbox_data.gear_ratios().iter().zip(original_ratios) {
                assert_eq!(*actual, expected);
            }

            let _ = gearbox_data.update_gears(updated_ratios.clone());
            assert_eq!(gearbox_data.num_gears(), updated_ratios.len());
            assert!(gearbox_data.update_car_data(&mut drivetrain).is_ok());
            assert!(drivetrain.write().is_ok());
        }

        let mut car = helper.load_tmp_car();
        let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
        let gearbox_data = Gearbox::load_from_parent(&mut drivetrain).unwrap();
        assert_eq!(gearbox_data.num_gears(), updated_ratios.len());
        assert_eq!(gearbox_data.final_drive(), 3.10);
        for (actual, expected, ) in gearbox_data.gear_ratios().iter().zip(updated_ratios) {
            assert_eq!(expected, *actual);
        }
        helper.delete_tmp_car();
        Ok(())
    }

    #[test]
    fn update_less_gear_ratios() -> Result<(), String> {
        let original_ratios: Vec<f64> = vec![2.36, 1.94, 1.56, 1.29, 1.10, 0.92];
        let updated_ratios: Vec<f64> = vec![2.36, 1.98, 1.66, 1.33];
        let helper = TestFileHelper::new("update_less_gear_ratios".to_string());
        {
            let mut car = helper.setup_tmp_car_as("six-gears");
            let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
            let mut gearbox_data = Gearbox::load_from_parent(&mut drivetrain).unwrap();
            assert_eq!(gearbox_data.num_gears(), 6);
            assert_eq!(gearbox_data.final_drive(), 3.10);
            for (actual, expected, ) in gearbox_data.gear_ratios().iter().zip(original_ratios) {
                assert_eq!(*actual, expected);
            }

            let _ = gearbox_data.update_gears(updated_ratios.clone());
            assert!(gearbox_data.update_car_data(&mut drivetrain).is_ok());
            assert!(drivetrain.write().is_ok());
        }

        let mut car = helper.load_tmp_car();
        let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
        let gearbox_data = Gearbox::load_from_parent(&mut drivetrain).unwrap();
        assert_eq!(gearbox_data.num_gears(), updated_ratios.len());
        assert_eq!(gearbox_data.final_drive(), 3.10);
        for (actual, expected, ) in gearbox_data.gear_ratios().iter().zip(updated_ratios) {
            assert_eq!(expected, *actual);
        }
        helper.delete_tmp_car();
        Ok(())
    }

    #[test]
    fn update_final_drive() -> Result<(), String> {
        let original: f64 = 3.10;
        let updated: f64 = 3.20;
        let helper = TestFileHelper::new("update_final_drive".to_string());
        {
            let mut car = helper.setup_tmp_car_as("six-gears");
            let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
            let mut gearbox_data = Gearbox::load_from_parent(&mut drivetrain).unwrap();
            assert_eq!(gearbox_data.num_gears(), 6);
            assert_eq!(gearbox_data.final_drive(), original);

            gearbox_data.update_final_drive(updated);
            assert!(gearbox_data.update_car_data(&mut drivetrain).is_ok());
            assert!(drivetrain.write().is_ok());
        }

        let mut car = helper.load_tmp_car();
        let mut drivetrain = Drivetrain::from_car(&mut car).unwrap();
        let gearbox_data = Gearbox::load_from_parent(&mut drivetrain).unwrap();
        assert_eq!(gearbox_data.num_gears(), 6);
        assert_eq!(gearbox_data.final_drive(), updated);
        helper.delete_tmp_car();
        Ok(())
    }

    // #[test]
    // fn update_gearbox() -> Result<(), String> {
    //     let new_change_up_time = 140;
    //     let new_change_dn_time = 190;
    //     let new_auto_cutoof_time = 160;
    //     let new_supports_shifter = 1;
    //     let new_valid_shift_window = 700;
    //     let new_controls_window_gain = 0.3;
    //     let new_inertia = 0.02;
    //     let new_ratios: Vec<f64> = vec!(2.40, 1.9, 1.61, 1.33, 1.12, 0.99);
    //     let new_final_drive = 3.2;
    //     let new_reverse_drive = -3.700;
    //
    //     let output_ini_string = subcomponent_update_test(|gearbox: &mut Gearbox| {
    //         gearbox.change_up_time = new_change_up_time;
    //         gearbox.change_dn_time = new_change_dn_time;
    //         gearbox.auto_cutoff_time = new_auto_cutoof_time;
    //         gearbox.supports_shifter = new_supports_shifter;
    //         gearbox.valid_shift_rpm_window = new_valid_shift_window;
    //         gearbox.controls_window_gain = new_controls_window_gain;
    //         gearbox.inertia = new_inertia;
    //         gearbox.reverse_gear_ratio = new_reverse_drive;
    //         gearbox.update_gears(new_ratios.clone(), new_final_drive);
    //     })?;
    //     validate_subcomponent(output_ini_string, |gearbox: &Gearbox| {
    //         assert_eq!(gearbox.change_up_time, new_change_up_time, "Change up time is correct");
    //         assert_eq!(gearbox.change_dn_time, new_change_dn_time, "Change dn time is correct");
    //         assert_eq!(gearbox.auto_cutoff_time, new_auto_cutoof_time, "Auto cutoff time is correct");
    //         assert_eq!(gearbox.supports_shifter, new_supports_shifter, "Supports shifter is correct");
    //         assert_eq!(gearbox.valid_shift_rpm_window, new_valid_shift_window, "Valid shift rpm window is correct");
    //         assert_eq!(gearbox.controls_window_gain, new_controls_window_gain, "Controls window gain is correct");
    //         assert_eq!(gearbox.reverse_gear_ratio, new_reverse_drive, "Reverse gear ratio is correct");
    //         assert_eq!(gearbox.inertia, new_inertia, "Inertia is correct");
    //         assert_eq!(gearbox.gear_ratios, new_ratios, "New ratios are correct");
    //         assert_eq!(gearbox.final_gear_ratio, new_final_drive, "Final drive correct");
    //         assert_eq!(gearbox.gear_count, new_ratios.len() as i32, "Gear count is correct");
    //     })
    // }
    //
    // #[test]
    // fn update_clutch() -> Result<(), String> {
    //     let new_max_torque = 300;
    //
    //     let output_ini_string = subcomponent_update_test(|clutch: &mut Clutch|{
    //         clutch.max_torque = new_max_torque;
    //     })?;
    //     validate_subcomponent(output_ini_string,
    //                        |clutch: &Clutch| {
    //         assert_eq!(clutch.max_torque, new_max_torque, "Max torque is correct");
    //     })
    // }
    //
    // #[test]
    // fn update_differential() -> Result<(), String> {
    //     let new_preload = 15;
    //     let new_power = 0.2;
    //     let new_coast = 0.8;
    //
    //     let output_ini_string = subcomponent_update_test(|differential: &mut Differential|{
    //         differential.preload = new_preload;
    //         differential.power = new_power;
    //         differential.coast = new_coast;
    //     })?;
    //     validate_subcomponent(output_ini_string, |differential: &Differential| {
    //         assert_eq!(differential.preload, new_preload, "Preload is correct");
    //         assert_eq!(differential.power, new_power, "Power is correct");
    //         assert_eq!(differential.coast, new_coast, "Coast is correct");
    //     })
    // }
    //
    // #[test]
    // fn update_auto_clutch() -> Result<(), String> {
    //     let new_min_rpm = 2250;
    //     let new_max_rpm = 3250;
    //     let new_use_on_changes = 0;
    //     let new_forced_on = 1;
    //
    //     let output_ini_string = subcomponent_update_test(|auto_clutch: &mut AutoClutch|{
    //         auto_clutch.min_rpm = new_min_rpm;
    //         auto_clutch.max_rpm = new_max_rpm;
    //         auto_clutch.use_on_changes = new_use_on_changes;
    //         auto_clutch.forced_on = new_forced_on;
    //     })?;
    //     validate_subcomponent(output_ini_string, |auto_clutch: &AutoClutch| {
    //         assert_eq!(auto_clutch.min_rpm, new_min_rpm, "MinRpm is correct");
    //         assert_eq!(auto_clutch.max_rpm, new_max_rpm, "MaxRpm is correct");
    //         assert_eq!(auto_clutch.use_on_changes, new_use_on_changes, "new_use_on_changes is correct");
    //         assert_eq!(auto_clutch.forced_on, new_forced_on, "new_forced_on is correct");
    //     })
    // }
    //
    // #[test]
    // fn update_auto_blip() -> Result<(), String> {
    //     let new_level = 0.8;
    //     let new_points = vec![20, 200, 260];
    //     let new_electronic = 0;
    //
    //     let output_ini_string = subcomponent_update_test(|auto_blip: &mut AutoBlip|{
    //         auto_blip.level = new_level;
    //         auto_blip.points = new_points.clone();
    //         auto_blip.electronic = new_electronic;
    //     })?;
    //     validate_subcomponent(output_ini_string, |auto_blip: &AutoBlip| {
    //         assert_eq!(auto_blip.level, new_level, "Level is correct");
    //         assert_eq!(auto_blip.points, new_points, "Points are correct");
    //         assert_eq!(auto_blip.electronic, new_electronic, "Electronic is correct");
    //     })
    // }
    //
    // #[test]
    // fn update_auto_shifter() -> Result<(), String> {
    //     let new_up = 6000;
    //     let new_down = 4400;
    //     let new_slip_threshold = 1.0;
    //     let new_gas_cutoff_time = 0.3;
    //
    //     let output_ini_string = subcomponent_update_test(|auto_shifter: &mut AutoShifter|{
    //         auto_shifter.up = new_up;
    //         auto_shifter.down = new_down;
    //         auto_shifter.slip_threshold = new_slip_threshold;
    //         auto_shifter.gas_cutoff_time = new_gas_cutoff_time;
    //     })?;
    //     validate_subcomponent(output_ini_string, |auto_shifter: &AutoShifter| {
    //         assert_eq!(auto_shifter.up, new_up, "Up is correct");
    //         assert_eq!(auto_shifter.down, new_down, "Down are correct");
    //         assert_eq!(auto_shifter.slip_threshold, new_slip_threshold, "Slip threshold is correct");
    //         assert_eq!(auto_shifter.gas_cutoff_time, new_gas_cutoff_time, "Gas cutoff time is correct");
    //     })
    // }
    //
    // #[test]
    // fn update_downshift_protection() -> Result<(), String> {
    //     let new_active = 0;
    //     let new_debug = 1;
    //     let new_overrev = 300;
    //     let new_lock_n = 0;
    //
    //     let output_ini_string = subcomponent_update_test(|downshift_protection: &mut DownshiftProtection| {
    //         downshift_protection.active = new_active;
    //         downshift_protection.debug = new_debug;
    //         downshift_protection.overrev = new_overrev;
    //         downshift_protection.lock_n = new_lock_n;
    //     })?;
    //     validate_subcomponent(output_ini_string, |downshift_protection: &DownshiftProtection| {
    //         assert_eq!(downshift_protection.active, new_active, "Active is correct");
    //         assert_eq!(downshift_protection.debug, new_debug, "Debug is correct");
    //         assert_eq!(downshift_protection.overrev, new_overrev, "Overrev is correct");
    //         assert_eq!(downshift_protection.lock_n, new_lock_n, "Lock N is correct");
    //     })
    // }
    //
    // fn subcomponent_update_test<T: IniUpdater + MandatoryDataSection, F: FnOnce(&mut T)>(component_update_fn: F) -> Result<String, String> {
    //     let mut drivetrain = Drivetrain::load_from_ini_string(String::from(TEST_DATA));
    //     let mut component = extract_mandatory_section::<T>(&drivetrain).unwrap();
    //     component_update_fn(&mut component);
    //     drivetrain.update_subcomponent(&component).map_err(|err| format!("{}", err.to_string()))?;
    //     Ok(drivetrain.ini_data.to_string())
    // }
    //
    // fn validate_subcomponent<T, F>(ini_string: String, component_validation_fn: F) -> Result<(), String>
    //     where T: MandatoryDataSection,
    //           F: FnOnce(&T)
    // {
    //     let drivetrain = Drivetrain::load_from_ini_string(ini_string);
    //     let component = extract_mandatory_section::<T>(&drivetrain).map_err(|err| format!("{}", err.to_string()))?;
    //     component_validation_fn(&component);
    //     Ok(())
    // }
}