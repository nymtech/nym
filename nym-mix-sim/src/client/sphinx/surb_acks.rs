// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
