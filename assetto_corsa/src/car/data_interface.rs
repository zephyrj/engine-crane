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

use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::{fs, io};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use crate::car::acd_utils::AcdArchive;
use crate::traits::{_DataInterfaceI, DataInterface, DataInterfaceResult};
use crate::error::{Error, ErrorKind, Result};

#[derive(Debug)]
pub struct DataFolderInterface {
    data_folder_path: PathBuf,
    outstanding_data_updates: HashMap<String, Option<Vec<u8>>>
}

impl DataFolderInterface {
    pub(crate) fn new(data_folder_path: PathBuf) -> Result<Self> {
        if !data_folder_path.exists() {
            fs::create_dir(&data_folder_path)?;
        }
        Ok(DataFolderInterface {
            data_folder_path,
            outstanding_data_updates: HashMap::new()
        })
    }

    pub(crate) fn from(path: &Path) -> Result<Self> {
        if !path.is_dir() {
            return Err(Error::new(ErrorKind::IOError,
                                  format!("Directory {} doesn't exist", path.display())));
        }
        Ok(DataFolderInterface {
            data_folder_path: path.to_path_buf(),
            outstanding_data_updates: HashMap::new()
        })
    }

    fn construct_file_path(&self, filename: &str) -> PathBuf {
        (&self.data_folder_path).join(Path::new(filename))
    }
}

impl _DataInterfaceI for DataFolderInterface {
    fn get_original_file_data(&self, filename: &str) -> DataInterfaceResult<Option<Vec<u8>>> {
        let file_path = (&self.data_folder_path).join(Path::new(filename));
        let f = match File::open(file_path) {
            Ok(file) => Ok(file),
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => return Ok(None),
                    _ => Err(e),
                }
            }
        }?;
        let mut reader = BufReader::new(f);
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(Some(data))
    }

    fn contains_file(&self, filename: &str) -> bool {
        match self.outstanding_data_updates.get(filename) {
            None => {
                self.data_folder_path.join(Path::new(filename)).is_file()
            }
            Some(data) => {
                data.is_some()
            }
        }
    }

    fn update_file_data(&mut self, filename: &str, data: Vec<u8>) {
        self.outstanding_data_updates.insert(filename.to_owned(), Some(data));
    }

    fn remove_file(&mut self, filename: &str) {
        self.outstanding_data_updates.insert(filename.to_owned(), None);
    }

    fn write(&mut self) -> DataInterfaceResult<()> {
        for (filename, data) in &self.outstanding_data_updates {
            match data {
                None => {
                    std::fs::remove_file(&self.construct_file_path(&filename))?;
                }
                Some(data) => {
                    File::create(&self.construct_file_path(&filename))?.write_all(data.as_slice())?;
                }
            }
        }
        self.outstanding_data_updates.clear();
        Ok(())
    }
}

impl DataInterface for DataFolderInterface {}

#[derive(Debug)]
pub struct AcdDataInterface {
    acd_archive: AcdArchive
}

impl AcdDataInterface {
    pub fn new(path: &Path) -> Result<Self> {
        Ok(AcdDataInterface { acd_archive: AcdArchive::load_from_acd_file(path)? })
    }
}

impl _DataInterfaceI for AcdDataInterface {
    fn get_original_file_data(&self, filename: &str) -> DataInterfaceResult<Option<Vec<u8>>>  {
        Ok(self.acd_archive.get_file_data(filename))
    }

    fn contains_file(&self, filename: &str) -> bool {
        self.acd_archive.contains_file(filename)
    }

    fn update_file_data(&mut self, filename: &str, data: Vec<u8>) {
        self.acd_archive.update_file_data(filename.to_owned(), data);
    }

    fn remove_file(&mut self, filename: &str) {
        self.acd_archive.delete_file(filename);
    }

    fn write(&mut self) -> DataInterfaceResult<()> {
        self.acd_archive.write()?;
        Ok(())
    }
}

impl DataInterface for AcdDataInterface {}
