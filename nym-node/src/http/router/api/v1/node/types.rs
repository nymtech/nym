// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) use nym_bin_common::build_information::BinaryBuildInformationOwned;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Default, Debug, Copy, ToSchema, Serialize)]
pub struct NodeRoles {
    pub mixnode_enabled: bool,
    pub gateway_enabled: bool,
    pub network_requester_enabled: bool,
}
