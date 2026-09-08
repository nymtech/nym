// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The clock the LP data plane runs on.
//!
//! The handler owns both directions - [`inbound`] and [`outbound`] - and does nothing but tick them.
//! Everything else is theirs: their channels, their worker pools, their buffers. Neither knows the
//! other exists.

use std::time::{Duration, Instant};

use nym_task::{ShutdownToken, ShutdownTracker};
use tokio::time::interval;
use tracing::info;

use crate::client::lp::data::handler::inbound::ClientInbound;
use crate::client::lp::data::handler::outbound::ClientOutbound;

pub mod error;
pub mod inbound;
pub mod messages;
pub mod outbound;
pub mod pipeline;
pub mod processing;
mod worker_pool;

const PIPELINE_TICKING_DURATION: Duration = Duration::from_millis(1);

/// Drives the LP data plane, one millisecond at a time.
///
/// Per-packet work is fanned out across worker pools spawned on the shared blocking pool tracked by
/// the surrounding [`ShutdownTracker`], so this loop only ever moves things between queues.
pub(crate) struct LpDataHandler {
    inbound: ClientInbound,
    outbound: ClientOutbound,
    shutdown: ShutdownToken,
}

impl LpDataHandler {
    pub(crate) fn new(
        inbound: ClientInbound,
        outbound: ClientOutbound,
        shutdown_tracker: &ShutdownTracker,
    ) -> Self {
        LpDataHandler {
            inbound,
            outbound,
            shutdown: shutdown_tracker.clone_shutdown_token(),
        }
    }

    pub(crate) async fn run(&mut self) {
        info!(
            inbound_workers = self.inbound.worker_count(),
            outbound_workers = self.outbound.worker_count(),
            "LP data handler: starting"
        );
        let mut ticking_interval = interval(PIPELINE_TICKING_DURATION);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    info!("LP data handler: received shutdown signal");
                    break;
                }

                timestamp = ticking_interval.tick() => {
                    // Tokio instant into std::time::Instant
                    let now: Instant = timestamp.into();

                    self.inbound.tick(now).await;
                    self.outbound.tick(now);
                }
            }
        }

        // Workers will stop because we are dropping the receiving channels
        info!("LP data handler: shutdown complete");
    }
}
