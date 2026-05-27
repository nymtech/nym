// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Re-exports of the shared debug-logging helpers from `nym-wasm-utils`.
//!
//! Both macros gate on the calling crate's `debug` feature, so smolmix
//! controls verbose tracing via its own feature flag without affecting other
//! consumers of `nym-wasm-utils`. See `nym_wasm_utils::hex_preview` for the
//! binary-buffer formatter.

pub(crate) use nym_wasm_utils::debug_error;
pub(crate) use nym_wasm_utils::debug_log;
// Only referenced from inside `debug_log!(...)` macros, which compile to `()`
// when the `debug` feature is off — so the import is unused in release builds.
#[cfg(feature = "debug")]
pub(crate) use nym_wasm_utils::hex_preview;
