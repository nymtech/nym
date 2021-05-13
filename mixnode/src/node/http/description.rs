use crate::node::node_description::NodeDescription;
use rocket::State;
use rocket_contrib::json::Json;

/// Provides verifiable location (verloc) measurements for this mixnode - a list of the
/// round-trip times, in milliseconds, for all other mixnodes that this node knows about.
#[get("/description")]
pub(crate) fn description(description: State<NodeDescription>) -> Json<NodeDescription> {
    Json(description.inner().clone())
}
