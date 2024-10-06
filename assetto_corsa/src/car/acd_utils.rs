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
use std::io::{BufReader, Read, Write};
use std::{fs, io, mem};
use std::array::TryFromSliceError;
use indexmap::IndexMap;
use std::path::{Path, PathBuf};
use tracing::debug;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AcdError>;

#[derive(Error, Debug)]
pub enum AcdError {
    #[error("io error")]
    IoError(#[from] io::Error),
    #[error("Data size error")]
    DataSizeError(#[from] TryFromSliceError),
    #[error("Failed to generate key from {parent}. {reason}")]
    ExtractionKeyGenerationError {
        parent: String,
        reason: String
    },
    #[error("Failed to decode '{path}. {reason}")]
    DecodeError {
        path: String,
        reason: String
    },
    #[error("Failed to encode '{path}. {reason}")]
    EncodeError {
        path: String,
        reason: String
    },
}

fn missing_parent_error(path: &Path) -> AcdError {
    AcdError::DecodeError {
        path: path.display().to_string(),
        reason: String::from("Can't deduce parent folder")
    }
}

fn key_error(parent: &str, reason: String) -> AcdError {
    AcdError::ExtractionKeyGenerationError {
        parent: parent.to_owned(),
        reason
    }
}

fn get_parent_folder_str(path: &Path) -> Result<&str> {
    Ok(path.parent().ok_or(missing_parent_error(path))?
        .file_name().ok_or(missing_parent_error(path))?
        .to_str().ok_or(missing_parent_error(path))?)
}

#[derive(Debug)]
pub struct AcdArchive {
    acd_path: PathBuf,
    #[allow(dead_code)]
    extract_key: String,
    contents: AcdFileContents
}

fn load_data_from_file(file_path: &Path) -> io::Result<Vec<u8>> {
    let f = File::open(file_path)?;
    let mut reader = BufReader::new(f);
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;
    Ok(data)
}

impl AcdArchive {
    pub fn load_from_acd_file(acd_path: &Path) -> Result<AcdArchive> {
        AcdArchive::load_from_acd_file_with_key(acd_path, get_parent_folder_str(acd_path)?)
    }

    pub fn load_from_acd_file_with_key(acd_path: &Path, in_key: &str) -> Result<AcdArchive> {
        let key = generate_acd_key(in_key)?;
        let contents = extract_acd(acd_path, &key)?;
        Ok(AcdArchive{
            acd_path: acd_path.to_path_buf(),
            extract_key: key,
            contents
        })
    }

    pub fn create_from_data_dir(data_dir_path: &Path) -> Result<AcdArchive> {
        let mut contents = AcdFileContents::new();
        for entry in fs::read_dir(data_dir_path)? {
            let entry = entry?;
            if entry.path().is_dir() {
                continue;
            }
            let filename = entry.file_name().to_string_lossy().into_owned();
            let data = load_data_from_file(&entry.path())?;
            contents.files.insert(filename, data);
        }
        Ok(AcdArchive{
            acd_path: data_dir_path.parent().ok_or(missing_parent_error(data_dir_path))?.join("data.acd"),
            extract_key: get_parent_folder_str(data_dir_path)?.to_owned(),
            contents
        })
    }

    pub fn get_file_data(&self, filename: &str) -> Option<Vec<u8>> {
        match self.contents.files.get(filename) {
            Some(data) => {
                Some(data.clone())
            },
            None => None
        }
    }

    pub fn contains_file(&self, filename: &str) -> bool {
        self.contents.files.contains_key(filename)
    }

    pub fn update_file_data(&mut self, filename: String, data: Vec<u8>) -> Option<Vec<u8>> {
        self.contents.files.insert(filename, data)
    }

    pub fn delete_file(&mut self, filename: &str) -> Option<Vec<u8>> {
        self.contents.files.shift_remove(filename)
    }

    pub fn unpack(&self) -> Result<()> {
        self.unpack_to(self.acd_path.parent().ok_or(missing_parent_error(&self.acd_path))?.join("data").as_path())
    }

    pub fn unpack_to(&self, out_path: &Path) -> Result<()> {
        if !out_path.is_dir() {
            fs::create_dir(out_path)?;
        }
        for (filename, unpacked_buffer) in &self.contents.files {
            fs::write(out_path.join(filename), unpacked_buffer)?;
        }
        Ok(())
    }

    pub fn write(&self) -> Result<()> {
        self.write_to(self.acd_path.as_path())
    }

    pub fn write_to(&self, out_path: &Path) -> Result<()> {
        let parent_folder = get_parent_folder_str(out_path)?;
        let key = generate_acd_key(parent_folder)?;
        let mut out_file = File::create(out_path)?;
        for filename in self.contents.files.keys() {
            let mut key_byte_iter = key.chars().cycle();
            let filename_len = filename.len() as u32;
            out_file.write(&filename_len.to_le_bytes())?;
            out_file.write(filename.as_bytes())?;
            let data_len = self.contents.files[filename].len() as u32;
            out_file.write(&data_len.to_le_bytes())?;
            for byte in &self.contents.files[filename] {
                let out_byte = byte + u32::from(key_byte_iter.next().unwrap()) as u8;
                out_file.write(&[out_byte, 0, 0, 0])?;
            }
        }
        out_file.flush()?;
        Ok(())
    }
}

/// Credit for this goes to Luigi Auriemma (me@aluigi.org)
/// This is derived from his quickBMS script which can be found at:
/// https://zenhax.com/viewtopic.php?f=9&t=90&sid=330e7fe17c78d2bfe2d7e8b7227c6143
pub fn generate_acd_key(folder_name: &str) -> Result<String> {
    type KeyVal = i128;

    let mut key_list: Vec<String> = Vec::with_capacity(8);
    let mut push_key_component = |val: KeyVal| { key_list.push((val & 0xff).to_string()) };

    let index_error = |idx: usize| {
        key_error(folder_name, format!("Bad index ({}) into folder name", idx))
    };

    let mut key_1: KeyVal = 0;
    folder_name.chars().for_each(|c| key_1 += u64::from(c) as KeyVal);
    push_key_component(key_1);

    let mut key_2: KeyVal = 0;
    for idx in (0..folder_name.len()-1).step_by(2) {
        key_2 = key_2.wrapping_mul( u64::from(folder_name.chars().nth(idx).ok_or(index_error(idx))?) as KeyVal);
        key_2 = key_2.wrapping_sub(u64::from(folder_name.chars().nth(idx+1).ok_or(index_error(idx+1))?) as KeyVal);
    }
    push_key_component(key_2);

    let mut key_3: KeyVal = 0;
    for idx in (1..folder_name.len()-3).step_by(3) {
        key_3 *= u64::from(folder_name.chars().nth(idx).ok_or(index_error(idx))?) as KeyVal;
        key_3 /= (u64::from(folder_name.chars().nth(idx+1).ok_or(index_error(idx+1))?)) as KeyVal + 0x1b;
        key_3 += -0x1b - u64::from(folder_name.chars().nth(idx-1).ok_or(index_error(idx-1))?) as KeyVal;
    }
    push_key_component(key_3);

    let mut key_4: KeyVal = 0x1683;
    folder_name[1..].chars().for_each(|c| key_4 -= u64::from(c) as KeyVal);
    push_key_component(key_4);

    let mut key_5: KeyVal = 0x42;
    for idx in (1..folder_name.len()-4).step_by(4) {
        let mut tmp = u64::from(folder_name.chars().nth(idx).ok_or(index_error(idx))?) as KeyVal + 0xf;
        tmp *= key_5;
        let mut tmp2 = u64::from(folder_name.chars().nth(idx-1).ok_or(index_error(idx-1))?) as KeyVal + 0xf;
        tmp2 *= tmp;
        tmp2 += 0x16;
        key_5 = tmp2;
    }
    push_key_component(key_5);

    let mut key_6: KeyVal = 0x65;
    folder_name[0..folder_name.len()-2].chars().step_by(2).for_each(|c| key_6 = key_6.wrapping_sub(u64::from(c) as KeyVal));
    push_key_component(key_6);

    let mut key_7: KeyVal = 0xab;
    folder_name[0..folder_name.len()-2].chars().step_by(2).for_each(|c| key_7 %= u64::from(c) as KeyVal);
    push_key_component(key_7);

    let mut key_8: KeyVal = 0xab;
    for idx in 0..folder_name.len()-1 {
        key_8 /= u64::from(folder_name.chars().nth(idx).ok_or(index_error(idx))?) as KeyVal;
        key_8 += u64::from(folder_name.chars().nth(idx+1).ok_or(index_error(idx+1))?) as KeyVal;
    }
    push_key_component(key_8);

    Ok(key_list.join("-"))
}

// If first 4 bytes -> [A9, FB, FF, FF] signifies that the car is DLC
const DLC_BYTE_MARKER: &'static[u8] = &[0xA9, 0xFB, 0xFF, 0xFF];

// Second 4 bytes denotes the DLC pack it belongs to:
#[derive(Debug)]
pub enum DlcPack {
    DreamPack1,
    DreamPack2,
    DreamPack3,
    JapaneseCarPack,
    RedPack,
    TRIPL3Pack,
    PorschePack1,
    PorschePack2,
    PorschePack3,
    ReadytoRace,
    FerrariPack,
    Unknown
}

impl DlcPack {
    pub fn from_bytes(bytes: &[u8]) -> DlcPack {
        match bytes {
            &[0x91, 0x46, 0x0A, 0x00] => DlcPack::DreamPack1,
            &[0xFD, 0xEA, 0x0D, 0x00] => DlcPack::DreamPack2,
            &[0xB1, 0xEB, 0x0B, 0x00] => DlcPack::DreamPack3,
            &[0x87, 0xB7, 0x03, 0x00] => DlcPack::JapaneseCarPack,
            &[0x35, 0x57, 0x0B, 0x00] => DlcPack::RedPack,
            &[0x91, 0xC7, 0x04, 0x00] => DlcPack::TRIPL3Pack,
            &[0xA1, 0x09, 0x0E, 0x00] => DlcPack::PorschePack1,
            &[0x3F, 0xC0, 0x0C, 0x00] => DlcPack::PorschePack2,
            &[0xF2, 0x05, 0x09, 0x00] => DlcPack::PorschePack3,
            &[0xBF, 0xE4, 0x07, 0x00] => DlcPack::ReadytoRace,
            &[0xDA, 0xEB, 0x0D, 0x00] => DlcPack::FerrariPack,
            _ => DlcPack::Unknown
        }
    }
}

#[derive(Debug)]
pub struct AcdFileContents {
    pub dlc_pack: Option<DlcPack>,
    pub files: IndexMap<String, Vec<u8>>
}

impl AcdFileContents {
    pub fn new() -> AcdFileContents {
        AcdFileContents {
            dlc_pack: None,
            files: IndexMap::new()
        }
    }
}

struct PackedData {
    path: String,
    buffer: Vec<u8>,
    current_pos: usize,
}

fn throw_error_if_out_of_bounds(requested_idx: usize, max_idx: usize, path: &String, op_id: &str) -> Result<()> {
    if requested_idx > max_idx {
        return Err(
            AcdError::DecodeError {
                path: path.clone(),
                reason: format!("Failed to parse {}. Reached end of data", op_id)
            }
        );
    }
    Ok(())
}

impl PackedData {
    fn new(path: &Path, buffer: Vec<u8>) -> PackedData {
        let path_str = path.to_string_lossy().to_string();
        PackedData{ path: path_str, buffer, current_pos: 0 }
    }

    fn peek_bytes(&mut self, num_bytes: usize, peek_id: &str) -> Result<&[u8]> {
        throw_error_if_out_of_bounds(self.current_pos+num_bytes, self.buffer.len(), &self.path, peek_id)?;
        Ok(&self.buffer[self.current_pos..(self.current_pos+num_bytes)])
    }

    fn skip_bytes(&mut self, num_bytes: usize) {
        self.current_pos += num_bytes;
    }

    fn read_bytes(&mut self, num_bytes: usize, read_id: &str) -> Result<&[u8]> {
        throw_error_if_out_of_bounds(self.current_pos+num_bytes, self.buffer.len(), &self.path, read_id)?;
        let data = &self.buffer[self.current_pos..(self.current_pos+num_bytes)];
        self.current_pos += num_bytes;
        Ok(data)
    }

    fn parse_length(&mut self, length_id: &str) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_bytes(mem::size_of::<u32>(), length_id)?.try_into()?))
    }

    fn parse_utf8_string(&mut self, string_len: usize, string_id: &str) -> Result<String> {
        match String::from_utf8(self.read_bytes(string_len, string_id)?.to_owned()) {
            Ok(s) => Ok(s),
            Err(err) => {
                Err(
                    AcdError::DecodeError {
                        path: self.path.clone(),
                        reason: format!("Failed to parse {} from UTF-8. {}", string_id, err.to_string())
                    }
                )
            }
        }
    }

    fn has_bytes_remaining(&self) -> bool {
        return self.current_pos < self.buffer.len()
    }
}

