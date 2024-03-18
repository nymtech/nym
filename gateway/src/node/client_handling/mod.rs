// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::bandwidth::Bandwidth;

pub(crate) mod active_clients;
mod bandwidth;
pub(crate) mod embedded_clients;
pub(crate) mod websocket;

pub(crate) const FREE_TESTNET_BANDWIDTH_VALUE: Bandwidth = Bandwidth::new(64 * 1024 * 1024 * 1024); // 64GB
