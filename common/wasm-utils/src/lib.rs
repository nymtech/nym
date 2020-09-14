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

use wasm_bindgen::prelude::*;

pub mod websocket;

// will cause messages to be written as if console.log("...") was called
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => ($crate::log(&format_args!($($t)*).to_string()))
}

// will cause messages to be written as if console.warm("...") was called
#[macro_export]
macro_rules! console_warn {
    ($($t:tt)*) => ($crate::warn(&format_args!($($t)*).to_string()))
}

// will cause messages to be written as if console.error("...") was called
#[macro_export]
macro_rules! console_error {
    ($($t:tt)*) => ($crate::error(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}
