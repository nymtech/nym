// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod active_requests;
mod client;
mod config;
pub mod error;
mod fetch;
mod go_bridge;
mod harbourmaster;
mod helpers;
mod request_writer;
mod socks_helpers;

pub(crate) use fetch::{mix_fetch_client, RequestId, MIX_FETCH};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    wasm_utils::console_log!("[rust main]: rust module loaded")
}
