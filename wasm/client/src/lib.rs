// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
pub mod client;

#[cfg(target_arch = "wasm32")]
pub mod config;

#[cfg(target_arch = "wasm32")]
pub mod error;

#[cfg(target_arch = "wasm32")]
mod helpers;

#[cfg(target_arch = "wasm32")]
mod response_pusher;

#[cfg(target_arch = "wasm32")]
pub use wasm_client_core::set_panic_hook;
