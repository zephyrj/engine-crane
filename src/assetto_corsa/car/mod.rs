pub mod data_interface;
pub mod ui;
pub mod data;
pub mod acd_utils;
pub mod lut_utils;
pub(crate) mod structs;

pub use data_interface::DataFolderInterface;

use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader, LineWriter, Write};
use std::ops::Add;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::assetto_corsa;
use crate::assetto_corsa::traits:: DataInterface;
use crate::assetto_corsa::error::{Error, ErrorKind, Result};
use crate::assetto_corsa::{ini_utils, load_sfx_data};
use crate::assetto_corsa::ini_utils::Ini;
use acd_utils::AcdArchive;
use crate::assetto_corsa::car::data::CarIniData;
use crate::assetto_corsa::car::data_interface::AcdDataInterface;
use crate::assetto_corsa::car::ui::CarUiData;


pub fn clone_existing_car(existing_car_path: &Path, new_car_path: &Path) -> Result<()> {
    if existing_car_path == new_car_path {
        return Err(Error::new(ErrorKind::CarAlreadyExists,
                              format!("Cannot clone car to its existing location. ({})", existing_car_path.display())));
    }

    std::fs::create_dir(&new_car_path)?;
    let mut copy_options = fs_extra::dir::CopyOptions::new();
    copy_options.content_only = true;
    fs_extra::dir::copy(&existing_car_path,
                        &new_car_path,
                        &copy_options)?;

    let existing_car_name = existing_car_path.file_name().unwrap().to_str().unwrap();
    let data_path = new_car_path.join("data");
    let acd_path = new_car_path.join("data.acd");
    if !data_path.exists() {
        if !acd_path.exists() {
            return Err(Error::new(ErrorKind::InvalidCar,
                                  format!("{} doesn't contain a data dir or data.acd file", existing_car_path.display())));
        }
        info!("No data dir present in {}. Data will be extracted from data.acd", new_car_path.display());
        AcdArchive::load_from_path_with_key(acd_path.as_path(), existing_car_name)?.unpack()?;
    }
    info!("Deleting {} as data will be invalid after clone completion", acd_path.display());
    if let Some(err) = delete_data_acd_file(new_car_path).err(){
        warn!("Warning: {}", err.to_string());
    }
    fix_car_specific_filenames(new_car_path, existing_car_name)?;
    update_car_sfx(new_car_path, existing_car_name)?;
    Ok(())
}

pub fn create_new_car_spec(existing_car_name: &str, spec_name: &str) -> Result<PathBuf>{
    let installed_cars_path = match assetto_corsa::get_installed_cars_path() {
        Some(path) => { path },
        None => {
            return Err(Error::new(ErrorKind::NoSuchCar, existing_car_name.to_owned()));
        }
    };
    let existing_car_path = installed_cars_path.join(existing_car_name);
    if !existing_car_path.exists() {
        return Err(Error::new(ErrorKind::NoSuchCar, existing_car_name.to_owned()));
    }
    let new_car_name = format!(
        "{}_{}",
        existing_car_name,
        spec_name.to_lowercase().split_whitespace().collect::<Vec<&str>>().join("_")
    );
    let new_car_path = installed_cars_path.join(&new_car_name);
    if new_car_path.exists() {
        return Err(Error::new(ErrorKind::CarAlreadyExists, new_car_name));
    }
    info!("Cloning {} to {}", existing_car_path.display(), new_car_path.display());
    clone_existing_car(existing_car_path.as_path(), new_car_path.as_path())?;
    update_car_ui_data(new_car_path.as_path(), spec_name, existing_car_name)?;
    Ok(new_car_path)
}

pub fn delete_data_acd_file(car_path: &Path) -> Result<()> {
    let acd_path = car_path.join("data.acd");
    if acd_path.exists() {
        std::fs::remove_file(acd_path)?;
    }
    Ok(())
}

