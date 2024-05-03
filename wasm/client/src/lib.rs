// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
pub mod client;
#[cfg(target_arch = "wasm32")]
pub mod config;
#[cfg(target_arch = "wasm32")]
pub mod encoded_payload_helper;
#[cfg(target_arch = "wasm32")]
pub mod error;
#[cfg(target_arch = "wasm32")]
mod helpers;
#[cfg(target_arch = "wasm32")]
mod response_pusher;

#[cfg(target_arch = "wasm32")]
pub use wasm_client_core::set_panic_hook;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
#[cfg(target_arch = "wasm32")]
pub fn main() {
    wasm_utils::console_log!("[rust main]: rust module loaded");
    wasm_utils::console_log!(
        "wasm client version used: {}",
        nym_bin_common::bin_info_owned!()
    );
}
