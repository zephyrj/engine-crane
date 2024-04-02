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
use std::ffi::{c_float, c_int, CStr};
use std::fs;
use std::os::raw::c_char;
use std::path::{PathBuf};
use toml;
use toml::ser::Error;


use windows::{Win32::Foundation::*, Win32::System::SystemServices::*, };

use crate_engine;
use crate_engine::{CrateEngine, CrateEngineMetadata};

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(
    dll_module: HINSTANCE,
    call_reason: u32,
    _: *mut ())
    -> bool
{
    match call_reason {
        DLL_PROCESS_ATTACH => attach(),
        DLL_PROCESS_DETACH => detach(),
        _ => ()
    }

    true
}

fn attach() {
}

fn detach() {
}

#[no_mangle]
pub extern fn init(script_version: u32) -> *mut crate_engine::direct_export::DataV1 {
    let mut data: Box<crate_engine::direct_export::DataV1> = Box::new(crate_engine::direct_export::DataV1::new());
    data.exporter_script_version = script_version;
    Box::into_raw(data)
}

#[no_mangle]
pub extern fn add_string(instance: *mut crate_engine::direct_export::DataV1,
                         key: *const c_char,
                         val: *const c_char) {
    let mut data = unsafe { &mut*(instance) };
    let key_cstr = unsafe { CStr::from_ptr(key) };
    let val_cstr = unsafe { CStr::from_ptr(val) };
    data.string_data.insert(String::from_utf8_lossy(key_cstr.to_bytes()).to_string(),
                            String::from_utf8_lossy(val_cstr.to_bytes()).to_string());
}

#[no_mangle]
pub extern fn add_float(instance: *mut crate_engine::direct_export::DataV1,
                        key: *const c_char,
                        val: c_float) {
    let mut data = unsafe { &mut*(instance) };
    let key_cstr = unsafe { CStr::from_ptr(key) };
    data.float_data.insert(String::from_utf8_lossy(key_cstr.to_bytes()).to_string(),
                           val);
}

#[no_mangle]
pub extern fn dump_toml(instance: *mut crate_engine::direct_export::DataV1,
                        path_char: *const c_char) -> bool
{
    let data = unsafe { &mut*(instance) };
    let path_cstr = unsafe { CStr::from_ptr(path_char) };
    let path_str = String::from_utf8_lossy(path_cstr.to_bytes()).to_string();
    let mut parent_path = PathBuf::from(path_str);
    if parent_path.is_file() {
        parent_path = match parent_path.parent() {
            None => PathBuf::new(),
            Some(p) => PathBuf::from(p)
        }
    }
    match fs::create_dir_all(&parent_path) {
        Ok(_) => {}
        Err(_) => {
            return false;
        }
    }
    match toml::to_string(data) {
        Ok(toml_string) => {
            match fs::write(parent_path.join("test_data.toml"), toml_string) {
                Ok(_) => true,
                Err(_) => false
            }
        }
        Err(_) => {
            false
        }
    }
}

#[no_mangle]
pub extern fn finalise(instance: *mut crate_engine::direct_export::DataV1,
                       path_char: *const c_char) -> bool
{
    let data = unsafe {Box::from_raw(instance)};
    let result = CrateEngine::from_exporter_data(crate_engine::direct_export::Data::V1(*data));
    match result {
        Ok(eng) => {
            let path_cstr = unsafe { CStr::from_ptr(path_char) };
            let path_str = String::from_utf8_lossy(path_cstr.to_bytes()).to_string();
            let mut path = PathBuf::from(path_str);
            if path.is_file() {
                path = match path.parent() {
                    None => PathBuf::new(),
                    Some(p) => PathBuf::from(p)
                }
            }
            match fs::create_dir_all(path) {
                Ok(_) => {}
                Err(e) => {
                    return false;
                }
            }
            // Write file
            //eng.serialize_to()
            true
        }
        Err(_) => false
    }
}

#[no_mangle]
pub extern fn destroy(struct_instance: *mut crate_engine::direct_export::DataV1) {
    unsafe { drop(Box::from_raw(struct_instance)); }
}

