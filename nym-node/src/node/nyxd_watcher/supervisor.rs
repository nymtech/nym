// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rand::RngExt;
use std::time::Duration;

/// The wait before the first attempt to start a new watcher.
const FIRST_RESTART_WAIT: Duration = Duration::from_secs(30);

/// The longest wait between attempts, and how long a watcher has to run for the wait to start over.
const MAX_RESTART_WAIT: Duration = Duration::from_secs(15 * 60);

/// The wait before each attempt to start a new watcher.
struct RestartBackoff {
    next: Duration,
}

impl RestartBackoff {
    fn new() -> Self {
        RestartBackoff {
            next: FIRST_RESTART_WAIT,
        }
    }

    /// A watcher that ran for at least `MAX_RESTART_WAIT` brings the wait back to `FIRST_RESTART_WAIT`.
    fn watcher_ran_for(&mut self, uptime: Duration) {
        if uptime >= MAX_RESTART_WAIT {
            self.next = FIRST_RESTART_WAIT;
        }
    }

    /// Each call doubles the wait returned by the next one, up to `MAX_RESTART_WAIT`.
    fn next_wait(&mut self) -> Duration {
        let wait = self.next;
        self.next = (wait * 2).min(MAX_RESTART_WAIT);
        wait
    }

    /// A random wait between half of `wait` and `wait`, so nodes that fail together do not retry together.
    fn with_jitter(wait: Duration) -> Duration {
        rand::rng().random_range(wait / 2..=wait)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wait_doubles_up_to_the_cap_and_starts_over_after_a_healthy_run() {
        let mut backoff = RestartBackoff::new();
        assert_eq!(backoff.next_wait(), FIRST_RESTART_WAIT);
        assert_eq!(backoff.next_wait(), FIRST_RESTART_WAIT * 2);

        let later_waits: Vec<_> = (0..32).map(|_| backoff.next_wait()).collect();
        assert_eq!(later_waits.last(), Some(&MAX_RESTART_WAIT));
        assert!(later_waits.into_iter().all(|wait| wait <= MAX_RESTART_WAIT));

        backoff.watcher_ran_for(MAX_RESTART_WAIT - Duration::from_secs(1));
        assert_eq!(backoff.next_wait(), MAX_RESTART_WAIT);

        backoff.watcher_ran_for(MAX_RESTART_WAIT);
        assert_eq!(backoff.next_wait(), FIRST_RESTART_WAIT);
    }
}
