// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_defaults::TicketTypeRepr::V1MixnetEntry;

pub(crate) mod active_clients;
mod bandwidth;
pub(crate) mod embedded_clients;
pub(crate) mod websocket;

// as defined in common/client-libs/gateway-client/src/client/config.rs::BandwidthTickets::DEFAULT_REMAINING_BANDWIDTH_THRESHOLD
pub const DEFAULT_MIXNET_CLIENT_BANDWIDTH_THRESHOLD: i64 =
    (V1MixnetEntry.bandwidth_value() / 5) as i64;
