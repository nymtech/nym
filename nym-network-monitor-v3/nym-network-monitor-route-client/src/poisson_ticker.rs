// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_sphinx::utils::sample_poisson_duration;
use std::time::Duration;
use tokio::time::{Instant, sleep_until};

/// Ticks at the instants of a Poisson process, paced as the client's traffic streams are.
pub(crate) struct PoissonTicker {
    average_delay: Duration,
    next_tick: Instant,
}

impl PoissonTicker {
    /// A ticker whose first tick is immediate, as the client's cover stream sends at once.
    pub(crate) fn new(average_delay: Duration) -> PoissonTicker {
        PoissonTicker {
            average_delay,
            next_tick: Instant::now(),
        }
    }

    /// Waits for the next tick; cancel-safe, as the deadline moves only once it is reached.
    pub(crate) async fn tick(&mut self) {
        sleep_until(self.next_tick).await;
        // from the previous deadline rather than from now, as the client's streams do
        self.next_tick += sample_poisson_duration(&mut rand::rng(), self.average_delay);
    }
}
