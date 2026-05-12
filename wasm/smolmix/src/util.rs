// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

//! Internal helpers shared by every module: byte-preview formatting and the
//! `debug_log!` / `debug_error!` macros gated on the `debug` feature.
//!
//! In release builds the macros expand to `()`, so calls cost nothing and the
//! format-argument expressions are elided by the compiler. Six call sites in
//! `tunnel.rs` and `ipr.rs` remain unconditional (tunnel start/IPR connect/
//! tunnel ready/shutdown); those are user-visible lifecycle events worth
//! shipping in production.

/// Hex preview of a buffer (truncated with ` ...` suffix when over `max_bytes`).
///
/// Used for debug logs of binary payloads, never used to construct data.
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

/// Debug-only `console.log`. Expands to `()` without the `debug` feature.
///
/// The inner `nym_wasm_utils::console_log!` call carries a `#[cfg]` attribute
/// so the format arguments are stripped entirely when the feature is off.
macro_rules! debug_log {
    ($($arg:tt)*) => {{
        #[cfg(feature = "debug")]
        ::nym_wasm_utils::console_log!($($arg)*);
    }};
}
pub(crate) use debug_log;

/// Debug-only `console.error`. See [`debug_log!`].
macro_rules! debug_error {
    ($($arg:tt)*) => {{
        #[cfg(feature = "debug")]
        ::nym_wasm_utils::console_error!($($arg)*);
    }};
}
pub(crate) use debug_error;
