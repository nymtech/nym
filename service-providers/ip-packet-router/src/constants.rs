// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

// We routinely check if any clients needs to be disconnected at this interval
pub(crate) const DISCONNECT_TIMER_INTERVAL: Duration = Duration::from_secs(10);

// We consider a client inactive if it hasn't sent any mixnet packets in this duration
pub(crate) const CLIENT_MIXNET_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(5 * 60);

// We consider a client handler inactive if it hasn't received any packets from the tun device in
// this duration
pub(crate) const CLIENT_HANDLER_ACTIVITY_TIMEOUT: Duration = Duration::from_secs(10 * 60);
