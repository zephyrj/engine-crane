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

pub mod car;
pub mod sandbox;

use std::path::PathBuf;
use crate::steam;

pub const STEAM_GAME_NAME: &str = "Automation";
pub const STEAM_GAME_ID: i64 = 293760;

pub fn is_installed() -> bool {
    get_install_path().is_dir()
}

pub fn get_install_path() -> PathBuf {
    steam::get_game_install_path(STEAM_GAME_NAME)
}
