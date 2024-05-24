// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
mod active_requests;
#[cfg(target_arch = "wasm32")]
mod client;
#[cfg(target_arch = "wasm32")]
mod config;
#[cfg(target_arch = "wasm32")]
pub mod error;
#[cfg(target_arch = "wasm32")]
mod fetch;
#[cfg(target_arch = "wasm32")]
mod go_bridge;
#[cfg(target_arch = "wasm32")]
mod harbourmaster;
#[cfg(target_arch = "wasm32")]
mod helpers;
#[cfg(target_arch = "wasm32")]
mod request_writer;
#[cfg(target_arch = "wasm32")]
mod socks_helpers;

#[cfg(target_arch = "wasm32")]
pub(crate) use fetch::{mix_fetch_client, RequestId};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
#[cfg(target_arch = "wasm32")]
pub fn main() {
    wasm_utils::console_log!("[rust main]: rust module loaded");
    wasm_utils::console_log!(
        "mix fetch version used: {}",
        nym_bin_common::bin_info_owned!()
    );
}
