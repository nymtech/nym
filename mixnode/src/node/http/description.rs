use rocket_contrib::json::Json;
use serde::Serialize;

/// Provides verifiable location (verloc) measurements for this mixnode - a list of the
/// round-trip times, in milliseconds, for all other mixnodes that this node knows about.
#[get("/description")]
pub(crate) fn description() -> Json<NodeDescription> {
    Json(description)
}