fn fix_car_specific_filenames(car_path: &Path, name_to_change: &str) -> Result<()> {
    let new_car_name = car_path.file_name().unwrap().to_str().unwrap();
    let mut paths_to_update: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&car_path).into_iter().filter_map(|e| e.ok()) {
        if !entry.metadata().unwrap().is_file() {
            continue
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
                let old_value: String = ini_utils::get_value(&lod_ini, &current_lod_name, "FILE").unwrap();
                ini_utils::set_value(&mut lod_ini,
                                     &current_lod_name,
                                     "FILE",
                                     old_value.replace(name_to_change, new_car_name));
                idx += 1;
            }
            lod_ini.write_to_file(entry.path())?;
        }
    }

    for path in paths_to_update {
        let mut new_path = path.clone();
        let new_filename = path.file_name().unwrap().to_str().unwrap().replace(name_to_change, new_car_name);
        info!("Changing {} to {}", path.display(), new_filename);
        new_path.pop();
        new_path.push(new_filename);
        std::fs::rename(&path, &new_path)?;
    }
    Ok(())
}

pub fn update_car_ui_data(car_path: &Path, new_suffix: &str, parent_car_folder_name: &str) -> Result<()> {
    let mut car = Car::load_from_path(car_path)?;
    let mut existing_name = String::new();
    let mut new_name = String::new();
    {
        let mut ini_data = CarIniData::from_car(&mut car)?;
        existing_name = match ini_data.screen_name() {
            None => { String::from(car_path.file_name().unwrap().to_str().unwrap()) }
            Some(name) => { name }
        };
        new_name = existing_name.clone().add(format!(" {}", new_suffix).as_str());
        info!("Updating screen name and ui data from {} to {}", existing_name, new_name);
        ini_data.set_screen_name(new_name.as_str());
        ini_data.write()?;
    }

    {
        let mut ui_data = CarUiData::from_car(&mut car)?;
        ui_data.ui_info.set_name(new_name);
        match ui_data.ui_info.parent() {
            None => {
                info!("Updating parent name");
                ui_data.ui_info.set_parent(String::from(parent_car_folder_name));
            }
            Some(existing_parent) => {
                info!("Parent name already set to {}", existing_parent);
            }
        }
        ui_data.ui_info.add_tag("engine crane".to_owned());
        ui_data.write()?;
    }
    Ok(())
}

fn update_car_sfx(car_path: &Path, name_to_change: &str) -> Result<()> {
    let guids_file_path = car_path.join(PathBuf::from_iter(["sfx", "GUIDs.txt"]));
    let car_name = car_path.file_name().unwrap().to_str().unwrap();

    let mut updated_lines: Vec<String> = Vec::new();
    if guids_file_path.exists() {
        info!("Updating contents of '{}'. Replacing refs to '{}' with '{}'", guids_file_path.display(), name_to_change, car_name);
        let file = File::open(&guids_file_path)?;
        updated_lines = BufReader::new(file).lines().into_iter().filter_map(|res| {
            match res {
                Ok(string) => Some(string.replace(name_to_change, car_name)),
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
        updated_lines = load_sfx_data()?.generate_clone_guid_info(name_to_change, car_name);
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
    pub fn load_from_path(car_folder_path: &Path) -> Result<Car> {
        let data_dir_path = car_folder_path.join("data");
        let data_file_path = car_folder_path.join("data.acd");
        Ok(Car{
            root_path: car_folder_path.to_path_buf(),
            data_interface: match data_dir_path.is_dir() {
                true => Box::new(DataFolderInterface::new(&data_dir_path)),
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
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use crate::assetto_corsa;
    use crate::assetto_corsa::car::{Car, create_new_car_spec};
    use crate::assetto_corsa::car::data::CarIniData;
    use crate::assetto_corsa::car::ui::CarUiData;

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
        let specs = ui_info.specs().unwrap();
        Ok(())
    }

    #[test]
    fn clone_car() {
        let new_car_path = create_new_car_spec("zephyr_za401", "test").unwrap();
        println!("{}", new_car_path.display());
    }

    #[test]
    fn installed_car_test() {
        let installed_cars = assetto_corsa::get_list_of_installed_cars().unwrap();
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
