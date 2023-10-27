use axum::extract::{Query, State};
use nym_mixnode_common::verloc::{AtomicVerlocResult, VerlocResult};
use nym_node::http::api::{FormattedResponse, OutputParams};
use rocket::serde::json::Json;
use rocket::State as RocketState;

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
#[get("/verloc")]
pub(crate) async fn verloc(state: &RocketState<VerlocState>) -> Json<VerlocResult> {
    // since it's impossible to get a mutable reference to the state, we can't cache any results outside the lock : (
    Json(state.shared.clone_data().await)
}

/// Provides verifiable location (verloc) measurements for this mixnode - a list of the
/// round-trip times, in milliseconds, for all other mixnodes that this node knows about.
pub(crate) async fn verloc_axum(
    State(verloc): State<VerlocState>,
    Query(output): Query<OutputParams>,
) -> MixnodeVerlocResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(verloc.shared.clone_data().await)
}

pub type MixnodeVerlocResponse = FormattedResponse<VerlocResult>;
