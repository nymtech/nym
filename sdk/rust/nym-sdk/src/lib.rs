// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Rust SDK for the Nym platform
//!
//! The main component currently is [`mixnet`].
//! [`tcp_proxy`] is probably a good place to start for anyone wanting to integrate with existing app code and read/write from a socket.
//! [`client_pool`] is a configurable client pool.
//! [`ipr_wrapper`] tunnels IP packets through the mixnet to an IPR exit gateway.

mod error;

pub mod bandwidth;
pub mod client_pool;
pub mod ip_packet_client;
pub mod ipr_wrapper;
pub mod mixnet;
pub mod tcp_proxy;

pub use error::{Error, Result};
#[allow(deprecated)]
pub use nym_client_core::{
    client::{
        mix_traffic::transceiver::*,
        topology_control::{
            NymApiTopologyProvider, NymApiTopologyProviderConfig, TopologyProvider,
        },
    },
    config::{DebugConfig, RememberMe},
};
pub use nym_network_defaults::{
    ChainDetails, DenomDetails, DenomDetailsOwned, NymContracts, NymNetworkDetails,
    ValidatorDetails,
};
pub use nym_task::{ShutdownToken, ShutdownTracker};
pub use nym_validator_client::UserAgent;
