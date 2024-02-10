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
use sha2::Digest;

pub fn create_sha256_hash_array(hasher: impl Digest) -> Option<[u8; 32]> {
    let hash: Vec<u8> = hasher.finalize().iter().map(|b| *b).collect();
    match <[u8; 32]>::try_from(hash) {
        Ok(hash_array) => Some(hash_array),
        Err(_) => {
            None
        }
    }
}
