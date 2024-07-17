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
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use crate::Car;

#[derive(Debug)]
pub struct CarUpgradeIcon<'a> {
    #[allow(dead_code)]
    car: &'a Car,
    img_path: Option<PathBuf>
}

impl<'a> CarUpgradeIcon<'a> {
    pub fn from_car(car: &'a Car) -> CarUpgradeIcon<'a> {
        let path = car.root_path.join(["ui", "upgrade.png"].iter().collect::<PathBuf>());
        let img_path= match path.is_file() {
            true => Some(path),
            false => None
        };
        CarUpgradeIcon{ car, img_path }
    }

    pub fn is_present(&self) -> bool {
        self.img_path.is_some()
    }

    pub fn update(&'a mut self, image_bytes: &[u8]) -> io::Result<()> {
        let path = self.car.root_path.join(["ui", "upgrade.png"].iter().collect::<PathBuf>());
        let mut file = File::create(&path)?;
        self.img_path = Some(path);
        file.write_all(image_bytes)
    }
}