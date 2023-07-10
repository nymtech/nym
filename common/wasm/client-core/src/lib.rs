// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// that's so stupid...
#[cfg(target_arch = "wasm32")]
pub mod config;

#[cfg(target_arch = "wasm32")]
pub mod error;

#[cfg(target_arch = "wasm32")]
pub mod helpers;

#[cfg(target_arch = "wasm32")]
pub mod storage;

#[cfg(target_arch = "wasm32")]
pub mod topology;

// re-export types for ease of use
pub use nym_client_core::*;
pub use nym_client_core::{
    client::key_manager::ManagedKeys, error::ClientCoreError, init::InitialisationDetails,
};
pub use nym_gateway_client::{error::GatewayClientError, GatewayClient};
pub use nym_sphinx::{
    addressing::{clients::Recipient, nodes::NodeIdentity},
    params::PacketType,
    receiver::ReconstructedMessage,
};
pub use nym_task;
pub use nym_topology::{HardcodedTopologyProvider, MixLayer, NymTopology, TopologyProvider};
pub use nym_validator_client::nym_api::Client as ApiClient;
// TODO: that's a very nasty import path. it should come from contracts instead!
pub use nym_validator_client::client::IdentityKey;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
