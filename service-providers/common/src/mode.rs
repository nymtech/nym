// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Where a service provider runs, asked once.
//!
//! A provider is either **standalone** - on its own, reaching a gateway somewhere else over a
//! websocket - or **embedded** in a nym-node, which hosts it. That one fact decides everything that
//! differs between the two: which transceiver its mixnet client is built with, and whether it has
//! an LP data plane running beside that client. Both answers come out of [`ProviderMode`], so
//! nothing downstream has to keep two flags in agreement.
//!
//! An embedded provider currently runs a mixnet client *and* an LP data plane. The client is what
//! the data plane eventually replaces; a standalone one keeps its client for good, and reaches LP
//! the way any client does - underneath it, not beside it.

use std::sync::Arc;

use nym_client_core::client::mix_traffic::transceiver::GatewayTransceiver;
use nym_client_core::client::topology_control::TopologyAccessor;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_client_core::config::DebugConfig;
use nym_crypto::asymmetric::x25519;
use nym_pemstore::KeyPairPath;
use nym_task::ShutdownTracker;
use rand::rngs::OsRng;

use crate::lp::error::LpProviderError;
use crate::lp::handler::pipeline::{SpInboundPipeline, SpOutboundPipeline};
use crate::lp::handler::{ProviderLink, SpLpDataSetup};
use crate::lp::PipelineLink;

/// What starting a [`ProviderMode`] hands back: the transceiver its mixnet client is built with,
/// and the LP data plane running beside that client.
///
/// Both `Some` or both `None`, because they are the same decision.
pub type StartedMode = (
    Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    Option<ProviderLink>,
);

/// How a provider reaches the mixnet.
#[derive(Default)]
pub enum ServiceProviderMode {
    /// On its own, through a gateway it connects to over a websocket.
    ///
    /// No LP data plane: the channels one runs over have a nym-node at the far end, and a
    /// standalone provider has no such thing.
    #[default]
    Standalone,

    /// Inside a nym-node, which hands over everything only a host can give.
    Embedded(EmbeddedSetup),
}

/// What a nym-node settles once, for every provider it hosts.
///
/// The per-host half of [`EmbeddedSetup`]'s per-provider one: these describe the machine the
/// providers share rather than anything one of them does, which is why the host decides them and
/// no provider gets a say.
pub struct HostedProvidersLp {
    /// The one view their outbound pipelines route against.
    pub topology: TopologyAccessor,

    /// How many threads each provider's inbound pipeline peels on.
    pub inbound_workers: usize,
}

/// What a nym-node gives the provider it hosts.
pub struct EmbeddedSetup {
    /// Mix traffic goes into the host's forwarder rather than out of a socket.
    // TODO : remove when we force LP use
    pub transceiver: Box<dyn GatewayTransceiver + Send + Sync>,

    /// This provider's side of the link to its host.
    pub link: PipelineLink,

    /// The host's view of the network, for routing what this provider sends.
    pub topology: TopologyAccessor,

    /// How many threads peel what arrives for this provider.
    ///
    /// The host's call rather than the provider's: it is the one that knows how much of the machine
    /// there is and how many other providers are on it.
    pub inbound_workers: usize,
}

impl EmbeddedSetup {
    /// Build this provider's LP pipelines, start them, and hand back what the rest of it runs with.
    ///
    /// The pipelines are built by the provider rather than by its host because the key the inbound
    /// half peels with is the provider's own: on the LP path the provider is the final sphinx hop,
    /// so nothing else in the process holds what opens that layer.
    pub fn start(
        self,
        paths: &CommonClientPaths,
        debug_config: DebugConfig,
        shutdown: &ShutdownTracker,
        provider: &'static str,
    ) -> Result<(Box<dyn GatewayTransceiver + Send + Sync>, ProviderLink), LpProviderError> {
        let encryption_keys: x25519::KeyPair = nym_pemstore::load_keypair(&KeyPairPath::new(
            paths.keys.private_encryption_key().to_path_buf(),
            paths.keys.public_encryption_key().to_path_buf(),
        ))
        .map_err(|source| LpProviderError::UnreadableKeys { provider, source })?;

        let (plane, channels) = SpLpDataSetup::new(
            SpInboundPipeline::new(Arc::new(encryption_keys)),
            SpOutboundPipeline::new(OsRng, debug_config, self.topology),
            self.link,
            self.inbound_workers,
        );
        plane.start_tasks(shutdown, provider);

        Ok((self.transceiver, channels))
    }
}

impl ServiceProviderMode {
    /// Start what this mode implies, and hand back the two pieces a provider runs with.
    ///
    /// A provider with a transceiver into its host has an LP data plane, and one without has
    /// neither - see [`StartedMode`].
    pub fn start(
        self,
        paths: &CommonClientPaths,
        debug_config: DebugConfig,
        shutdown: &ShutdownTracker,
        provider: &'static str,
    ) -> Result<StartedMode, LpProviderError> {
        match self {
            ServiceProviderMode::Standalone => Ok((None, None)),
            ServiceProviderMode::Embedded(setup) => {
                let (transceiver, channels) =
                    setup.start(paths, debug_config, shutdown, provider)?;
                Ok((Some(transceiver), Some(channels)))
            }
        }
    }
}
