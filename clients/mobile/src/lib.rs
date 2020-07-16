// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::ffi::{CStr};
use std::os::raw::c_char;
use std::env;

mod built_info;
mod config;
mod commands;
mod node;

#[cfg(target_os = "android")]
mod android;

#[no_mangle]
pub unsafe extern "C" fn init(id: *const c_char) {

    env::set_var("RUST_BACKTRACE", "full");
    env::set_var("RUST_APP_LOG", "debug");

    let id_value = CStr::from_ptr(id);
    let id_str = match id_value.to_str() {
        Ok(s) => s,
        Err(_) => "default_id",
    };
    
    println!("Id: {}", id_str.to_string());
    commands::init::execute(id_str.to_string());
}

#[no_mangle]
pub unsafe extern "C" fn run(id: *const c_char, host: *const c_char) {
    
    env::set_var("RUST_BACKTRACE", "full");
    env::set_var("RUST_APP_LOG", "debug");

    let id_value = CStr::from_ptr(id);
    let id_str = match id_value.to_str() {
        Ok(s) => s,
        Err(_) => "default_id",
    };

    let host_value = CStr::from_ptr(host);
    let host_str = match host_value.to_str() {
        Ok(s) => s,
        Err(_) => "0.0.0.0",
    };

    println!("{}", id_str.to_string());

    commands::run::execute(id_str.to_string(), host_str.to_string());
}

