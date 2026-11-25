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
//!
//! The LP path is two channels: one from the node to each provider, carrying sphinx still to be
//! peeled, and one from every provider back to the node, carrying frames to forward.

use nym_client_core::client::mix_traffic::transceiver::GatewayTransceiver;
use nym_client_core::client::topology_control::LpTopologyAccessor;
use nym_lp_data::packet::LpFrame;

/// How much either direction buffers before it starts dropping.
///
/// The same depth the LP data plane uses on both sides of its socket, and for the same reason: a
/// consumer that has fallen this far behind will not catch up by being given more to hold.
pub const PROVIDER_CHANNEL_DEPTH: usize = 100;

/// Sphinx packets from the node to one provider it hosts, still wrapped.
///
/// Deliberately not the provider's legacy channel, which carries *plaintext*. That one is a
/// `Vec<Vec<u8>>` feeding `PacketRouter::route_received`, which splits by length into acks and
/// messages - so handing it a sphinx packet produces silent nonsense rather than an error. These
/// bytes only ever reach something that peels them.
pub type ProviderInputSender = std::sync::mpsc::SyncSender<Vec<u8>>;
pub type ProviderInputReceiver = std::sync::mpsc::Receiver<Vec<u8>>;

/// Frames from the providers to the node hosting them, for forwarding.
///
/// One channel for every provider on the node, each holding a clone of the sender. A tokio
/// channel, unlike the input, because the node's end is read by the LP data handler's async event
/// loop; the providers' ends are written by blocking tick loops, which only ever `try_send` - and
/// that needs no runtime.
pub type ProviderOutputSender = tokio::sync::mpsc::Sender<LpFrame>;
pub type ProviderOutputReceiver = tokio::sync::mpsc::Receiver<LpFrame>;

/// The channel the node sends one provider its sphinx packets down. One per provider.
///
/// Synchronous, because only blocking threads touch it: the node's workers write it, and the
/// provider's dispatcher reads it. Bounded, and the node only ever `try_send`s: full means drop, as
/// it does at the socket.
pub fn provider_input_channel() -> (ProviderInputSender, ProviderInputReceiver) {
    std::sync::mpsc::sync_channel(PROVIDER_CHANNEL_DEPTH)
}

/// The channel every provider a node hosts sends its frames down, sized for `providers`.
///
/// Shared, so a provider that floods it crowds out the others; sized so each still has the depth
/// a channel of its own would give it.
pub fn provider_output_channel(providers: usize) -> (ProviderOutputSender, ProviderOutputReceiver) {
    tokio::sync::mpsc::channel(PROVIDER_CHANNEL_DEPTH * providers.max(1))
}

/// What a nym-node gives the provider it hosts.
pub struct EmbeddedSetup {
    /// The LP path
    pub lp: EmbeddedLp,

    /// The legacy path,
    pub legacy: Box<dyn GatewayTransceiver + Send + Sync>,
}

/// The LP path of an [`EmbeddedSetup`].
pub struct EmbeddedLp {
    /// Sphinx packets the node has decided are for this provider, still wrapped.
    pub from_node: ProviderInputReceiver,

    /// Finished frames, for the node to forward. Shared with every other provider it hosts.
    pub to_node: ProviderOutputSender,

    /// The node's view of the network, for routing what this provider sends.
    pub topology: LpTopologyAccessor,

    /// How many threads peel what arrives for this provider.
    ///
    /// The node's call rather than the provider's: it is the one that knows how much of the machine
    /// there is and how many other providers are on it.
    pub inbound_workers: usize,
}
