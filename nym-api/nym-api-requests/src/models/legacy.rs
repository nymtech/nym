// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::{GatewayBond, NodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[deprecated]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayBondAnnotated {}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayDescription {
    // for now only expose what we need. this struct will evolve in the future (or be incorporated into nym-node properly)
}

#[deprecated]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MixNodeBondAnnotated {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyGatewayBondWithId {
    #[serde(flatten)]
    pub bond: GatewayBond,
    pub node_id: NodeId,
}
