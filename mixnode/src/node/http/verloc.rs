use mixnode_common::verloc::{AtomicVerlocResult, VerlocResult};
use rocket::serde::json::Json;
use rocket::State;

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
pub(crate) async fn verloc(state: &State<VerlocState>) -> Json<VerlocResult> {
    // since it's impossible to get a mutable reference to the state, we can't cache any results outside the lock : (
    Json(state.shared.clone_data().await)
}
