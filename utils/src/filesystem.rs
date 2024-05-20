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

/// Takes a name and turns it into a safe filename in the provided path. The filename
/// will be "safe" in the sense that the returned filename will be free of any characters that
/// would be illegal to use in a filesystem path and also unique so as not to
/// override anything else in the provided path. Additionally, any spaces in the filename will
/// be replaced with underscores.
///
/// To provide uniqueness a number will be appended to the returned filename if the name would
/// clash with anything else in the provided path. i.e. if you have a file called test.txt present
/// in the path then the next filename returned would be test2.txt
///
pub fn create_safe_filename_in_path(path: &Path, name: &str, extension: &str) -> PathBuf {
    let mut sanitized_name = sanitize_filename::sanitize(name);
    sanitized_name = sanitized_name.replace(" ", "_");
    let mut file_path = path.join(format!("{}.{}", sanitized_name, extension));
    let mut extra_num = 2;
    while file_path.exists() {
        file_path = path.join(format!("{}{}.{}", sanitized_name, extra_num, extension));
        extra_num += 1;
    }
    file_path
}
