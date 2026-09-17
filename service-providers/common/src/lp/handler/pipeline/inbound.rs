// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! A sphinx packet arriving for a provider, taken down to the bytes it was carrying.
//!
//! A provider on the LP path is the last sphinx hop, exactly as a client is, so peeling is the same
//! operation - and is the same code: [`process`] is client-core's, called rather than copied.
//!
//! The two wire stages are no-ops here for the mirror of the reason they are in
//! [`outbound`](super::outbound): the gateway hands the packet over a channel, so there is no
//! transport to strip and no frames to put back together. Message-level reassembly is a different
//! thing and very much stays - a request larger than one sphinx packet arrives as several
//! [`Fragment`](nym_sphinx::chunking::fragment::Fragment)s, and `process` holds them until the last
//! one lands.

use std::sync::{Arc, Mutex};

use nym_client_core::client::lp::data::handler::processing;
use nym_crypto::asymmetric::x25519;
use nym_lp_data::clients::traits::ClientUnwrappingPipeline;
use nym_lp_data::common::helpers::NoOpWireUnwrapper;
use nym_lp_data::TimedPayload;
use nym_sphinx::chunking::reconstruction::MessageReconstructor;
use tracing::warn;

/// Unwraps what arrives for a provider on the LP path.
///
/// `Clone` so a pool of workers can share one: what the clones share is what matters, and the
/// reassembler is behind an `Arc<Mutex<_>>` precisely so that whichever worker happens to receive
/// the last fragment of a message is the one that completes it.
#[derive(Clone)]
pub struct SpInboundPipeline {
    /// The provider's own keys - the packet's last layer is addressed to them.
    encryption_keys: Arc<x25519::KeyPair>,

    /// Half-assembled messages, shared across workers.
    reconstructor: Arc<Mutex<MessageReconstructor>>,
}

impl SpInboundPipeline {
    pub fn new(encryption_keys: Arc<x25519::KeyPair>) -> Self {
        SpInboundPipeline {
            encryption_keys,
            reconstructor: Arc::new(Mutex::new(MessageReconstructor::new())),
        }
    }
}

/// No transport to strip and no frames to reassemble: the gateway handed the packet over a channel.
impl NoOpWireUnwrapper for SpInboundPipeline {}

impl ClientUnwrappingPipeline<Vec<u8>, ()> for SpInboundPipeline {
    /// The plaintext of a completed message, or `None` while one is still missing fragments.
    ///
    // TODO : add cover traffic and reliability handling
    /// What comes out here goes straight to the provider, with none of the
    /// client's delivery machinery in between.
    fn process_unwrapped(&mut self, payload: TimedPayload, _: ()) -> Option<Vec<u8>> {
        processing::sphinx::process(&self.encryption_keys, &self.reconstructor, payload)
            .inspect_err(|err| warn!("LP provider inbound: dropping a packet: {err}"))
            .ok()
            .flatten()
    }
}
