use crate::node::node_statistics::{NodeStats, NodeStatsSimple, SharedNodeStats};
use rocket::serde::json::Json;
use rocket::State;
use serde::Serialize;

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum NodeStatsResponse {
    Full(NodeStats),
    Simple(NodeStatsSimple),
}

/// Returns a running stats of the node.
#[get("/stats?<debug>")]
pub(crate) async fn stats(
    stats: &State<SharedNodeStats>,
    debug: Option<bool>,
) -> Json<NodeStatsResponse> {
    let snapshot_data = stats.clone_data().await;

    // there's no point in returning the entire hashmap of sending destinations in regular mode
    if let Some(debug) = debug {
        if debug {
            return Json(NodeStatsResponse::Full(snapshot_data));
        }
    }

    Json(NodeStatsResponse::Simple(snapshot_data.simplify()))
}
