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

use crate::assetto_corsa::error::{Result, Error, ErrorKind};
use crate::assetto_corsa::{ini_utils};
use crate::assetto_corsa::car::Car;
use crate::assetto_corsa::ini_utils::Ini;
use crate::assetto_corsa::traits::{CarDataFile, DataInterface};


#[derive(Debug)]
pub struct Drivetrain<'a> {
    car: &'a mut Car,
    ini_data: Ini,
}

impl<'a> Drivetrain<'a> {
    const INI_FILENAME: &'static str = "drivetrain.ini";

    pub fn from_car(car: &'a mut Car) -> Result<Drivetrain<'a>> {
        let file_data = match car.data_interface.get_file_data(Drivetrain::INI_FILENAME) {
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
    use crate::assetto_corsa::car::data::drivetrain::{AutoBlip, AutoClutch, AutoShifter, Clutch, Differential, DownshiftProtection, Drivetrain, Gearbox, Traction};
    use crate::assetto_corsa::traits::{extract_mandatory_section, MandatoryDataSection};

    const TEST_DATA: &'static str = r#"
[HEADER]
VERSION=3

[TRACTION]
TYPE=RWD					; Wheel drive. Possible options: FWD (Front Wheel Drive), RWD (Rear Wheel Drive)

[GEARS]
COUNT=6				; forward gears number
GEAR_R=-3.818			; rear gear ratio
; forward gears ratios. must be equal to number of gears defined on count
GEAR_1=2.36
GEAR_2=1.94
GEAR_3=1.56
GEAR_4=1.29
GEAR_5=1.10
GEAR_6=0.92

FINAL=3.10		; final gear ratio

[DIFFERENTIAL]
POWER=0.10			; differential lock under power. 1.0=100% lock - 0 0% lock
COAST=0.90			; differential lock under coasting. 1.0=100% lock 0=0% lock
PRELOAD=13			; preload torque setting

[GEARBOX]
CHANGE_UP_TIME=130		; change up time in milliseconds
CHANGE_DN_TIME=180		; change down time in milliseconds
AUTO_CUTOFF_TIME=150		; Auto cutoff time for upshifts in milliseconds, 0 to disable
SUPPORTS_SHIFTER=0		; 1=Car supports shifter, 0=car supports only paddles
VALID_SHIFT_RPM_WINDOW=800			;range window additional to the precise rev matching rpm that permits gear engage.
CONTROLS_WINDOW_GAIN=0.4			;multiplayer for gas,brake,clutch pedals that permits gear engage on different rev matching rpm. the lower the more difficult.
INERTIA=0.018					; gearbox inertia. default values to 0.02 if not set

[CLUTCH]
MAX_TORQUE=400

[AUTOCLUTCH]
UPSHIFT_PROFILE=NONE			; Name of the autoclutch profile for upshifts. NONE to disable autoclutch on shift up
DOWNSHIFT_PROFILE=DOWNSHIFT_PROFILE	; Same as above for downshifts
USE_ON_CHANGES=1				; Use the autoclutch on gear shifts even when autoclutch is set to off. Needed for cars with semiautomatic gearboxes. values 1,0
MIN_RPM=2000					; Minimum rpm for autoclutch engadgement
MAX_RPM=3000					; Maximum rpm for autoclutch engadgement
FORCED_ON=0

[DOWNSHIFT_PROFILE]
POINT_0=10				; Time to reach fully depress clutch
POINT_1=190				; Time to start releasing clutch
POINT_2=250				; Time to reach fully released clutch

[AUTOBLIP]
ELECTRONIC=1				; If =1 then it is a feature of the car and cannot be disabled
POINT_0=10				; Time to reach full level
POINT_1=150				; Time to start releasing gas
POINT_2=180			; Time to reach 0 gas
LEVEL=0.7				; Gas level to be reached

[AUTO_SHIFTER]
UP=6300
DOWN=4500
SLIP_THRESHOLD=1.1
GAS_CUTOFF_TIME=0.28

[DOWNSHIFT_PROTECTION]
ACTIVE=1
DEBUG=0				; adds a line in the log for every missed downshift
OVERREV=200		; How many RPM over the limiter the car is allowed to go
LOCK_N=1
"#;

    // #[test]
    // fn update_traction() -> Result<(), String> {
    //     let new_drive_type = DriveType::FWD;
    //
    //     let output_ini_string = subcomponent_update_test(|traction: &mut Traction| {
    //         traction.drive_type = new_drive_type.clone();
    //     })?;
    //     validate_subcomponent(output_ini_string, |gearbox: &Traction| {
    //         assert_eq!(gearbox.drive_type, new_drive_type, "Drive type is correct");
    //     })
    // }
    //
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