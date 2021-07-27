// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Internally used struct to catch results from the database to calculate uptimes for given mixnode/gateway
pub(crate) struct NodeStatus {
    pub(crate) timestamp: i64,
    pub(crate) up: bool,
}

// Internally used struct to catch results from the database to find active mixnodes/gateways
pub(crate) struct ActiveNode {
    pub(crate) id: i64,
    pub(crate) identity: String,
    pub(crate) owner: String,
}
