use crate::node::node_statistics::{NodeStats, NodeStatsWrapper};
use rocket::State;
use rocket_contrib::json::Json;

/// Returns a description of the node and why someone might want to delegate stake to it.
#[get("/stats")]
pub(crate) fn stats(description: State<NodeStatsWrapper>) -> Json<NodeStats> {
    todo!()
    // Json(description.inner().clone())
}
