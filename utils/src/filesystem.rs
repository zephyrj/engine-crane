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

use std::{fs, io};
use std::path::{Path, PathBuf};

pub fn get_filetypes_in_path(path: &Path, file_type: &str) -> io::Result<Vec<PathBuf>> {
    let dir_entries = fs::read_dir(path)?;
    Ok(dir_entries.filter_map(|e| {
        match e {
            Ok(dir_entry) => {
                if dir_entry.path().is_file() {
                    match dir_entry.path().extension() {
                        Some(ext) => {
                            if ext.ne(file_type) {
                                return None
                            }
                        },
                        None => return None
                    }
                    Some(dir_entry.path())
                } else {
                    None
                }
            },
            _ => None
        }
    }).collect())
}
