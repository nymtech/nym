// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_mixnet_contract_common::NodeId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Serialize, Deserialize, Debug, JsonSchema, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PaginationRequest {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub(crate) struct NodeIdParam {
    #[schema(value_type = u32)]
    pub(crate) node_id: NodeId,
}
