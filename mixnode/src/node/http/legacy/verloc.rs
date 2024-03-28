// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_http_api::api::api_requests::v1::metrics::models::VerlocResultData;
use nym_node_http_api::state::metrics::SharedVerlocStats;

/// Provides verifiable location (verloc) measurements for this mixnode - a list of the
/// round-trip times, in milliseconds, for all other mixnodes that this node knows about.
pub(crate) async fn verloc(
    State(verloc): State<SharedVerlocStats>,
    Query(output): Query<OutputParams>,
) -> MixnodeVerlocResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(verloc.read().await.current_run_data.clone())
}

pub type MixnodeVerlocResponse = FormattedResponse<VerlocResultData>;
