// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

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
    /// Callback invoked with the raw fd of sockets opened for registration,
    /// before connecting. Used to set `SO_MARK` on Linux so the connection is
    /// allowed through the VPN firewall during the connecting state.
    #[cfg(unix)]
    pub(crate) connection_fd_callback: std::sync::Arc<dyn Fn(std::os::fd::RawFd) + Send + Sync>,
}
