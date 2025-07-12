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
extern crate core;

use std::cell::RefCell;
use std::ffi::{c_float, c_ulong, CStr, CString};
use std::fs;
use std::os::raw::c_char;
use std::path::{PathBuf};

use windows::{Win32::Foundation::*, Win32::System::SystemServices::*, };

use crate_engine;
use crate_engine::{CrateEngine};
use crate_engine::direct_export::LuaDataContainer;

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

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = RefCell::new(None);
}

fn set_last_error(err: &str) {
    let c_string = CString::new(err).unwrap();
    LAST_ERROR.with(|last_error| {
        *last_error.borrow_mut() = Some(c_string);
    });
}

fn clear_last_error() {
    LAST_ERROR.with(|last_error| {
        *last_error.borrow_mut() = None;
    })
}

#[no_mangle]
pub extern "C" fn get_last_error() -> *const c_char {
    LAST_ERROR.with(|last_error| {
        match &*last_error.borrow() {
            Some(err) => err.as_ptr(),
            None => std::ptr::null(),
        }
    })
}

#[no_mangle]
pub extern "C" fn init(script_version: u32) -> *mut LuaDataContainer {
    let mut data: Box<LuaDataContainer> = Box::new(LuaDataContainer::new());
    data.exporter_script_version = script_version;
    Box::into_raw(data)
}

#[no_mangle]
pub extern "C" fn add_string(instance: *mut LuaDataContainer,
                             group: *const c_char,
                             key: *const c_char,
                             val: *const c_char) {
    let data = unsafe { &mut*(instance) };
    let group_cstr = unsafe { CStr::from_ptr(group) };
    let key_cstr = unsafe { CStr::from_ptr(key) };
    let val_cstr = unsafe { CStr::from_ptr(val) };
    data.add_string(String::from_utf8_lossy(group_cstr.to_bytes()).to_string(),
                    String::from_utf8_lossy(key_cstr.to_bytes()).to_string(),
                    String::from_utf8_lossy(val_cstr.to_bytes()).to_string());
}

#[no_mangle]
pub extern "C" fn add_float(instance: *mut LuaDataContainer,
                            group: *const c_char,
                            key: *const c_char,
                            val: c_float) {
    let data = unsafe { &mut*(instance) };
    let group_cstr = unsafe { CStr::from_ptr(group) };
    let key_cstr = unsafe { CStr::from_ptr(key) };
    data.add_float(String::from_utf8_lossy(group_cstr.to_bytes()).to_string(),
                   String::from_utf8_lossy(key_cstr.to_bytes()).to_string(),
                   val);
}

#[no_mangle]
pub extern "C" fn add_curve_data(instance: *mut LuaDataContainer,
                                 curve_name: *const c_char,
                                 index: c_ulong,
                                 val: c_float) {
    let data = unsafe { &mut*(instance) };
    let curve_name_cstr = unsafe { CStr::from_ptr(curve_name) };
    data.add_curve_data(String::from_utf8_lossy(curve_name_cstr.to_bytes()).to_string(),
                        index as usize,
                        val);
}

#[no_mangle]
pub extern "C" fn dump_json(instance: *mut LuaDataContainer,
                            path_char: *const c_char) -> bool
{
    clear_last_error();
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
        Err(e) => {
            set_last_error(&format!("Failed to create directory ({}) to write json dump to. {}",
                                    parent_path.display(), e.to_string()));
            return false;
        }
    }
    let json_string = match serde_json::to_string_pretty(&data) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Failed to encode data as json. {}",
                                    e.to_string()));
            return false;
        }
    };
    let file_path =
        utils::filesystem::create_safe_filename_in_path(&parent_path,
                                                        &data.deduce_engine_name(),
                                                        "json");
    match fs::write(file_path, json_string) {
        Ok(_) => true,
        Err(e) => {
            set_last_error(&format!("Failed to write json file. {}",
                                    e.to_string()));
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn finalise(instance: *mut LuaDataContainer,
                           path_char: *const c_char) -> bool
{
    clear_last_error();
    let data = unsafe {Box::from_raw(instance)};
    let path_cstr = unsafe { CStr::from_ptr(path_char) };
    let path_str = String::from_utf8_lossy(path_cstr.to_bytes()).to_string();
    let mut path = PathBuf::from(path_str);
    if path.is_file() {
        path = match path.parent() {
            None => PathBuf::new(),
            Some(p) => PathBuf::from(p)
        }
    }

    let result = CrateEngine::from_exporter_data(crate_engine::direct_export::Data::from_lua_data(*data));
    match result {
        Ok(eng) => {
            match fs::create_dir_all(&path) {
                Ok(_) => {}
                Err(e) => {
                    set_last_error(&format!("Failed to create directory ({} )to write crate engine to. {}",
                                            path.display(), e.to_string()));
                    return false;
                }
            }
            return match eng.write_to_path(path) {
                Ok(_) => {
                    true
                }
                Err(e) => {
                    set_last_error(&format!("Failed to write crate engine file. {}",
                                            e.to_string()));
                    false
                }
            }
        }
        Err(_) => false
    }
}

#[no_mangle]
pub extern "C" fn destroy(struct_instance: *mut LuaDataContainer) {
    unsafe { drop(Box::from_raw(struct_instance)); }
}
