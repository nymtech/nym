// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! How a service provider embedded in a nym-node talks over the Lewes Protocol.
//!
//! A provider inside its gateway is not a client, and this is what it uses instead of one: a
//! pipeline pair that routes what it sends and peels what arrives, exchanging sphinx with the
//! gateway over channels rather than a socket. What it replaces, eventually, is the whole mixnet
//! client a provider builds today - there is no session to establish, no gateway to register with,
//! no acknowledgements to track and no cover traffic to emit, because the peer is the same process.
//!
//! ```text
//! outbound   plaintext -> chunk -> sphinx route -> frame (whole)  -> channel -> gateway
//! inbound    gateway -> channel -> peel final hop -> reassemble   -> plaintext
//! ```
//!
//! One implementation, one instance per provider: the peel needs *that* provider's keys and the
//! reassembler holds *that* provider's half-finished messages, so there is nothing here that three
//! providers could usefully share an instance of.

pub mod error;
pub mod handler;

use nym_lp_data::packet::LpFrame;

/// How much either direction buffers before it starts dropping.
///
/// The same depth the LP data plane uses on both sides of its socket, and for the same reason: a
/// consumer that has fallen this far behind will not catch up by being given more to hold.
pub const PROVIDER_CHANNEL_DEPTH: usize = 100;

/// Sphinx packets from a gateway to the provider it hosts.
///
/// Deliberately not the provider's legacy channel, which carries *plaintext*. That one is a
/// `Vec<Vec<u8>>` feeding `PacketRouter::route_received`, which splits by length into acks and
/// messages - so handing it a sphinx packet produces silent nonsense rather than an error. These
/// bytes only ever reach something that peels them.
pub type ServiceProviderInputSender = std::sync::mpsc::SyncSender<Vec<u8>>;
pub type ServiceProviderInputReceiver = std::sync::mpsc::Receiver<Vec<u8>>;

/// Frames from a provider to the gateway hosting it, for forwarding.
pub type ServiceProviderOutputSender = std::sync::mpsc::SyncSender<LpFrame>;
pub type ServiceProviderOutputReceiver = std::sync::mpsc::Receiver<LpFrame>;

/// Open the link between a hosted provider's pipelines and the node hosting them.
///
/// One struct per side, each holding that side's two ends - so a name says which half you have, and
/// a field says which way its bytes go.
///
/// Synchronous channels because every end of them is either a blocking pipeline worker or a tick
/// loop that only ever `try_`s - no runtime needed, and full means drop, as it does at the socket.
pub fn gateway_link() -> (PipelineLink, GatewayLink) {
    let (to_provider, from_gateway) = std::sync::mpsc::sync_channel(PROVIDER_CHANNEL_DEPTH);
    let (to_gateway, from_provider) = std::sync::mpsc::sync_channel(PROVIDER_CHANNEL_DEPTH);

    (
        PipelineLink {
            from_gateway,
            to_gateway,
        },
        GatewayLink {
            to_provider,
            from_provider,
        },
    )
}

/// The pipelines' side of that link: what they read, and what they write.
pub struct PipelineLink {
    /// Packets the gateway has decided are for this provider, still wrapped.
    pub from_gateway: ServiceProviderInputReceiver,

    /// Finished frames, for the gateway to forward.
    pub to_gateway: ServiceProviderOutputSender,
}

/// The gateway's side of it.
pub struct GatewayLink {
    pub to_provider: ServiceProviderInputSender,
    pub from_provider: ServiceProviderOutputReceiver,
}
