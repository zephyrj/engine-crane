/*
 * Copyright (c):
 * 2022 zephyrj
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

use std::cell::RefCell;
use std::io;
use std::path::Path;
use std::rc::Rc;
use crate::assetto_corsa::ini_utils::Ini;


pub fn load_ini_file(ini_path: &Path) -> io::Result<Ini> {
    Ini::load_from_file(ini_path)
}

pub fn load_ini_file_rc(ini_path: &Path) -> io::Result<Rc<RefCell<Ini>>> {
    Ok(Rc::new(RefCell::new(load_ini_file(ini_path)?)))
}
