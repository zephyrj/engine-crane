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
use std::ffi::CStr;
use std::os::raw::c_char;
use windows::{ Win32::Foundation::*, Win32::System::SystemServices::*, };



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
pub extern fn build_crate_engine_from_toml(toml_data: *const c_char) -> usize {
    let cstr = unsafe { CStr::from_ptr(toml_data) };
    let toml_string = String::from_utf8_lossy(cstr.to_bytes()).to_string();
}


#[no_mangle]
pub extern fn init() -> *const c_char {
    let data = Box::new(direct_export::DataV1)
}

