// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
mod bandwidth;
#[cfg(target_arch = "wasm32")]
mod credential;
#[cfg(target_arch = "wasm32")]
mod error;
#[cfg(target_arch = "wasm32")]
mod helpers;
#[cfg(target_arch = "wasm32")]
mod opts;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
#[cfg(target_arch = "wasm32")]
pub fn main() {
    wasm_utils::console_log!("[rust main]: rust module loaded");
    wasm_utils::console_log!(
        "credential client version used: {:#?}",
        nym_bin_common::bin_info!()
    );
    wasm_utils::console_log!("[rust main]: setting panic hook");
    wasm_utils::set_panic_hook();
}