/// Credit for this goes to Luigi Auriemma (me@aluigi.org)
/// This is derived from his quickBMS script which can be found at:
/// https://zenhax.com/viewtopic.php?f=9&t=90&sid=330e7fe17c78d2bfe2d7e8b7227c6143
pub fn extract_acd(acd_path: &Path,
                   extraction_key: &str) -> Result<AcdFileContents> {
    let f = File::open(acd_path)?;
    let mut reader = BufReader::new(f);
    let mut packed_buffer = Vec::new();
    reader.read_to_end(&mut packed_buffer)?;

    let mut buffer = PackedData::new(acd_path, packed_buffer);
    let mut out_map = IndexMap::new();

    let mut dlc_pack = None;
    if buffer.peek_bytes(DLC_BYTE_MARKER.len(), "DLC byte marker")? == DLC_BYTE_MARKER {
        buffer.skip_bytes(DLC_BYTE_MARKER.len());
        dlc_pack = Some(
            DlcPack::from_bytes(buffer.read_bytes(4, "DLC Pack id")?)
        );
    }
    while buffer.has_bytes_remaining() {
        // 4 bytes contain the length of filename
        let filename_len = buffer.parse_length("filename length")?;

        // The next 'filename_len' bytes are the filename
        let filename = buffer.parse_utf8_string(filename_len as usize, "filename")?;

        // The next 4 bytes contain the length of the file content
        let content_length = buffer.parse_length(&format!("{} content length", &filename))?;

        // The file content is spread out such that each byte of content is stored in 4 bytes.
        // Read each single byte of content, subtract the value of the extraction key from it and store the result
        // Move along the packed data by 4 bytes to the next byte of content, increment the extraction key position by 1 and repeat
        // Loop back to the start of the extraction key if we hit the end
        // Repeat until we have read the full content for the file
        let mut unpacked_buffer: Vec<u8> = Vec::new();
        let mut key_byte_iter = extraction_key.chars().cycle();
        let content_data = buffer.read_bytes((content_length * 4) as usize, &format!("{} content", &filename))?;
        content_data.iter().step_by(4).for_each(|byte|{
            unpacked_buffer.push(byte.wrapping_sub(u32::from(key_byte_iter.next().unwrap()) as u8));
        });
        debug!("{} - {} bytes", filename, content_length);
        out_map.insert(filename, unpacked_buffer);
    }
    Ok(AcdFileContents{dlc_pack, files: out_map})
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;
    use crate::car::acd_utils::{AcdArchive, generate_acd_key};

    #[test]
    fn derive_acd_key() {
        assert_eq!(generate_acd_key("abarth500").unwrap(), "7-248-6-221-246-250-21-49");
    }

    #[test]
    fn derive_acd_key_from_long_name() {
        assert_eq!(generate_acd_key("ks_maserati_gt_mc_gt4").unwrap(), "16-39-7-162-182-31-30-101");
    }

    #[test]
    fn derive_acd_key_with_large_values() {
        println!("{}", generate_acd_key("dallara_f312").unwrap())
    }

    #[test]
    fn extract_acd() {
        ///~/Downloads/car/RSS_Formula_RSS_4_2024-Assetto_Corsa-v1/content/cars/rss_formula_rss_4_2024
        let path = Path::new("/home/josykes/Downloads/car/RSS_Formula_RSS_4_2024-Assetto_Corsa-v1/content/cars/rss_formula_rss_4_2024/data.acd");
        AcdArchive::load_from_acd_file(path).unwrap().unpack().unwrap();
    }

    #[test]
    fn extract_all_acd_in_folder() {
        let path = Path::new("C:/Program Files (x86)/Steam/steamapps/common/assettocorsa/content/cars/");
        if let Ok(res) = std::fs::read_dir(path) {
            res.for_each(|p| {
                let mut x = p.unwrap().path();
                if x.is_dir() {
                    x.push("data.acd");
                    if x.is_file() {
                        AcdArchive::load_from_acd_file(&x).unwrap().unpack().unwrap();
                    }
                }
            })
        }
    }

    #[test]
    fn read_and_write() {
        let path = Path::new("C:/Program Files (x86)/Steam/steamapps/common/assettocorsa/content/cars/abarth500_s1/data.acd");
        let out_path = Path::new("C:/Program Files (x86)/Steam/steamapps/common/assettocorsa/content/cars/abarth500_s1/testdata.acd");
        let archive = AcdArchive::load_from_acd_file(path).unwrap();
        archive.write_to(out_path).unwrap();
        let mut a = Vec::new();
        File::open(path).unwrap().read_to_end(&mut a).unwrap();
        let mut b = Vec::new();
        File::open(out_path).unwrap().read_to_end(&mut b).unwrap();
        assert_eq!(a, b)
    }


}