// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
mod ephemeral_receiver;
#[cfg(target_arch = "wasm32")]
pub mod error;
#[cfg(target_arch = "wasm32")]
mod helpers;
#[cfg(target_arch = "wasm32")]
pub mod tester;
#[cfg(target_arch = "wasm32")]
pub mod types;

#[cfg(target_arch = "wasm32")]
pub use wasm_client_core::set_panic_hook;
