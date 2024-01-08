// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod active_clients;
pub(crate) mod embedded_network_requester;
pub(crate) mod websocket;

pub(crate) const FREE_TESTNET_BANDWIDTH_VALUE: i64 = 64 * 1024 * 1024 * 1024; // 64GB
pub(crate) const TICKET_BANDWIDTH_VALUE: i64 = 100 * 1000 * 1000 * 1000; // 100 MB
