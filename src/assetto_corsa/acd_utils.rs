use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::{fs, io, mem};
use indexmap::IndexMap;
use std::path::{Path, PathBuf};
use tracing::debug;
use thiserror::Error;
use itertools::Itertools;

pub type Result<T> = std::result::Result<T, AcdError>;

#[derive(Error, Debug)]
pub enum AcdError {
    #[error("io error")]
    IoError(#[from] io::Error),
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

pub struct AcdArchive {
    acd_path: PathBuf,
    extract_key: String,
    contents: IndexMap<String, Vec<u8>>
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

impl AcdArchive {
    pub fn load_from_path(acd_path: &Path) -> Result<AcdArchive> {
        AcdArchive::load_from_path_with_parent(acd_path, get_parent_folder_str(acd_path)?)
    }

    pub fn load_from_path_with_parent(acd_path: &Path, parent: &str) -> Result<AcdArchive> {
        let key = generate_acd_key(parent)?;
        let contents = extract_acd(acd_path, &key)?;
        Ok(AcdArchive{
            acd_path: acd_path.to_path_buf(),
            extract_key: key,
            contents
        })
    }

    pub fn unpack(&self) -> Result<()> {
        self.unpack_to(self.acd_path.parent().ok_or(missing_parent_error(&self.acd_path))?.join("data").as_path())
    }

    pub fn unpack_to(&self, out_path: &Path) -> Result<()> {
        if !out_path.is_dir() {
            std::fs::create_dir(out_path).unwrap();
        }
        for (filename, unpacked_buffer) in &self.contents {
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
        for filename in self.contents.keys() {
            let mut key_byte_iter = key.chars().cycle();
            let filename_len = filename.len() as u32;
            out_file.write(&filename_len.to_le_bytes())?;
            out_file.write(filename.as_bytes())?;
            let data_len = self.contents[filename].len() as u32;
            out_file.write(&data_len.to_le_bytes())?;
            for byte in &self.contents[filename] {
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
    let mut key_list: Vec<String> = Vec::with_capacity(8);
    let mut push_key_component = |val: i64| { key_list.push((val & 0xff).to_string()) };

    let index_error = |idx: usize| {
        key_error(folder_name, format!("Bad index ({}) into folder name", idx))
    };

    let mut key_1 = 0_i64;
    folder_name.chars().for_each(|c| key_1 += u64::from(c) as i64);
    push_key_component(key_1);

    let mut key_2: i64 = 0;
    for idx in (0..folder_name.len()-1).step_by(2) {
        key_2 *= u64::from(folder_name.chars().nth(idx).ok_or(index_error(idx))?) as i64;
        key_2 -= u64::from(folder_name.chars().nth(idx+1).ok_or(index_error(idx+1))?) as i64;
    }
    push_key_component(key_2);

    let mut key_3: i64 = 0;
    for idx in (1..folder_name.len()-3).step_by(3) {
        key_3 *= u64::from(folder_name.chars().nth(idx).ok_or(index_error(idx))?) as i64;
        key_3 /= (u64::from(folder_name.chars().nth(idx+1).ok_or(index_error(idx+1))?) as i64) + 0x1b;
        key_3 += -0x1b - u64::from(folder_name.chars().nth(idx-1).ok_or(index_error(idx-1))?) as i64;
    }
    push_key_component(key_3);

    let mut key_4 = 0x1683_i64;
    folder_name[1..].chars().for_each(|c| key_4 -= u64::from(c) as i64);
    push_key_component(key_4);

    let mut key_5 = 0x42_i64;
    for idx in (1..folder_name.len()-4).step_by(4) {
        let mut tmp = u64::from(folder_name.chars().nth(idx).ok_or(index_error(idx))?) as i64 + 0xf;
        tmp *= key_5;
        let mut tmp2 = u64::from(folder_name.chars().nth(idx-1).ok_or(index_error(idx-1))?) as i64 + 0xf;
        tmp2 *= tmp;
        tmp2 += 0x16;
        key_5 = tmp2;
    }
    push_key_component(key_5);

    let mut key_6 = 0x65_i64;
    folder_name[0..folder_name.len()-2].chars().step_by(2).for_each(|c| key_6 -= u64::from(c) as i64 );
    push_key_component(key_6);

    let mut key_7 = 0xab_i64;
    folder_name[0..folder_name.len()-2].chars().step_by(2).for_each(|c| key_7 %= u64::from(c) as i64 );
    push_key_component(key_7);

    let mut key_8 = 0xab;
    for idx in 0..folder_name.len()-1 {
        key_8 /= u64::from(folder_name.chars().nth(idx).ok_or(index_error(idx))?) as i64;
        key_8 += u64::from(folder_name.chars().nth(idx+1).ok_or(index_error(idx+1))?) as i64
    }
    push_key_component(key_8);

    Ok(key_list.join("-"))
}

/// Credit for this goes to Luigi Auriemma (me@aluigi.org)
/// This is derived from his quickBMS script which can be found at:
/// https://zenhax.com/viewtopic.php?f=9&t=90&sid=330e7fe17c78d2bfe2d7e8b7227c6143
pub fn extract_acd(acd_path: &Path,
                   extraction_key: &str) -> Result<IndexMap<String, Vec<u8>>> {
    let f = File::open(acd_path)?;
    let mut reader = BufReader::new(f);
    let mut packed_buffer = Vec::new();
    reader.read_to_end(&mut packed_buffer)?;

    let mut out_map = IndexMap::new();
    type LengthField = u32;
    let mut current_pos: usize = 0;
    while current_pos < packed_buffer.len() {
        // 4 bytes contain the length of filename
        let filename_len = LengthField::from_le_bytes(packed_buffer[current_pos..(current_pos+mem::size_of::<LengthField>())].try_into().expect("Failed to parse filename length"));
        current_pos += mem::size_of::<LengthField>();

        // The next 'filename_len' bytes are the filename
        let filename = String::from_utf8(packed_buffer[current_pos..(current_pos + filename_len as usize)].to_owned()).expect("Failed to parse filename");
        current_pos += filename_len as usize;

        // The next 4 bytes contain the length of the file content
        let mut content_length = LengthField::from_le_bytes(packed_buffer[current_pos..(current_pos+mem::size_of::<LengthField>())].try_into().expect("Failed to parse filename length"));
        current_pos += mem::size_of::<LengthField>();

        // The file content is spread out such that each byte of content is stored in 4 bytes.
        // Read each single byte of content, subtract the value of the extraction key from it and store the result
        // Move along the packed data by 4 bytes to the next byte of content, increment the extraction key position by 1 and repeat
        // Loop back to the start of the extraction key if we hit the end
        // Repeat until we have read the full content for the file
        let mut unpacked_buffer: Vec<u8> = Vec::new();
        let mut key_byte_iter = extraction_key.chars().cycle();
        packed_buffer[current_pos..current_pos+(content_length*4) as usize].iter().step_by(4).for_each(|byte|{
            unpacked_buffer.push(byte - u32::from(key_byte_iter.next().unwrap()) as u8);
        });

        debug!("{} - {} bytes", filename, content_length);
        out_map.insert(filename, unpacked_buffer);
        current_pos += (content_length*4) as usize;
    }
    Ok(out_map)
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;
    use crate::assetto_corsa::acd_utils::{AcdArchive, generate_acd_key};

    #[test]
    fn derive_acd_key() {
        assert_eq!(generate_acd_key("abarth500").unwrap(), "7-248-6-221-246-250-21-49");
    }

    #[test]
    fn extract_acd() {
        let path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/abarth500_s1/data.acd");
        let out_path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/abarth500_s1/data");
        AcdArchive::load_from_path(path).unwrap().unpack().unwrap();
    }

    #[test]
    fn read_and_write() {
        let path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/abarth500_s1/data.acd");
        let out_path = Path::new("/home/josykes/.steam/debian-installation/steamapps/common/assettocorsa/content/cars/abarth500_s1/testdata.acd");
        let archive = AcdArchive::load_from_path(path).unwrap();
        archive.write_to(out_path).unwrap();
        let mut a = Vec::new();
        File::open(path).unwrap().read_to_end(&mut a).unwrap();
        let mut b = Vec::new();
        File::open(out_path).unwrap().read_to_end(&mut b).unwrap();
        assert_eq!(a, b)
    }
}