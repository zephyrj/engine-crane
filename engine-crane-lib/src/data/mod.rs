/*
 * Copyright (c):
 * 2025 zephyrj
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


use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use directories::BaseDirs;
use tracing::warn;

use utils::filesystem::get_filetypes_in_path;
use crate_engine::{CrateEngine, CrateEngineMetadata, CrateEngineData, FromBeamNGModOptions};
use crate_engine::source::DataSource;

const LOCAL_DATA_DIRNAME: &'static str = "EngineCrane";
const DEFAULT_CRATE_ENGINE_DIRNAME: &'static str = "crate";

#[derive(Debug, Default)]
pub struct CrateEngineFilter {
    source: Option<DataSource>,
    min_data_version: Option<u16>
}

impl CrateEngineFilter {
    pub fn new(source: Option<DataSource>, min_data_version: Option<u16>) -> Self {
        Self {
            source,
            min_data_version
        }
    }

    pub fn set_source_filter(&mut self, source: Option<DataSource>) {
        self.source = source;
    }

    pub fn set_min_data_version_filter(&mut self, data_version: Option<u16>) {
        self.min_data_version = data_version;
    }

    pub fn matches(&self, metadata: &CrateEngineMetadata) -> bool {
        if let Some(source) = &self.source {
            if &metadata.get_source() != source {
                return false;
            }
        }
        if let Some(data_version) = &self.min_data_version {
            if &metadata.data_version() != data_version {
                return false;
            }
        }
        true
    }
}

pub struct CrateEngineFilterBuilder {
    source: Option<DataSource>,
    min_data_version: Option<u16>
}

impl CrateEngineFilterBuilder {
    pub fn new() -> Self {
        Self {
            source: None,
            min_data_version: None
        }
    }

    pub fn source(mut self, source: DataSource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn min_data_version(mut self, data_version: u16) -> Self {
        self.min_data_version = Some(data_version);
        self
    }

    pub fn build(self) -> CrateEngineFilter {
        CrateEngineFilter::new(self.source, self.min_data_version)
    }
}

pub fn find_crate_engines_in_path(path: &Path, filter: Option<CrateEngineFilter>) -> std::io::Result<BTreeMap<PathBuf, CrateEngineMetadata>> {
    let mut found_metadata = BTreeMap::new();
    let paths = get_filetypes_in_path(path, crate_engine::CRATE_ENGINE_FILE_SUFFIX)?;
    for path in paths.into_iter() {
        match File::open(&path) {
            Ok(mut f) => match CrateEngineMetadata::from_reader(&mut f) {
                Ok(m) => {
                    if let Some(filter) = &filter {
                        if !filter.matches(&m) {
                            continue;
                        }
                    }
                    found_metadata.insert(path, m);
                }
                Err(e) => {
                    warn!("Error occurred for {}. {}", path.display(), e);
                }
            }
            Err(e) => warn!("Couldn't open {}. {}", path.display(), e.to_string())
        }
    }
    Ok(found_metadata)
}

#[cfg(target_os = "windows")]
fn backup_data_dir() -> PathBuf {
    let username = whoami::username();
    PathBuf::from_iter(["C:", "Users", &username, "AppData", "Local"])
}

#[cfg(target_os = "linux")]
fn backup_data_dir() -> PathBuf {
    let username = whoami::username();
    PathBuf::from_iter(["home", &username, ".local", "share"])
}


pub fn get_local_app_data_path() -> PathBuf {
    let mut local_data_root : PathBuf = match BaseDirs::new() {
        None => backup_data_dir(),
        Some(basedirs) => { basedirs.data_local_dir().to_path_buf() }
    };
    local_data_root.push(LOCAL_DATA_DIRNAME);
    local_data_root
}

pub fn get_default_crate_engine_path() -> PathBuf {
    let mut path = get_local_app_data_path();
    path.push(DEFAULT_CRATE_ENGINE_DIRNAME);
    path
}
