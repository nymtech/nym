// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(target_arch = "wasm32", feature = "client"))]
pub use nym_client_wasm as client;

#[cfg(all(target_arch = "wasm32", feature = "node-tester"))]
pub use nym_node_tester_wasm as node_tester;

#[cfg(all(target_arch = "wasm32", feature = "mix-fetch"))]
pub use mix_fetch_wasm as mix_fetch;
