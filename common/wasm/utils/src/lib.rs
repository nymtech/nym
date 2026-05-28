// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::prelude::*;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "crypto")]
pub mod crypto;

pub mod error;

#[doc(hidden)]
pub static DEBUG_LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable or disable `debug_log!` / `debug_error!` output at runtime.
///
/// Consumers call this from their own init path when their own `debug`
/// (or equivalent) feature is on. The default is `false`, so the macros
/// compile to a single relaxed-load + branch and otherwise no-op.
///
/// Also exposed to JS as `setDebugLogging(enabled: boolean)` for ad-hoc
/// toggling without rebuilding.
#[wasm_bindgen(js_name = "setDebugLogging")]
pub fn set_debug_logging(enabled: bool) {
    DEBUG_LOGGING_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Read the current debug-logging state. Used by the macros below.
#[inline]
pub fn debug_logging_enabled() -> bool {
    DEBUG_LOGGING_ENABLED.load(Ordering::Relaxed)
}

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

/// `console.log` gated behind the runtime [`DEBUG_LOGGING_ENABLED`] flag.
///
/// Format-args evaluation stays inside the `if` arm, so it's skipped at
/// runtime when logging is off. Consumers turn this on via
/// [`set_debug_logging`] from their own init path (typically when their
/// own `debug` feature is enabled).
#[macro_export]
macro_rules! debug_log {
    ($($t:tt)*) => {{
        if $crate::debug_logging_enabled() {
            $crate::console_log!($($t)*);
        }
    }};
}

/// `console.error` gated behind the runtime [`DEBUG_LOGGING_ENABLED`] flag.
/// See [`debug_log!`] for semantics.
#[macro_export]
macro_rules! debug_error {
    ($($t:tt)*) => {{
        if $crate::debug_logging_enabled() {
            $crate::console_error!($($t)*);
        }
    }};
}

/// Hex preview of a buffer, truncated with ` ...` when over `max_bytes`.
/// Useful for `console.log`-style binary debug output.
pub fn hex_preview(buf: &[u8], max_bytes: usize) -> String {
    let len = buf.len().min(max_bytes);
    let hex: String = buf[..len]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if buf.len() > max_bytes {
        format!("{hex} ...")
    } else {
        hex
    }
}

#[wasm_bindgen]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
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
