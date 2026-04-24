// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::{TimedPayload, clients::traits::Obfuscation};
use nym_sphinx::cover::LOOP_COVER_MESSAGE_PAYLOAD;
use rand::Rng;

use crate::{client::sphinx::SphinxInputOptions, node::NodeId, packet::sphinx::GenerateDelay};

pub struct PoissonCoverTraffic<Ts, R>
where
    Ts: Clone + GenerateDelay + PartialOrd,
    R: Rng,
{
    main_loop_next_timestamp: Ts,
    secondary_loop_next_timestamp: Ts,
    rng: R,
}

impl<Ts, R> PoissonCoverTraffic<Ts, R>
where
    Ts: Clone + GenerateDelay + PartialOrd,
    R: Rng,
{
    pub fn new(current_timestamp: Ts, rng: R) -> Self {
        Self {
            main_loop_next_timestamp: current_timestamp.clone(),
            secondary_loop_next_timestamp: current_timestamp,
            rng,
        }
    }
}

impl<Ts, R> Obfuscation<Ts, SphinxInputOptions, NodeId> for PoissonCoverTraffic<Ts, R>
where
    Ts: Clone + GenerateDelay + PartialOrd,
    R: Rng,
{
    fn obfuscate(
        &mut self,
        input: Option<TimedPayload<Ts>>,
        _: SphinxInputOptions,
        timestamp: Ts,
    ) -> Vec<TimedPayload<Ts>> {
        let mut output = Vec::new();

        // Secondary cover traffic loop
        // We should not schedule those in advance, because backpressure can't tell if it has real or cover traffic.
        if timestamp >= self.secondary_loop_next_timestamp {
            output.push(TimedPayload::new(
                timestamp.clone(),
                LOOP_COVER_MESSAGE_PAYLOAD.to_vec(),
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
                output.push(TimedPayload::new(
                    timestamp,
                    LOOP_COVER_MESSAGE_PAYLOAD.to_vec(),
                ));
                self.main_loop_next_timestamp = self.main_loop_next_timestamp.clone()
                    + Ts::generate_sending_delay(&mut self.rng);
            }
            // No message, not the time to send anything, nothing to do
            None => {}
        }

        output
    }

    fn buffer_size(&self) -> usize {
        // SW Do I need that after all?
        0
    }
}
