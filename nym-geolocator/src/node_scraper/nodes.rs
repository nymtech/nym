// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_node_requests::api::v1::node::models::HostInformation;
use nym_validator_client::client::NodeId;

#[derive(Clone)]
pub(crate) struct MinimalNodeDetails {
    pub(crate) node_id: NodeId,
    pub(crate) host_information: HostInformation,
}
