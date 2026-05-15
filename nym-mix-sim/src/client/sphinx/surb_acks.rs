// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! SURB-ACK reliability layer for [`SphinxClient`].
//!
//! Implements the [`Reliability`] trait by prepending a single-use reply block
//! (SURB) acknowledgement to every outgoing payload.  The SURB allows the
//! recipient to send a compact acknowledgement back to the sender through the
//! mix network without revealing the sender's address to intermediate nodes.
//!
//! [`SphinxClient`]: super::SphinxClient

use std::sync::Arc;

use crate::{
    client::{
        ClientId,
        sphinx::{SphinxInputOptions, SphinxPipelinePayload},
    },
    node::NodeId,
    packet::sphinx::{GenerateDelay, SurbAck},
    topology::directory::Directory,
};

use nym_lp_data::clients::traits::Reliability;

use rand::Rng;

/// Prepends a freshly-constructed SURB acknowledgement to every outgoing packet.
///
/// Each call to [`Reliability::reliable_encode`] builds a new [`SurbAck`] keyed
/// to a random 64-bit packet identifier, prepends it to the payload, and returns
/// the augmented packet.  If `input` is `None` (cover-traffic slot) no packet
/// is produced.
pub struct SurbAcksReliability<R>
where
    R: Rng,
{
    address: ClientId,
    directory: Arc<Directory>,
    rng: R,
}

impl<R> SurbAcksReliability<R>
where
    R: Rng,
{
    /// Create a new SURB-ACK reliability layer.
    ///
    /// `address` is used as the SURB reply destination so that ACKs are routed
    /// back to this client.  `directory` is used to sample the 3-hop SURB route.
    pub fn new(rng: R, address: ClientId, directory: Arc<Directory>) -> Self {
        Self {
            address,
            directory,
            rng,
        }
    }
}

impl<Ts, R> Reliability<Ts, SphinxInputOptions, NodeId> for SurbAcksReliability<R>
where
    R: Rng,
    Ts: GenerateDelay,
{
    const OVERHEAD_SIZE: usize = SurbAck::len();

    /// Prepend a SURB ACK to `input`, or return an empty vec for cover slots.
    ///
    /// A fresh [`SurbAck`] is constructed for each real packet so that every
    /// in-flight packet carries a unique acknowledgement path.
    fn reliable_encode(
        &mut self,
        input: Option<SphinxPipelinePayload<Ts>>,
        _timestamp: Ts,
    ) -> Vec<SphinxPipelinePayload<Ts>> {
        if let Some(packet) = input {
            let random_id = self.rng.next_u64();
            tracing::debug!("Generating SURB Ack with ID {random_id}");
            let surb_ack = SurbAck::construct::<Ts, R>(
                &mut self.rng,
                self.address,
                random_id,
                &self.directory,
            )
            .prepare_for_sending()
            .1;
            let reliable_packet = packet.data_transform(|payload| {
                surb_ack.iter().copied().chain(payload).collect::<Vec<_>>()
            });

            vec![reliable_packet]
        } else {
            Vec::new()
        }
    }
}
