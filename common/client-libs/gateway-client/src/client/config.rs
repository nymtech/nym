// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::GatewayClientError;
use nym_network_defaults::TicketTypeRepr::V1MixnetEntry;
use si_scale::helpers::bibytes2;
use std::time::Duration;

#[derive(Debug, Default, Clone, Copy)]
pub struct GatewayClientConfig {
    pub connection: Connection,
    pub bandwidth: BandwidthTickets,
}

impl GatewayClientConfig {
    pub fn new_default() -> Self {
        Default::default()
    }

    #[must_use]
    pub fn with_disabled_credentials_mode(mut self, disabled_credentials_mode: bool) -> Self {
        self.bandwidth.require_tickets = !disabled_credentials_mode;
        self
    }

    #[must_use]
    pub fn with_reconnection_on_failure(mut self, should_reconnect_on_failure: bool) -> Self {
        self.connection.should_reconnect_on_failure = should_reconnect_on_failure;
        self
    }

    #[must_use]
    pub fn with_response_timeout(mut self, response_timeout_duration: Duration) -> Self {
        self.connection.response_timeout_duration = response_timeout_duration;
        self
    }

    #[must_use]
    pub fn with_reconnection_attempts(mut self, reconnection_attempts: usize) -> Self {
        self.connection.reconnection_attempts = reconnection_attempts;
        self
    }

    #[must_use]
    pub fn with_reconnection_backoff(mut self, backoff: Duration) -> Self {
        self.connection.reconnection_backoff = backoff;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Connection {
    /// Specifies the timeout for gateway responses
    pub response_timeout_duration: Duration,

    /// Specifies whether client should try to reconnect to gateway on connection failure.
    pub should_reconnect_on_failure: bool,

    /// Specifies maximum number of attempts client will try to reconnect to gateway on failure
    /// before giving up.
    pub reconnection_attempts: usize,

    /// Delay between each subsequent reconnection attempt.
    pub reconnection_backoff: Duration,
}

impl Connection {
    // Set this to a high value for now, so that we don't risk sporadic timeouts that might cause
    // bought bandwidth tokens to not have time to be spent; Once we remove the gateway from the
    // bandwidth bridging protocol, we can come back to a smaller timeout value
    pub const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
    pub const DEFAULT_RECONNECTION_ATTEMPTS: usize = 10;
    pub const DEFAULT_RECONNECTION_BACKOFF: Duration = Duration::from_secs(5);
}

impl Default for Connection {
    fn default() -> Self {
        Connection {
            response_timeout_duration: Self::DEFAULT_RESPONSE_TIMEOUT,
            should_reconnect_on_failure: true,
            reconnection_attempts: Self::DEFAULT_RECONNECTION_ATTEMPTS,
            reconnection_backoff: Self::DEFAULT_RECONNECTION_BACKOFF,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BandwidthTickets {
    /// specifies whether this client will be sending bandwidth tickets or will attempt to use 'free' testnet bandwidth instead
    pub require_tickets: bool,

    /// specifies threshold (in bytes) under which the client should send another ticket to the gateway
    pub remaining_bandwidth_threshold: i64,

    /// specifies threshold (in bytes) under which the client will NOT send any tickets because it got accused of double spending and got its bandwidth revoked
    /// if not specified, the client will always send tickets
    pub cutoff_remaining_bandwidth_threshold: Option<i64>,
}

impl BandwidthTickets {
    // TO BE CHANGED \/
    pub const DEFAULT_REQUIRES_TICKETS: bool = false;

    // 20% of entry ticket value
    pub const DEFAULT_REMAINING_BANDWIDTH_THRESHOLD: i64 =
        (V1MixnetEntry.bandwidth_value() / 5) as i64;

    pub const DEFAULT_CUTOFF_REMAINING_BANDWIDTH_THRESHOLD: Option<i64> = None;

    pub fn ensure_above_cutoff(&self, available: i64) -> Result<(), GatewayClientError> {
        if let Some(cutoff) = self.cutoff_remaining_bandwidth_threshold {
            if available < cutoff {
                let available_bi2 = bibytes2(available as f64);
                let cutoff_bi2 = bibytes2(cutoff as f64);
                return Err(GatewayClientError::BandwidthBelowCutoffValue {
                    available_bi2,
                    cutoff_bi2,
                });
            }
        }

        Ok(())
    }
}

impl Default for BandwidthTickets {
    fn default() -> Self {
        BandwidthTickets {
            require_tickets: Self::DEFAULT_REQUIRES_TICKETS,
            remaining_bandwidth_threshold: Self::DEFAULT_REMAINING_BANDWIDTH_THRESHOLD,
            cutoff_remaining_bandwidth_threshold:
                Self::DEFAULT_CUTOFF_REMAINING_BANDWIDTH_THRESHOLD,
        }
    }
}
