use std::fs::File;
use std::io;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use tracing::info;
use crate::assetto_corsa::traits::{_DataInterfaceI, DataInterface};

#[derive(Debug)]
pub struct DataFolderInterface {
    data_folder_path: PathBuf
}

impl DataFolderInterface {
    pub(crate) fn new(path: &Path) -> Self {
        DataFolderInterface { data_folder_path: path.to_path_buf() }
    }
}

impl _DataInterfaceI for DataFolderInterface {
    fn load(&self) {}

    fn get_file_data(&self, filename: &str) -> io::Result<Vec<u8>> {
        let file_path = (&self.data_folder_path).join(Path::new(filename));
        info!("Trying to load {}", file_path.display());
        let f = File::open(file_path)?;
        let mut reader = BufReader::new(f);
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(data)
    }

    fn write_file_data(&mut self, filename: &str, data: Vec<u8>) -> io::Result<()>{
        let file_path = (&self.data_folder_path).join(Path::new(filename));
        let mut f = File::create(file_path)?;
        f.write_all(data.as_slice())
    }

    fn delete_file(&mut self, filename: &str) -> io::Result<()> {
        let file_path = (&self.data_folder_path).join(Path::new(filename));
        std::fs::remove_file(&file_path)
    }
}

impl DataInterface for DataFolderInterface {}
