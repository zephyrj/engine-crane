/*
 * Copyright (c):
 * 2024 zephyrj
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

pub mod data_interface;
pub mod ui;
pub mod data;
pub mod acd_utils;
pub mod lut_utils;
pub(crate) mod structs;
mod max_speed_est;
pub mod model;

use crate::ini_utils::Comment;
pub use data_interface::DataFolderInterface;

use std::fmt::Debug;
use std::fs::File;
use std::{fs, io};
use std::io::{BufRead, BufReader, LineWriter, Write};
use std::ops::Add;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};
use walkdir::WalkDir;

use crate::traits:: DataInterface;
use crate::error::{Error, ErrorKind, Result};
use crate::{ini_utils, Installation};
use crate::ini_utils::Ini;
use acd_utils::AcdArchive;
use crate::car::data::CarIniData;
use crate::car::data_interface::AcdDataInterface;
use crate::car::ui::CarUiData;

pub const ENGINE_CRANE_CAR_TAG: &'static str = "engine crane";

pub fn clone_existing_car(ac_installation: &Installation,
                          existing_car_path: &Path,
                          new_car_path: &Path,
                          unpack_data_dir: bool) -> Result<()> {
    let existing_car_name = get_final_path_part(existing_car_path)?;

    if existing_car_path == new_car_path {
        return Err(Error::new(ErrorKind::CarAlreadyExists,
                              format!("Cannot clone car to its existing location. ({})", existing_car_path.display())));
    }

    if let Err(e) = std::fs::create_dir(&new_car_path) {
        return if e.kind() == io::ErrorKind::AlreadyExists {
            Err(Error::new(ErrorKind::CarAlreadyExists,
                           format!("Car {} directory already exists", new_car_path.display())))
        } else {
            Err(Error::from(e))
        }
    }

    let clone_actions = || -> Result<()> {
        let mut copy_options = fs_extra::dir::CopyOptions::new();
        copy_options.content_only = true;
        fs_extra::dir::copy(&existing_car_path,
                            &new_car_path,
                            &copy_options)?;

        let data_path = new_car_path.join("data");
        let acd_path = new_car_path.join("data.acd");
        if !data_path.is_dir() {
            if !acd_path.is_file() {
                return Err(Error::new(ErrorKind::InvalidCar,
                                      format!("{} doesn't contain a data dir or data.acd file", existing_car_path.display())));
            }
            info!("No data dir present in {}. Data will be extracted from data.acd", new_car_path.display());
            AcdArchive::load_from_acd_file_with_key(acd_path.as_path(), &existing_car_name)?.unpack()?;
        }

        fix_car_specific_filenames(new_car_path, &existing_car_name)?;
        update_car_sfx(ac_installation, new_car_path, &existing_car_name)?;

        match unpack_data_dir {
            true => {
                info!("Deleting {} as data will be invalid after clone completion", acd_path.display());
                if let Some(err) = delete_data_acd_file(new_car_path).err(){
                    warn!("Warning: {}", err.to_string());
                }
            }
            false => {
                info!("Packing {} into an .acd file", data_path.display());
                AcdArchive::create_from_data_dir(&data_path)?.write()?;
                if data_path.exists() {
                    info!("Deleting {} as data will be invalid after clone completion", data_path.display());
                    std::fs::remove_dir_all(data_path)?;
                }
            }
        }
        Ok(())
    };

    return match clone_actions() {
        Ok(_) => { Ok(()) }
        Err(e) => {
            error!("Clone of {} failed. {}", existing_car_path.display(), e.to_string());
            if let Err(remove_err) = std::fs::remove_dir_all(new_car_path) {
                warn!("Failed to remove {}. {}", new_car_path.display(), remove_err.to_string())
            }
            Err(e)
        }
    }
}

pub const UNPACK_DATA_BIT: u64 = 1 << 0;
pub const AC_CAR_TUNER_COMPAT_BIT: u64 = 1 << 1;

pub fn create_new_car_spec(ac_installation: &Installation,
                           existing_car_path: &PathBuf,
                           spec_name: &str,
                           opt_flags: u64) -> Result<PathBuf> {
    
    let opt_is_set = |opt_bit| {(opt_flags & opt_bit) > 0};
    let existing_car_name = get_final_path_part(existing_car_path)?;
    if !existing_car_path.exists() {
        return Err(Error::new(ErrorKind::NoSuchCar, existing_car_name));
    }
    let path_suffix = spec_name.to_lowercase().split_whitespace().collect::<Vec<&str>>().join("_");
    let new_car_name = format!("{}_{}", existing_car_name, path_suffix);
    let new_car_path = get_parent_path_part(existing_car_path)?.join(&new_car_name);
    if new_car_path.exists() {
        return Err(Error::new(ErrorKind::CarAlreadyExists, new_car_name));
    }
    info!("Cloning {} to {}", existing_car_path.display(), new_car_path.display());
    let unpack_data_dif = opt_is_set(UNPACK_DATA_BIT) || opt_is_set(AC_CAR_TUNER_COMPAT_BIT);
    clone_existing_car(ac_installation,
                       existing_car_path.as_path(),
                       new_car_path.as_path(),
                       unpack_data_dif)?;
    update_car_ui_data(new_car_path.as_path(), &existing_car_path, spec_name)?;
    if opt_is_set(AC_CAR_TUNER_COMPAT_BIT) {
        match create_x_tuned_file(&new_car_path, &path_suffix) {
            Err(e) => error!("Failed to create x.tuned file. {}", e),
            _ => info!("Added x.tuned for AC Car Tuner compatibility")
        }
        let drivetrain_ini_path = new_car_path.join("data").join("drivetrain.ini");
        // Workaround a bug in CarTuner when drive type doesn't have any indentation after it
        match Ini::load_from_file(&drivetrain_ini_path) {
            Ok(mut ini_file) => {
                match ini_file.get_mut_section("TRACTION") {
                    None => warn!("TRACTION section missing from drivetrain.ini file."),
                    Some(section) => {
                        match section.get_property_mut("TYPE") {
                            None => warn!("TRACTION section missing a type property."),
                            Some(property) => {
                                if !property.has_comment() {
                                    let _ = property.add_comment(
                                        Comment::new(
                                            String::from("Possible options: FWD, RWD, AWD or AWD2"),
                                            Some(String::from("  "))
                                        )
                                    );
                                }
                            }
                        }
                    }
                }
                match ini_file.write_to_file(&drivetrain_ini_path) {
                    Ok(_) => info!("Applied Car Tuner drivetrain.ini workaround"),
                    Err(e) => warn!("Failed to write drivetrain.ini Car Tuner workaround. {}", e.to_string())
                }
            }
            Err(e) => warn!("Car appears to be missing a drivetrain.ini file. {}", e.to_string())
        }
    }
    Ok(new_car_path)
}

fn create_x_tuned_file(car_path: &PathBuf, tune_name: &str) -> io::Result<()> {
    let mut f = File::create(car_path.join("x.tuned"))?;
    f.write_all(tune_name.as_bytes())?;
    Ok(())
}

pub fn delete_data_acd_file(car_path: &Path) -> Result<()> {
    let acd_path = car_path.join("data.acd");
    if acd_path.exists() {
        std::fs::remove_file(acd_path)?;
    }
    Ok(())
}

pub fn delete_car(ac_installation: &Installation, car_folder_name: &Path) -> std::io::Result<()> {
    let path = ac_installation.get_installed_car_path().join(car_folder_name);
    std::fs::remove_dir_all(path)
}

fn fix_car_specific_filenames(car_path: &Path, name_to_change: &str) -> Result<()> {
    let new_car_name = get_final_path_part(car_path)?;
    let mut paths_to_update: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&car_path).into_iter().filter_map(|e| e.ok()) {
        match entry.metadata() {
            Ok(metadata) => if !metadata.is_file() { continue; }
            Err(e) => {
                warn!("Error occurred whilst trying to obtain metadata during walk of {}. {}",
                      car_path.display(), e.to_string());
                continue;
            }
        }

        let filename = entry.file_name();
        let filename_string = filename.to_string_lossy();
        if (filename_string.starts_with(name_to_change)) &&
            (filename_string.ends_with(".kn5") || (filename_string.ends_with(".bank"))) {
            paths_to_update.push(entry.path().to_path_buf());
        } else if filename_string == "lods.ini" {
            let mut lod_ini = Ini::load_from_file(entry.path())?;
            let mut idx = 0;
            loop {
                let current_lod_name = format!("LOD_{}", idx);
                if !lod_ini.section_contains_property(&current_lod_name, "FILE") {
                    break
                }
                info!("Updating {}", current_lod_name);
                if let Some(old_value) = ini_utils::get_value::<String>(&lod_ini, &current_lod_name, "FILE") {
                    ini_utils::set_value(&mut lod_ini,
                                         &current_lod_name,
                                         "FILE",
                                         old_value.replace(name_to_change, &new_car_name));
                } else {
                    warn!("{} was unexpectedly missing from {}", current_lod_name, entry.path().display());
                }
                idx += 1;
            }
            lod_ini.write_to_file(entry.path())?;
        }
    }

    for path in paths_to_update {
        let mut new_path = path.clone();
        if let Some(os_string) = path.file_name() {
            if let Some(filename) = os_string.to_str() {
                let new_filename = filename.replace(name_to_change, &new_car_name);
                info!("Changing {} to {}", path.display(), new_filename);
                new_path.pop();
                new_path.push(new_filename);
                std::fs::rename(&path, &new_path)?;
            }
        }
    }
    Ok(())
}

pub fn update_car_ui_data(new_car_path: &Path, old_car_path: &Path, new_suffix: &str) -> Result<()> {
    let mut car = Car::load_from_path(new_car_path)?;
    let existing_name;
    let new_name ;
    {
        let mut ini_data = CarIniData::from_car(&mut car)?;
        existing_name = match ini_data.screen_name() {
            None => { get_final_path_part(new_car_path)? }
            Some(name) => { name }
        };
        new_name = existing_name.clone().add(format!(" {}", new_suffix).as_str());
        info!("Updating screen name from {} to {}", existing_name, new_name);
        ini_data.set_screen_name(new_name.as_str());
        ini_data.write()?;
    }

    {
        let mut ui_data = CarUiData::from_car(&mut car)?;
        let ui_name = match ui_data.ui_info.name() {
            None => new_name,
            Some(old_name) => format!("{} {}", old_name, new_suffix)
        };
        info!("Updating ui name to {}", ui_name);
        ui_data.ui_info.set_name(ui_name);
        match ui_data.ui_info.parent() {
            None => {
                info!("Updating parent name");
                match get_final_path_part(old_car_path) {
                    Ok(parent_car_folder_name) => {
                        ui_data.ui_info.set_parent(String::from(parent_car_folder_name));
                    }
                    Err(e) => warn!("Error occurred whilst trying to update parent name. {}",e.to_string())
                }
            }
            Some(existing_parent) => {
                info!("Parent name already set to {}", existing_parent);
            }
        }
        match ui_data.ui_info.add_tag_if_unique(ENGINE_CRANE_CAR_TAG.to_owned()) {
            Ok(added) => match added {
                true => info!("Added {} tag", ENGINE_CRANE_CAR_TAG),
                false => info!("{} already present in tags", ENGINE_CRANE_CAR_TAG)
            }
            Err(e) => warn!("Couldn't add {} tag. {}", ENGINE_CRANE_CAR_TAG, e)
        }
        ui_data.write()?;
    }
    Ok(())
}

fn update_car_sfx(ac_installation: &Installation,
                  car_path: &Path,
                  name_to_change: &str) -> Result<()> {
    let guids_file_path = car_path.join(PathBuf::from_iter(["sfx", "GUIDs.txt"]));
    let car_name = get_final_path_part(car_path)?;

    let updated_lines: Vec<String>;
    if guids_file_path.exists() {
        info!("Updating contents of '{}'. Replacing refs to '{}' with '{}'", guids_file_path.display(), name_to_change, &car_name);
        let file = File::open(&guids_file_path)?;
        updated_lines = BufReader::new(file).lines().into_iter().filter_map(|res| {
            match res {
                Ok(string) => Some(string.replace(name_to_change, &car_name)),
                Err(err) => {
                    println!("Warning: Encountered error reading from {}. {}",
                             guids_file_path.display(),
                             err.to_string());
                    None
                }
            }
        }).collect();
    } else {
        info!("Generating new '{}' with contents from the installation sfx data", guids_file_path.display());
        updated_lines = ac_installation.load_sfx_data()?.generate_clone_guid_info(name_to_change, &car_name);
    }

    let file = File::create(&guids_file_path)?;
    let mut file = LineWriter::new(file);
    for line in updated_lines {
        write!(file, "{}\n", line)?;
    }
    Ok(())
}

#[derive(Debug)]
pub struct Car {
    root_path: PathBuf,
    data_interface: Box<dyn DataInterface>,
}

impl Car {
    pub fn new(root_path: PathBuf) -> Result<Car> {
        if !root_path.exists() {
            fs::create_dir(&root_path)?;
        }
        let data_dir_path = root_path.join("data");
        Ok(Car{
            root_path,
            data_interface: Box::new(DataFolderInterface::new(data_dir_path)?)
        })
    }

    pub fn load_from_path(car_folder_path: &Path) -> Result<Car> {
        let data_dir_path = car_folder_path.join("data");
        let data_file_path = car_folder_path.join("data.acd");
        Ok(Car{
            root_path: car_folder_path.to_path_buf(),
            data_interface: match data_dir_path.is_dir() {
                true => Box::new(DataFolderInterface::from(&data_dir_path)?),
                false => Box::new(AcdDataInterface::new(&data_file_path)?),
            }
        })
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn data_interface(&self) -> &dyn DataInterface {
        self.data_interface.as_ref()
    }

    pub fn mut_data_interface(&mut self) -> & mut dyn DataInterface {
        self.data_interface.as_mut()
    }
    
    pub fn is_ac_car_tuner_tune(&self) -> bool {
        self.root_path.join("x.tuned").is_file()
    }
}

fn get_final_path_part(full_path: &Path) -> Result<String> {
    return match full_path.file_name() {
        Some(n) => { Ok(n.to_string_lossy().to_string()) }
        None => {
            return Err(Error::new(ErrorKind::ArgumentError,
                                  format!("Can't get last part from provided path ({})", full_path.display())));
        }
    };
}

fn get_parent_path_part(full_path: &Path) -> Result<&Path> {
    return match full_path.parent() {
        Some(n) => { Ok(n) }
        None => {
            return Err(Error::new(ErrorKind::ArgumentError,
                                  format!("Can't get  parent part from provided path ({})", full_path.display())));
        }
    }
}



#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path};
    use crate::car::{Car, create_new_car_spec};
    use crate::car::data::CarIniData;
    use crate::car::ui::CarUiData;
    use crate::Installation;

    #[test]
    fn load_car() -> Result<(), String> {
        let this_file = Path::new(file!());
        let this_dir = this_file.parent().unwrap();
        let path = this_dir.join("test-data/car-with-turbo-with-ctrls");
        let mut car = match Car::load_from_path(&path) {
            Ok(car) => {
                car
            }
            Err(e) => {  return Err(e.to_string()) }
        };
        let ui_data = CarUiData::from_car(&mut car).unwrap();
        let ui_info = ui_data.ui_info;
        assert_eq!(ui_info.name().unwrap(), "Turbo with CTRL");
        assert_eq!(ui_info.brand().unwrap(), "Test");
        assert_eq!(ui_info.class().unwrap(), "street");
        assert_eq!(ui_info.tags().unwrap(), Vec::from(["#Supercars", "awd", "semiautomatic", "street", "turbo", "germany"]));
        let _specs = ui_info.specs().unwrap();
        Ok(())
    }

    #[test]
    fn clone_car() {
        let ac_install = Installation::new();
        let mut opts = 0;
        opts |= crate::car::AC_CAR_TUNER_COMPAT_BIT;
        let new_car_path = create_new_car_spec(&ac_install,
                                               &ac_install.get_installed_car_path().join("urd_radical_sr3xxr_2023"),
                                               "test",
                                               opts).unwrap();
        println!("{}", new_car_path.display());
    }

    #[test]
    fn installed_car_test() {
        let ac_install = Installation::new();
        let installed_cars = ac_install.get_list_of_installed_cars().unwrap();
        let mut pass_file = File::create(Path::new("pass.txt")).unwrap();
        let mut fail_file = File::create(Path::new("fail.txt")).unwrap();
        for path in &installed_cars {
            match Car::load_from_path(path) {
                Ok(mut car) => {
                    {
                        match CarIniData::from_car(&mut car) {
                            Ok(ini_data) => {
                                if let Some(name) = ini_data.screen_name() {
                                    write!(pass_file, "{} at {} passed\n", name, path.display()).unwrap();
                                    if let Some(header_data) = car.data_interface.get_original_file_data("_header").unwrap() {
                                        write!(pass_file, "Contained header {:02X?}\n", header_data).unwrap();
                                    }
                                    pass_file.flush();
                                } else {
                                    write!(pass_file, "{} has no screen name\n", path.display()).unwrap();
                                    pass_file.flush();
                                }
                            }
                            Err(err) => {
                                write!(fail_file, "{} ini load failed. {}\n", path.display(), err.to_string()).unwrap();
                                fail_file.flush();
                            }
                        }
                    }
                }
                Err(err) => {
                    write!(fail_file, "{} car load failed. {}\n", path.display(), err.to_string()).unwrap();
                    fail_file.flush();
                }
            }
        }
    }
}
