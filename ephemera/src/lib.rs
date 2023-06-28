//! # Ephemera Node

//! An Ephemera node does reliable broadcast of inbound messages to all other Ephemera nodes in the cluster.
//!
//! Each node has a unique ID, and each message is signed by the node that first received it.
//! Messages are then re-broadcast and re-signed by all other nodes in the cluster.
//!
//! At the end of the process, each message is signed by every node in the cluster, and each node has also
//! signed all messages that were broadcast by other nodes. This means that nodes are unable to repudiate messages
//! once they are seen and signed, so there is a strong guarantee of message integrity within the cluster.
//!
//! # Why would I want this?
//!
//! Let's say you have blockchain system that needs to ship large amounts of information around, but the information
//! is relatively short-lived. You could use a blockchain to store the information, but that would be expensive,
//! slow, and wasteful. Instead, you could use Ephemera to broadcast the information to all nodes in the cluster,
//! and then store only a cryptographic commitment in the blockchain's data store.
//!
//! Ephemera nodes then keep messages around for inspection in a data availability layer (accessible over HTTP)
//! so that interested parties can verify correctness. Ephemeral information can then be automatically discarded
//! once it's no longer useful.
//!
//! This gives very similar guarantees to a blockchain, but without incurring the permanent storage costs.
//!
//! Note that it *requires* a blockchain to be present.

//'Denying' everything and allowing exceptions seems better than other way around.
#![deny(clippy::pedantic)]

// PUBLIC MODULES

pub use crate::core::builder::{
    EphemeraStarterInit, EphemeraStarterWithApplication, EphemeraStarterWithProvider,
};
pub use crate::core::ephemera::Ephemera;
pub use crate::core::shutdown::Handle as ShutdownHandle;

/// Ephemera API. Public interface and types.
pub mod ephemera_api {
    pub use crate::api::{
        application::{
            Application, CheckBlockResult, Dummy, Error as ApplicationError, RemoveMessages,
            Result as ApplicationResult,
        },
        http::client::{Client, Error as HttpClientError, Result as HttpClientResult},
        types::{
            ApiBlock, ApiBlockBroadcastInfo, ApiBroadcastInfo, ApiCertificate, ApiDhtQueryRequest,
            ApiDhtQueryResponse, ApiDhtStoreRequest, ApiEphemeraConfig, ApiEphemeraMessage,
            ApiError, ApiHealth, ApiVerifyMessageInBlock, RawApiEphemeraMessage,
        },
        CommandExecutor,
    };
}

/// Peer identification
#[allow(clippy::module_name_repetitions)]
pub mod peer {
    pub use super::network::{PeerId, PeerIdError, ToPeerId};
}

/// Ephemera membership. How to find other nodes in the cluster.
pub mod membership {
    pub use super::network::members::{
        ConfigMembersProvider, DummyMembersProvider, PeerInfo, PeerSetting, ProviderError, Result,
    };
}

/// Ephemera keypair and public key
pub mod crypto {
    pub use super::utilities::crypto::{
        EphemeraKeypair, EphemeraPublicKey, KeyPairError, Keypair, PublicKey,
    };
}

/// Ephemera codec to encode and decode messages
pub mod codec {
    pub use super::utilities::codec::{Decode, Encode};
}

/// Ephemera node configuration
pub mod configuration {
    pub use super::config::Configuration;
}

/// Ephemera CLI. Helpers for creating configuration, running node, etc.
pub mod cli;

/// Utilities to set up logging.
pub mod logging;

// PRIVATE MODULES

/// External interface for Ephemera
mod api;

/// Block creation code
mod block;

/// Ephemera reliable broadcast
mod broadcast;

/// Ephemera configuration
mod config;

/// Ephemera core. Ephemera builder and instance.
mod core;

/// Ephemera networking with peers
mod network;

/// Ephemera storage. Block storage and certificate storage.
mod storage;

/// Ephemera utilities. Crypto, codec, etc.
mod utilities;

/// Ephemera websocket. Websocket server where external clients can subscribe.
mod websocket;
