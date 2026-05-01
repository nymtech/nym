// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Poisson cover traffic generator.
//!
//! Implements the [`Obfuscation`] trait for [`SphinxClient`] using two
//! independent Poisson processes:
//!
//! * **Main loop** — schedules one slot per inter-arrival time drawn from an
//!   exponential distribution.  Real messages are injected into these slots; if
//!   no real message is ready when a slot fires, a cover-traffic payload is sent
//!   instead.
//! * **Secondary loop** — independently fires cover-traffic packets at a lower
//!   rate, providing additional traffic volume that is independent of the main
//!   loop's cadence.
//!
//! Together the two loops ensure that an observer cannot determine from traffic
//! patterns alone whether the client is actively sending real messages.
//!
//! [`SphinxClient`]: super::SphinxClient

use std::sync::Arc;

use nym_lp_data::clients::traits::Obfuscation;
use nym_sphinx::cover::LOOP_COVER_MESSAGE_PAYLOAD;
use rand::Rng;

use crate::{
    client::{
        ClientId,
        sphinx::{SphinxInputOptions, SphinxPipelinePayload},
    },
    node::NodeId,
    packet::sphinx::GenerateDelay,
    topology::directory::Directory,
};

/// Two-loop Poisson cover traffic generator.
///
/// Maintains two independent next-fire timestamps — one for the main sending
/// loop and one for the secondary cover loop — and advances them by independent
/// exponential delays on each firing.
pub struct PoissonCoverTraffic<Ts, R>
where
    Ts: Clone + GenerateDelay + PartialOrd,
    R: Rng,
{
    address: ClientId,
    directory: Arc<Directory>,
    /// Timestamp at which the main loop next fires (real or cover packet).
    main_loop_next_timestamp: Ts,
    /// Timestamp at which the secondary cover loop next fires.
    secondary_loop_next_timestamp: Ts,
    /// Random number generator used for exponential delay sampling.
    rng: R,
}

impl<Ts, R> PoissonCoverTraffic<Ts, R>
where
    Ts: Clone + GenerateDelay + PartialOrd,
    R: Rng,
{
    /// Construct a new cover traffic generator.
    ///
    /// Both loops are initialised to fire immediately at `current_timestamp` so
    /// that cover traffic begins on the very first tick.
    pub fn new(
        address: ClientId,
        directory: Arc<Directory>,
        current_timestamp: Ts,
        rng: R,
    ) -> Self {
        Self {
            address,
            directory,
            main_loop_next_timestamp: current_timestamp.clone(),
            secondary_loop_next_timestamp: current_timestamp,
            rng,
        }
    }

    /// Build [`SphinxInputOptions`] for a self-addressed cover-traffic packet.
    ///
    /// The destination is set to this client's own address and the first hop is
    /// chosen at random from the directory, matching the real-message behaviour.
    pub fn cover_traffic_options(&mut self) -> SphinxInputOptions {
        SphinxInputOptions {
            dst: self.address,
            next_hop: self.directory.random_next_hop(&mut self.rng),
        }
    }
}

impl<Ts, R> Obfuscation<Ts, SphinxInputOptions, NodeId> for PoissonCoverTraffic<Ts, R>
where
    Ts: Clone + GenerateDelay + PartialOrd,
    R: Rng,
{
    /// Produce the set of payloads to send at `timestamp`.
    ///
    /// Called once per tick with an optional real message (`input`).  May
    /// return zero, one, or two payloads depending on which loops fire and
    /// whether a real message is available.
    fn obfuscate(
        &mut self,
        input: Option<SphinxPipelinePayload<Ts>>,
        timestamp: Ts,
    ) -> Vec<SphinxPipelinePayload<Ts>> {
        let mut output = Vec::new();

        // Secondary cover traffic loop
        // We should not schedule those in advance, because backpressure can't tell if it has real or cover traffic.
        if timestamp >= self.secondary_loop_next_timestamp {
            let cover_options = self.cover_traffic_options();
            output.push(SphinxPipelinePayload::new(
                timestamp.clone(),
                LOOP_COVER_MESSAGE_PAYLOAD.to_vec(),
                cover_options,
                cover_options.next_hop,
            ));
            self.secondary_loop_next_timestamp = self.secondary_loop_next_timestamp.clone()
                + Ts::generate_cover_traffic_delay(&mut self.rng);
        }

        // Main cover traffic loop

        match input {
            // If we have a message, schedule it for the next timestamp, prepare the following one
            Some(real_message) => {
                output.push(real_message.ts_transform(|_| self.main_loop_next_timestamp.clone()));
                self.main_loop_next_timestamp = self.main_loop_next_timestamp.clone()
                    + Ts::generate_sending_delay(&mut self.rng);
            }
            // No message, but we need to send something => Send cover traffic right away, prepare next timestamp
            None if timestamp >= self.main_loop_next_timestamp => {
                let cover_options = self.cover_traffic_options();
                output.push(SphinxPipelinePayload::new(
                    timestamp,
                    LOOP_COVER_MESSAGE_PAYLOAD.to_vec(),
                    cover_options,
                    cover_options.next_hop,
                ));
                self.main_loop_next_timestamp = self.main_loop_next_timestamp.clone()
                    + Ts::generate_sending_delay(&mut self.rng);
            }
            // No message, not the time to send anything, nothing to do
            None => {}
        }

        output
    }
}
