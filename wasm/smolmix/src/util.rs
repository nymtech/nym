// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Re-exports of the shared debug-logging helpers from `nym-wasm-utils`.
//!
//! Both macros gate on a runtime flag in `nym-wasm-utils`, which smolmix
//! flips on in `lib.rs::main()` when its own `debug` feature is enabled
//! (see `nym_wasm_utils::set_debug_logging`). `hex_preview` is the
//! binary-buffer formatter used inside those macros.

pub(crate) use nym_wasm_utils::debug_error;
pub(crate) use nym_wasm_utils::debug_log;
pub(crate) use nym_wasm_utils::hex_preview;

/// MSRV-safe equivalent of `str::floor_char_boundary` (stable in 1.91; workspace
/// MSRV is 1.87). Returns the largest byte index `≤ target` where `s.is_char_boundary(i)`
/// holds; useful for truncating a string at a safe UTF-8 boundary.
pub(crate) fn floor_char_boundary(s: &str, target: usize) -> usize {
    let mut i = target.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
