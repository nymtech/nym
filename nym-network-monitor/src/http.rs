use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use dashmap::DashMap;
use futures::StreamExt;
use log::{debug, error, warn};
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sphinx::chunking::{ReceivedFragment, SentFragment, FRAGMENTS_RECEIVED, FRAGMENTS_SENT};
use nym_topology::{gateway, mix, NymTopology};
use petgraph::dot::Dot;
use petgraph::Graph;
use rand::distributions::Alphanumeric;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::IntoFuture;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::ClientsWrapper;

pub struct HttpServer {
    listener: SocketAddr,
    cancel: CancellationToken,
}

#[derive(Clone)]
struct AppState {
    clients: ClientsWrapper,
}

impl HttpServer {
    pub fn new(listener: SocketAddr, cancel: CancellationToken) -> Self {
        HttpServer { listener, cancel }
    }

    pub async fn run(self, clients: ClientsWrapper) -> anyhow::Result<()> {
        let n_clients = clients.read().await.len();
        let state = AppState { clients };
        let app = Router::new()
            .route("/", get(handler).with_state(state))
            .route("/accounting", get(accounting_handler))
            .route("/sent", get(sent_handler))
            .route("/dot/:node_address", get(mix_graph_handler))
            .route("/dot", get(graph_handler))
            .route("/mermaid", get(mermaid_handler))
            .route("/stats", get(stats_handler))
            .route("/received", get(recv_handler));
        let listener = tokio::net::TcpListener::bind(self.listener).await?;

        let shutdown_future = self.cancel.cancelled();
        let server_future = axum::serve(listener, app).into_future();

        println!("##########################################################################################");
        println!("######################### HTTP server running, with {} clients ############################################", n_clients);
        println!("##########################################################################################");

        tokio::select! {
            _ = shutdown_future => {
                println!("received shutdown");
            }
            res = server_future => {
                println!("the http server has terminated");
                if let Err(err) = res {
                    println!("with the following error: {err}");
                    return Err(err.into())
                }
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct NetworkAccountStats {
    complete_fragment_sets: usize,
    incomplete_fragment_sets: usize,
    missing_fragments: usize,
    complete_routes: usize,
    incomplete_routes: usize,
    tested_nodes: usize,
}

impl From<NetworkAccount> for NetworkAccountStats {
    fn from(account: NetworkAccount) -> Self {
        NetworkAccountStats {
            complete_fragment_sets: account.complete_fragment_sets.len(),
            incomplete_fragment_sets: account.incomplete_fragment_sets.len(),
            missing_fragments: account.missing_fragments.values().map(|v| v.len()).sum(),
            complete_routes: account.complete_routes.len(),
            incomplete_routes: account.incomplete_routes.len(),
            tested_nodes: account.tested_nodes.len(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct NetworkAccount {
    complete_fragment_sets: Vec<i32>,
    incomplete_fragment_sets: Vec<i32>,
    missing_fragments: HashMap<i32, Vec<u8>>,
    complete_routes: Vec<Vec<u32>>,
    incomplete_routes: Vec<Vec<u32>>,
    #[serde(skip)]
    topology: NymTopology,
    tested_nodes: HashSet<u32>,
}

impl NetworkAccount {
    fn finalize() -> Self {
        let mut account = NetworkAccount::new();
        account.find_incomplete_fragments();
        account.hydrate_all_fragments();
        account
    }

    fn new() -> Self {
        let topology = NymTopology::new_from_file("topology.json").unwrap();
        let mut account = NetworkAccount {
            topology,
            ..Default::default()
        };
        for fragment_set in FRAGMENTS_SENT.iter() {
            let sent_fragments = fragment_set
                .value()
                .first()
                .map(|f| f.header().total_fragments())
                .unwrap_or(0);

            debug!(
                "SENT Fragment set {} has {} fragments",
                fragment_set.key(),
                sent_fragments
            );

            let recv = FRAGMENTS_RECEIVED.get(fragment_set.key());
            let recv_fragments = recv.as_ref().map(|r| r.value().len()).unwrap_or(0);
            debug!(
                "RECV Fragment set {} has {} fragments",
                fragment_set.key(),
                recv_fragments
            );

            // Due to retransmission we can recieve a fragment multiple times
            if sent_fragments as usize <= recv_fragments {
                account.push_complete(*fragment_set.key());
            } else {
                account.push_incomplete(*fragment_set.key());
            }
        }
        account
    }

    pub fn hydrate_route(&self, fragment: SentFragment) -> (Vec<mix::Node>, gateway::Node) {
        let mut rng = ChaCha8Rng::seed_from_u64(fragment.seed() as u64);
        self.topology
            .random_path_to_gateway(
                &mut rng,
                fragment.mixnet_params().hops(),
                fragment.mixnet_params().destination(),
            )
            .unwrap()
    }

    fn hydrate_all_fragments(&mut self) {
        for fragment_id in &self.complete_fragment_sets {
            let fragment_set = FRAGMENTS_SENT.get(fragment_id).unwrap();
            let path = self.hydrate_route(fragment_set.value().first().unwrap().clone());

            let route = path.0.iter().map(|n| n.mix_id).collect::<Vec<u32>>();

            for node in &route {
                self.tested_nodes.insert(*node);
            }
            self.complete_routes.push(route);
        }

        for fragment_id in &self.incomplete_fragment_sets {
            let fragment_set = FRAGMENTS_SENT.get(fragment_id).unwrap();
            let route = self
                .hydrate_route(fragment_set.value().first().unwrap().clone())
                .0
                .iter()
                .map(|n| n.mix_id)
                .collect::<Vec<u32>>();
            self.incomplete_routes.push(route);
        }
    }

    fn find_incomplete_fragments(&mut self) {
        let mut missing_fragments_map = HashMap::new();
        for fragment_set_id in &self.incomplete_fragment_sets {
            if let Some(fragment_ref) = FRAGMENTS_RECEIVED.get(fragment_set_id) {
                let ref_fragment = fragment_ref.value().first().unwrap().header();
                let ref_id_set = (0..ref_fragment.total_fragments()).collect::<HashSet<u8>>();
                let recieved_set = fragment_ref
                    .value()
                    .iter()
                    .map(|f| f.header().current_fragment())
                    .collect::<HashSet<u8>>();
                let missing_fragments = ref_id_set
                    .difference(&recieved_set)
                    .cloned()
                    .collect::<Vec<u8>>();
                missing_fragments_map.insert(*fragment_set_id, missing_fragments);
            };
        }
        self.missing_fragments = missing_fragments_map;
    }

    fn push_complete(&mut self, id: i32) {
        self.complete_fragment_sets.push(id);
    }

    fn push_incomplete(&mut self, id: i32) {
        self.incomplete_fragment_sets.push(id);
    }
}

async fn stats_handler() -> Json<NetworkAccountStats> {
    let account = NetworkAccount::finalize();
    Json(account.into())
}

async fn accounting_handler() -> Json<NetworkAccount> {
    let account = NetworkAccount::finalize();
    Json(account)
}

fn generate_dot(mix_id: Option<u32>) -> String {
    let account = NetworkAccount::finalize();
    let mut nodes = HashSet::new();
    let mut edges: Vec<(u32, u32)> = vec![];
    let mut broken_edges: Vec<(u32, u32)> = vec![];

    let mix_id = mix_id.unwrap_or(0);

    for route in &account.complete_routes {
        if mix_id == 0 || route.contains(&mix_id) {
            for window in route.windows(2) {
                nodes.insert(window[0]);
                nodes.insert(window[1]);
                edges.push((window[0], window[1]));
            }
        }
    }

    for route in &account.incomplete_routes {
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
    dot.to_string()
}

async fn mix_graph_handler(Path(mix_id): Path<u32>) -> String {
    generate_dot(Some(mix_id))
}

async fn graph_handler() -> String {
    generate_dot(None)
}

async fn mermaid_handler() -> String {
    let account = NetworkAccount::finalize();
    let mut mermaid = String::new();
    mermaid.push_str("flowchart LR;\n");
    for route in account.complete_routes {
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
    for route in account.incomplete_routes {
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
    mermaid
}

async fn handler(State(state): State<AppState>) -> Result<String, StatusCode> {
    send_receive_mixnet(state).await
}

async fn sent_handler() -> Json<DashMap<i32, Vec<SentFragment>>> {
    Json((*FRAGMENTS_SENT).clone())
}

async fn recv_handler() -> Json<DashMap<i32, Vec<ReceivedFragment>>> {
    Json((*FRAGMENTS_RECEIVED).clone())
}

async fn send_receive_mixnet(state: AppState) -> Result<String, StatusCode> {
    let msg: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let sent_msg = msg.clone();

    let client = {
        let mut clients = state.clients.write().await;
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
        match timeout(Duration::from_secs(10), recv.write().await.next()).await {
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
