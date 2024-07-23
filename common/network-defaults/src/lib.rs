// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod constants;
pub mod ecash;

#[cfg(all(feature = "env", feature = "network"))]
pub mod env_setup;
pub mod mainnet;
#[cfg(feature = "network")]
pub mod network;

#[cfg(feature = "env")]
pub mod var_names;

pub use ecash::*;

// re-export everything to not break existing imports
pub use constants::*;
#[cfg(all(feature = "env", feature = "network"))]
pub use env_setup::*;
#[cfg(feature = "network")]
pub use network::*;
