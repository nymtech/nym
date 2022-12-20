// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(not(target_arch = "wasm32"))]
mod non_wasm;

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use non_wasm::*;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
