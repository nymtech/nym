//! Rust SDK for the Nym platform
//!
//! The main component currently is [`mixnet`].
//! [`tcp_proxy`] is probably a good place to start for anyone wanting to integrate with existing app code and read/write from a socket.

mod error;

pub mod bandwidth;
pub mod mixnet;
pub mod tcp_proxy;

pub use error::{Error, Result};
pub use nym_client_core::client::{
    mix_traffic::transceiver::*,
    topology_control::{
        GeoAwareTopologyProvider, NymApiTopologyProvider, NymApiTopologyProviderConfig,
        TopologyProvider,
    },
};
pub use nym_network_defaults::{
    ChainDetails, DenomDetails, DenomDetailsOwned, NymContracts, NymNetworkDetails,
    ValidatorDetails,
};
pub use nym_validator_client::UserAgent;
// we have to re-expose TaskClient since we're allowing custom shutdown in public API
// (which is quite a shame if you ask me...)
pub use nym_task::TaskClient;
