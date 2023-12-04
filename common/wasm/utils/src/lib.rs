// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::prelude::*;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "crypto")]
pub mod crypto;

pub mod error;

// will cause messages to be written as if console.log("...") was called
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => ($crate::log(&format_args!($($t)*).to_string()))
}

// will cause messages to be written as if console.debug("...") was called
#[macro_export]
macro_rules! console_debug {
    ($($t:tt)*) => ($crate::debug(&format_args!($($t)*).to_string()))
}

// will cause messages to be written as if console.info("...") was called
#[macro_export]
macro_rules! console_info {
    ($($t:tt)*) => ($crate::info(&format_args!($($t)*).to_string()))
}

// will cause messages to be written as if console.warn("...") was called
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
    pub fn debug(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn info(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

#[cfg(feature = "sleep")]
pub async fn sleep(ms: i32) -> Result<(), wasm_bindgen::JsValue> {
    let promise = js_sys::Promise::new(&mut |yes, _| {
        let win = web_sys::window().expect("no window available!");
        win.set_timeout_with_callback_and_timeout_and_arguments_0(&yes, ms)
            .unwrap();
    });
    let js_fut = wasm_bindgen_futures::JsFuture::from(promise);
    js_fut.await?;
    Ok(())
}

#[wasm_bindgen]
#[cfg(feature = "panic-hook")]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    console_error_panic_hook::set_once();
}
