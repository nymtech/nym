//! Rust SDK for the Nym platform
//!
//! The main component currently is [`mixnet`].
//! [`client_pool`] is a configurable client pool.
//!
//! TODO OTHER MODULES

mod error;

pub mod bandwidth;
pub mod client_pool;
pub mod ip_packet_client;
pub mod mixnet;
pub mod stream_wrapper;
#[deprecated(
    note = "Functionality from this module is mostly superceded by the stream_wrapper::MixSocket and stream_wrapper::MixStreamIPR exports. This module is no longer maintained and will be removed in a future release."
)]
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
