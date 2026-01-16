//! Rust SDK for the Nym platform
//!
//! The main component currently is [`mixnet`].
//! [`client_pool`] is a configurable client pool.
//! [`tcp_proxy`] is a soon to be deprecated wrapper around the mixnet client which exposes a localhost port.
//! [`stream_wrapper`] is the v2 of the tcp_proxy, exposing a socket-like abstraction around the mixnet client.

mod error;

pub mod bandwidth;
pub mod client_pool;
pub mod ip_packet_client;
pub mod mixnet;
pub mod stream_wrapper;
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
