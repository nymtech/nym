// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `debug_log!` / `debug_error!` macros (gated on the `debug` feature) and
//! `hex_preview` for binary debug logs.

/// Hex preview of a buffer, truncated with ` ...` when over `max_bytes`.
pub(crate) fn hex_preview(buf: &[u8], max_bytes: usize) -> String {
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

/// `console.log` gated behind the `debug` feature (`()` in release).
macro_rules! debug_log {
    ($($arg:tt)*) => {{
        #[cfg(feature = "debug")]
        ::nym_wasm_utils::console_log!($($arg)*);
    }};
}
pub(crate) use debug_log;

/// `console.error` gated behind the `debug` feature.
macro_rules! debug_error {
    ($($arg:tt)*) => {{
        #[cfg(feature = "debug")]
        ::nym_wasm_utils::console_error!($($arg)*);
    }};
}
pub(crate) use debug_error;
