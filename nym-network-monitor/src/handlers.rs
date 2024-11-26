use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use futures::StreamExt;
use log::{debug, error, warn};
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sphinx::chunking::{monitoring, ReceivedFragment, SentFragment};
use petgraph::{dot::Dot, Graph};
use rand::{distributions::Alphanumeric, seq::SliceRandom, Rng};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::time::timeout;
use utoipa::ToSchema;

use crate::{
    accounting::{all_node_stats, NetworkAccount, NetworkAccountStats, NodeStats},
    http::AppState,
    MIXNET_TIMEOUT,
};

#[derive(ToSchema, Serialize)]
pub struct FragmentsSent(HashMap<i32, Vec<SentFragment>>);

#[derive(ToSchema, Serialize)]
pub struct FragmentsReceived(HashMap<i32, Vec<ReceivedFragment>>);

#[utoipa::path(
    get,
    path = "/v1/stats",
    responses(
        (status = 200, description = "Returns statistics collected since startup", body = NetworkAccountStats),
    )
)]
pub async fn stats_handler() -> Result<Json<NetworkAccountStats>, StatusCode> {
    let account = NetworkAccount::finalize().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(account.into()))
}

#[utoipa::path(
    get,
    path = "/v1/node_stats/{mix_id}",
    responses(
        (status = 200, description = "Returns statistics for a given mix_id, collected since startup", body = NodeStats),
    )
)]
pub async fn node_stats_handler(Path(mix_id): Path<u32>) -> Result<Json<NodeStats>, StatusCode> {
    let account = NetworkAccount::finalize().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(account.node_stats(mix_id)))
}

#[utoipa::path(
    get,
    path = "/v1/node_stats",
    responses(
        (status = 200, description = "Returns statistics for all nodes, collected since startup, sorted by reliability", body = Vec<NodeStats>),
    )
)]
pub async fn all_nodes_stats_handler() -> Result<Json<Vec<NodeStats>>, StatusCode> {
    let mut stats = all_node_stats()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    stats.sort_by(|a, b| a.reliability().partial_cmp(&b.reliability()).unwrap());
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/v1/accounting",
    responses(
        (status = 200, description = "Returns raw aggregated data collected since startup", body = NetworkAccount),
    )
)]
pub async fn accounting_handler() -> Result<Json<NetworkAccount>, StatusCode> {
    NetworkAccount::finalize()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/v1/dot/{mix_id}",
    responses(
        (status = 200, description = "Returns Subgraph for a given *mix_id* in `dot` format", body = String),
    )
)]
pub async fn mix_dot_handler(Path(mix_id): Path<u32>) -> Result<String, StatusCode> {
    generate_dot(Some(mix_id))
}

#[utoipa::path(
    get,
    path = "/v1/dot",
    responses(
        (status = 200, description = "Returns entire tested network graph in `dot` format", body = String),
    )
)]
pub async fn graph_handler() -> Result<String, StatusCode> {
    generate_dot(None)
}

#[utoipa::path(
    get,
    path = "/v1/sent",
    responses(
        (status = 200, description = "Returns a map of all fragments sent by the network monitor", body = FragmentsSent),
    )
)]
pub async fn sent_handler() -> Json<FragmentsSent> {
    Json(FragmentsSent(
        (*monitoring::FRAGMENTS_SENT)
            .clone()
            .into_iter()
            .collect::<HashMap<_, _>>(),
    ))
}

#[utoipa::path(
    get,
    path = "/v1/received",
    responses(
        (status = 200, description = "Returns a map of all fragments received by the network monitor", body = FragmentsReceived),
    )
)]
pub async fn recv_handler() -> Json<FragmentsReceived> {
    Json(FragmentsReceived(
        (*monitoring::FRAGMENTS_RECEIVED)
            .clone()
            .into_iter()
            .collect::<HashMap<_, _>>(),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/send",
    responses(
        (status = 200, description = "Sends a message to itself through the mixnet", body = String),
    )
)]
pub async fn send_handler(State(state): State<AppState>) -> Result<String, StatusCode> {
    send_receive_mixnet(state).await
}

#[utoipa::path(
    get,
    path = "/v1/mermaid",
    responses(
        (status = 200, description = "Returns entire tested network graph in `mermaid` format", body = String),
    )
)]
pub async fn mermaid_handler() -> Result<String, StatusCode> {
    let account = NetworkAccount::finalize().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut mermaid = String::new();
    mermaid.push_str("flowchart LR;\n");
    for route in account.complete_routes() {
        mermaid.push_str(
            route
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join("-->")
                .as_str(),
        );
        mermaid.push('\n')
    }
    for route in account.incomplete_routes() {
        mermaid.push_str(
            route
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join("-- ❌ -->")
                .as_str(),
        );
        mermaid.push('\n')
    }
    Ok(mermaid)
}

async fn send_receive_mixnet(state: AppState) -> Result<String, StatusCode> {
    let msg: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let sent_msg = msg.clone();

    let client = {
        let mut clients = state.clients().write().await;
        if let Some(client) = clients.make_contiguous().choose(&mut rand::thread_rng()) {
            Arc::clone(client)
        } else {
            error!("No clients currently available");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let recv = Arc::clone(&client);
    let sender = Arc::clone(&client);

    let recv_handle = tokio::spawn(async move {
        match timeout(
            Duration::from_secs(*MIXNET_TIMEOUT.get().expect("Set at the begining")),
            recv.write().await.next(),
        )
        .await
        {
            Ok(Some(received)) => {
                debug!("Received: {}", String::from_utf8_lossy(&received.message));
            }
            Ok(None) => debug!("No message received"),
            Err(e) => warn!("Failed to receive message: {e}"),
        }
    });

    let send_handle = tokio::spawn(async move {
        let mixnet_sender = sender.read().await.split_sender();
        let our_address = *sender.read().await.nym_address();
        match timeout(
            Duration::from_secs(5),
            mixnet_sender.send_plain_message(our_address, &msg),
        )
        .await
        {
            Ok(_) => debug!("Sent message: {msg}"),
            Err(e) => warn!("Failed to send message: {e}"),
        };
    });

    let results = futures::future::join_all(vec![send_handle, recv_handle]).await;
    for result in results {
        match result {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to send/receive message: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok(sent_msg)
}

fn generate_dot(mix_id: Option<u32>) -> Result<String, StatusCode> {
    let account = NetworkAccount::finalize().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut nodes = HashSet::new();
    let mut edges: Vec<(u32, u32)> = vec![];
    let mut broken_edges: Vec<(u32, u32)> = vec![];

    let mix_id = mix_id.unwrap_or(0);

    for route in account.complete_routes().iter() {
        if mix_id == 0 || route.contains(&mix_id) {
            for window in route.windows(2) {
                nodes.insert(window[0]);
                nodes.insert(window[1]);
                edges.push((window[0], window[1]));
            }
        }
    }

    for route in account.incomplete_routes().iter() {
        if mix_id == 0 || route.contains(&mix_id) {
            for window in route.windows(2) {
                nodes.insert(window[0]);
                nodes.insert(window[1]);
                broken_edges.push((window[0], window[1]));
            }
        }
    }

    let mut graph = Graph::new();

    let node_indices: HashMap<u32, _> = nodes
        .iter()
        .map(|node| (*node, graph.add_node(*node)))
        .collect();

    for (from, to) in edges {
        graph.add_edge(node_indices[&from], node_indices[&to], "");
    }

    for (from, to) in broken_edges {
        graph.add_edge(node_indices[&from], node_indices[&to], "❌");
    }

    let dot = Dot::new(&graph);
    Ok(dot.to_string())
}
