// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::{FormattedResponse, OutputParams};
use axum::extract::Query;
use nym_node_requests::api::v1::network_requester::exit_policy::models::UsedExitPolicy;

/// Returns information about the exit policy used by this node.
#[utoipa::path(
    get,
    path = "/exit-policy",
    context_path = "/api/v1/network-requester",
    tag = "Network Requester",
    responses(
        (status = 200, content(
            ("application/json" = UsedExitPolicy),
            ("application/yaml" = UsedExitPolicy)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn node_exit_policy(
    policy: UsedExitPolicy,
    Query(output): Query<OutputParams>,
) -> ExitPolicyResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(policy)
}

pub type ExitPolicyResponse = FormattedResponse<UsedExitPolicy>;
