use std::sync::Arc;

use chitchat::{Chitchat, ClusterStateSnapshot, NodeId};
use rocket::serde::json::Json;
use rocket::State;
use serde::Serialize;
use tokio::sync::Mutex;

#[derive(Serialize)]
pub struct ClusterState {
    cluster_id: String,
    cluster_state: ClusterStateSnapshot,
    live_nodes: Vec<NodeId>,
    dead_nodes: Vec<NodeId>,
}

/// Returns a description of the node and why someone might want to delegate stake to it.
#[get("/state")]
pub(crate) async fn state(chitchat: &State<Arc<Mutex<Chitchat>>>) -> Json<ClusterState> {
    let chitchat_guard = chitchat.lock().await;
    let cluster_state = ClusterState {
        cluster_id: chitchat_guard.cluster_id().to_string(),
        cluster_state: chitchat_guard.state_snapshot(),
        live_nodes: chitchat_guard.live_nodes().cloned().collect::<Vec<_>>(),
        dead_nodes: chitchat_guard.dead_nodes().cloned().collect::<Vec<_>>(),
    };
    Json(cluster_state)
}
