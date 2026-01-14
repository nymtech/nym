// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::builder::config::NymNodeWithKeys;

/// Registration mode for the client
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationMode {
    /// 5-hop mixnet with IPR (IP Packet Router)
    Mixnet,
    /// 2-hop WireGuard with authenticator
    Wireguard,
    /// 2-hop WireGuard with LP (Lewes Protocol)
    Lp,
}

impl RegistrationMode {
    /// Legacy method for backward compatibility
    #[deprecated(note = "use explicit enum variant instead")]
    pub fn legacy_two_hop(use_two_hop: bool) -> RegistrationMode {
        if use_two_hop {
            Self::Wireguard
        } else {
            Self::Mixnet
        }
    }
}

pub struct RegistrationClientConfig {
    pub(crate) entry: NymNodeWithKeys,
    pub(crate) exit: NymNodeWithKeys,
    pub(crate) mode: RegistrationMode,
}
