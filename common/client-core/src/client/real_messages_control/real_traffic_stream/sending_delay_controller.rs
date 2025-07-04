// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::helpers::{get_time_now, Instant};
use std::time::Duration;

// The minimum time between increasing the average delay between packets. If we hit the ceiling in
// the available buffer space we want to take somewhat swift action, but we still need to give a
// short time to give the channel a chance reduce pressure.
const INCREASE_DELAY_MIN_CHANGE_INTERVAL_SECS: u64 = 1;
// The minimum time between decreasing the average delay between packets. We don't want to change
// to quickly to keep things somewhat stable. Also there are buffers downstreams meaning we need to
// wait a little to see the effect before we decrease further.
const DECREASE_DELAY_MIN_CHANGE_INTERVAL_SECS: u64 = 2;
// The queue length that is required for us to register that backpressure occured. If there are
// more than this many packets waiting to be sent, we consider the channel to be under
// backpressure.
const BACKPRESSURE_THRESHOLD: usize = 10;
// If we enough time passes without any sign of backpressure in the channel, we can consider
// lowering the average delay.
const ACCEPTABLE_TIME_WITHOUT_BACKPRESSURE_SECS: u64 = 2;
// The maximum multiplier we apply to the base average Poisson delay.
const MAX_DELAY_MULTIPLIER: u32 = 6;
// The minium multiplier we apply to the base average Poisson delay.
const MIN_DELAY_MULTIPLIER: u32 = 1;
// If the multipler increases we log it, but we don't want to log about it too often.
const INTERVAL_BETWEEN_WARNING_ABOUT_ELEVATED_MULTIPLIER_SECS: u64 = 60;

pub(crate) struct SendingDelayController {
    /// Multiply the average sending delay.
    /// This is normally set to unity, but if we detect backpressure we increase this
    /// multiplier. We use discrete steps.
    current_multiplier: u32,

    /// Maximum delay multiplier
    upper_bound: u32,

    /// Minimum delay multiplier
    lower_bound: u32,

    /// We counter the number of times the multiplier has been elevated. If it is elevated for long
    /// enough we need to log about it.
    multiplier_elevated_counter: u32,

    /// We can't log about the elevated multiplier too often, so we keep track of the last time we
    /// did,
    time_when_logged_about_elevated_multiplier: Instant,

    /// To make sure we don't change the multiplier to fast, we limit a change to some duration
    time_when_changed: Instant,

    /// If we have a long enough time without any backpressure detected we try reducing the sending
    /// delay multiplier
    time_when_backpressure_detected: Instant,
}

impl Default for SendingDelayController {
    fn default() -> Self {
        SendingDelayController::new(MIN_DELAY_MULTIPLIER, MAX_DELAY_MULTIPLIER)
    }
}

impl SendingDelayController {
    pub(crate) fn new(lower_bound: u32, upper_bound: u32) -> Self {
        assert!(lower_bound <= upper_bound);
        let now = get_time_now();
        SendingDelayController {
            current_multiplier: MIN_DELAY_MULTIPLIER,
            upper_bound,
            lower_bound,
            multiplier_elevated_counter: 0,
            time_when_logged_about_elevated_multiplier: now
                .checked_sub(Duration::from_secs(
                    INTERVAL_BETWEEN_WARNING_ABOUT_ELEVATED_MULTIPLIER_SECS,
                ))
                .unwrap_or(now),
            time_when_changed: now,
            time_when_backpressure_detected: now,
        }
    }

    pub(crate) fn current_multiplier(&self) -> u32 {
        self.current_multiplier
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn min_multiplier(&self) -> u32 {
        self.lower_bound
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn max_multiplier(&self) -> u32 {
        self.upper_bound
    }

    pub(crate) fn increase_delay_multiplier(&mut self) {
        if self.current_multiplier < self.upper_bound {
            self.current_multiplier =
                (self.current_multiplier + 1).clamp(self.lower_bound, self.upper_bound);
            self.time_when_changed = get_time_now();
            tracing::debug!(
                "Increasing sending delay multiplier to: {}",
                self.current_multiplier
            );
        } else {
            tracing::warn!("Trying to increase delay multipler higher than allowed");
        }
    }

    pub(crate) fn decrease_delay_multiplier(&mut self) {
        if self.current_multiplier > self.lower_bound {
            self.current_multiplier =
                (self.current_multiplier - 1).clamp(self.lower_bound, self.upper_bound);
            self.time_when_changed = get_time_now();
            tracing::debug!(
                "Decreasing sending delay multiplier to: {}",
                self.current_multiplier
            );
        }
    }

    pub(crate) fn not_increased_delay_recently(&self) -> bool {
        get_time_now()
            > self.time_when_changed + Duration::from_secs(INCREASE_DELAY_MIN_CHANGE_INTERVAL_SECS)
    }

    pub(crate) fn not_decreased_delay_recently(&self) -> bool {
        get_time_now()
            > self.time_when_changed + Duration::from_secs(DECREASE_DELAY_MIN_CHANGE_INTERVAL_SECS)
    }

    pub(crate) fn is_backpressure_currently_detected(&self, queue_length: usize) -> bool {
        queue_length > BACKPRESSURE_THRESHOLD
    }

    pub(crate) fn record_backpressure_detected(&mut self) {
        self.time_when_backpressure_detected = get_time_now();
    }

    pub(crate) fn was_backpressure_detected_recently(&self) -> bool {
        get_time_now()
            < self.time_when_backpressure_detected
                + Duration::from_secs(ACCEPTABLE_TIME_WITHOUT_BACKPRESSURE_SECS)
    }

    pub(crate) fn record_delay_multiplier(&mut self) {
        // Count the number of times the multiplier has been elevated.
        let multiplier_elevated = self.current_multiplier - self.lower_bound;
        if multiplier_elevated == 0 {
            self.multiplier_elevated_counter = 0;
        } else {
            self.multiplier_elevated_counter += 1;
        }

        // If needed, log about the elevated multiplier.
        let now = get_time_now();
        if self.multiplier_elevated_counter > 20
            && now
                > self.time_when_logged_about_elevated_multiplier
                    + Duration::from_secs(INTERVAL_BETWEEN_WARNING_ABOUT_ELEVATED_MULTIPLIER_SECS)
        {
            let status_str = format!(
                "Poisson delay currently scaled by: {}",
                self.current_multiplier()
            );
            if self.current_multiplier() > 0 {
                tracing::debug!("{}", status_str);
            } else if self.current_multiplier() > 1 {
                tracing::info!("{}", status_str);
            } else if self.current_multiplier() > 2 {
                tracing::warn!("{}", status_str);
            }
            self.time_when_logged_about_elevated_multiplier = now;
        }
    }
}
