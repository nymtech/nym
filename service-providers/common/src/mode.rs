// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What a nym-node hands the service providers it hosts.
//!
//! A provider is either **standalone** - on its own, reaching a gateway somewhere else through its
//! mixnet client, legacy or LP - or **embedded** in a nym-node, which hosts it. Each provider has
//! one entry point per case, so whoever starts it says which: a standalone one needs nothing from
//! here, an embedded one is started with an [`EmbeddedSetup`].
//!
//! An embedded provider always reaches its host over LP, a pipeline pair, and for now also over the
//! legacy path, a mixnet client with a transceiver into the host. A standalone one keeps its client
//! for good, and reaches LP the way any client does - underneath it, not beside it.

use nym_client_core::client::mix_traffic::transceiver::GatewayTransceiver;
use nym_client_core::client::topology_control::LpTopologyAccessor;

use crate::lp::PipelineLink;

/// What a nym-node gives the provider it hosts.
pub struct EmbeddedSetup {
    /// The LP path
    pub lp: EmbeddedLp,

    /// The legacy path,
    pub legacy: Box<dyn GatewayTransceiver + Send + Sync>,
}

/// The LP path of an [`EmbeddedSetup`].
pub struct EmbeddedLp {
    /// This provider's side of the link to its host.
    pub link: PipelineLink,

    /// The host's view of the network, for routing what this provider sends.
    pub topology: LpTopologyAccessor,

    /// How many threads peel what arrives for this provider.
    ///
    /// The host's call rather than the provider's: it is the one that knows how much of the machine
    /// there is and how many other providers are on it.
    pub inbound_workers: usize,
}
