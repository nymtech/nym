// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::node_description::NodeDescription;
use axum::extract::Query;
use nym_http_api_common::{FormattedResponse, OutputParams};

/// Returns a description of the node and why someone might want to delegate stake to it.
pub(crate) async fn description(
    description: NodeDescription,
    Query(output): Query<OutputParams>,
) -> MixnodeDescriptionResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(description)
}

pub type MixnodeDescriptionResponse = FormattedResponse<NodeDescription>;
