// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Turning stored route sessions into the test messages five-hop liveness is scored from.

use crate::storage::models::{FiveHopMixnetSession, FiveHopMixnetSessionOutcome};
use nym_network_monitor_route_records::batch::Batch;
use nym_network_monitor_route_records::test_message::SessionTestMessages;
use tracing::warn;

/// A stored route session's test messages, with whether the discard rule kept them when submitted.
pub(crate) enum FiveHopMixnetSessionTestMessages {
    /// Stored with an error, or its batch no longer reads.
    Error(String),

    Kept(SessionTestMessages),

    Discarded(SessionTestMessages),
}

impl FiveHopMixnetSessionTestMessages {
    /// Decodes a stored session's batch into its test messages.
    pub(crate) fn read(session: &FiveHopMixnetSession) -> Self {
        match &session.outcome {
            FiveHopMixnetSessionOutcome::Error(error) => {
                FiveHopMixnetSessionTestMessages::Error(error.clone())
            }
            FiveHopMixnetSessionOutcome::Batch { batch, kept, .. } => match Batch::decode(batch) {
                // read whole when it was submitted, so only damage or a format change gets here
                Err(err) => {
                    warn!(
                        "the stored batch of route run {} through gateway {} no longer reads: {err:#}",
                        session.run_id, session.gateway_node_id
                    );
                    FiveHopMixnetSessionTestMessages::Error(format!(
                        "the stored batch no longer reads: {err:#}"
                    ))
                }
                Ok(batch) if *kept => {
                    FiveHopMixnetSessionTestMessages::Kept(SessionTestMessages::from_batch(&batch))
                }
                Ok(batch) => FiveHopMixnetSessionTestMessages::Discarded(
                    SessionTestMessages::from_batch(&batch),
                ),
            },
        }
    }
}
