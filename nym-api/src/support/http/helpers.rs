// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_http_api_common::Output;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Serialize, Deserialize, Debug, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PaginationRequest {
    pub output: Option<Output>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
#[schema(title = "NodeId")]
#[schema(as = NodeId)]
#[into_params(parameter_in = Path)]
pub(crate) struct NodeIdParam {
    #[schema(value_type = u32)]
    pub(crate) node_id: NodeId,
}
