// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use time::Duration as TimeDuration;

use crate::LpRegistrationConfig;

use crate::builder::config::NymNodeWithKeys;

/// Registration mode for the client
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationMode {
    /// 5-hop mixnet with IPR (IP Packet Router)
    Mixnet,
    /// 2-hop WireGuard
    Wireguard,
}

pub struct RegistrationClientConfig {
    pub(crate) entry: NymNodeWithKeys,
    pub(crate) exit: NymNodeWithKeys,
    pub(crate) mode: RegistrationMode,
    pub(crate) lp_registration_config: LpRegistrationConfig,
    /// Positive means the local clock is ahead of the remote one (same convention as
    /// `SkewManager::cached_skew` upstream), so it's *subtracted* from a local clock reading to
    /// correct it - `None` leaves the local clock reading uncorrected.
    pub(crate) spend_time_skew: Option<TimeDuration>,
}
