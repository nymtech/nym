// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::{Query, State};
use nym_mixnode_common::verloc::{AtomicVerlocResult, VerlocResult};
use nym_node::http::api::{FormattedResponse, OutputParams};

#[derive(Clone, Default)]
pub(crate) struct VerlocState {
    shared: AtomicVerlocResult,
}

impl VerlocState {
    pub fn new(atomic_verloc_result: AtomicVerlocResult) -> Self {
        VerlocState {
            shared: atomic_verloc_result,
        }
    }
}

/// Provides verifiable location (verloc) measurements for this mixnode - a list of the
/// round-trip times, in milliseconds, for all other mixnodes that this node knows about.
pub(crate) async fn verloc(
    State(verloc): State<VerlocState>,
    Query(output): Query<OutputParams>,
) -> MixnodeVerlocResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(verloc.shared.clone_data().await)
}

pub type MixnodeVerlocResponse = FormattedResponse<VerlocResult>;
