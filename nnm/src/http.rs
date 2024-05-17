use axum::extract::{Path, State};
use axum::http::Response;
use axum::routing::get;
use axum::{Json, Router};
use dashmap::DashMap;
use futures::StreamExt;
use log::debug;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sphinx::chunking::{ReceivedFragment, SentFragment, FRAGMENTS_RECEIVED, FRAGMENTS_SENT};
use nym_sphinx::Node;
use nym_topology::NymTopology;
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
use tokio::sync::RwLock;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::ClientWrapper;

pub struct HttpServer {
    listener: SocketAddr,
    cancel: CancellationToken,
}

#[derive(Clone)]
struct AppState {
    clients: Arc<RwLock<Vec<Arc<RwLock<ClientWrapper>>>>>,
}

impl HttpServer {
    pub fn new(listener: SocketAddr, cancel: CancellationToken) -> Self {
        HttpServer { listener, cancel }
    }

    pub async fn run(
        self,
        clients: Arc<RwLock<Vec<Arc<RwLock<ClientWrapper>>>>>,
    ) -> anyhow::Result<()> {
        let n_clients = clients.read().await.len();
        let state = AppState { clients };
        let app = Router::new()
            .route("/", get(handler).with_state(state))
            .route("/a", get(accounting_handler))
            .route("/s", get(sent_handler))
            .route("/g/:node_address", get(graph_handler))
            .route("/m", get(mermaid_handler))
            .route("/stats", get(stats_handler))
            .route("/r", get(recv_handler));
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
    complete_routes: Vec<Vec<String>>,
    incomplete_routes: Vec<Vec<String>>,
    #[serde(skip)]
    topology: NymTopology,
    tested_nodes: HashSet<String>,
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

    pub fn hydrate_route(&self, fragment: SentFragment) -> Vec<Node> {
        let mut rng = ChaCha8Rng::seed_from_u64(fragment.seed() as u64);
        self.topology
            .random_route_to_gateway(
                &mut rng,
                fragment.mixnet_params().hops(),
                fragment.mixnet_params().destination(),
            )
            .unwrap()
    }

    fn hydrate_all_fragments(&mut self) {
        for fragment_id in &self.complete_fragment_sets {
            let fragment_set = FRAGMENTS_SENT.get(fragment_id).unwrap();
            let route = self
                .hydrate_route(fragment_set.value().first().unwrap().clone())
                .iter()
                .map(|n| n.address.as_base58_string())
                .collect::<Vec<String>>();
            for node in &route {
                self.tested_nodes.insert(node.clone());
            }
            self.complete_routes.push(route);
        }

        for fragment_id in &self.incomplete_fragment_sets {
            let fragment_set = FRAGMENTS_SENT.get(fragment_id).unwrap();
            let route = self
                .hydrate_route(fragment_set.value().first().unwrap().clone())
                .iter()
                .map(|n| n.address.as_base58_string())
                .collect::<Vec<String>>();
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

async fn graph_handler(Path(node_address): Path<String>) -> String {
    let account = NetworkAccount::finalize();
    let mut nodes = HashSet::new();
    let mut edges: Vec<(String, String)> = vec![];
    let mut broken_edges: Vec<(String, String)> = vec![];

    for route in &account.complete_routes {
        if !route.contains(&node_address) {
            continue;
        }

        for chunk in route.windows(2) {
            nodes.insert(chunk[0].clone());
            nodes.insert(chunk[1].clone());
            edges.push((chunk[0].clone(), chunk[1].clone()));
        }
    }

    for route in &account.incomplete_routes {
        if !route.contains(&node_address) {
            continue;
        }

        for chunk in route.windows(2) {
            nodes.insert(chunk[0].clone());
            nodes.insert(chunk[1].clone());
            broken_edges.push((chunk[0].clone(), chunk[1].clone()));
        }
    }

    let mut graph = Graph::new();

    let node_indices: HashMap<String, _> = nodes
        .iter()
        .map(|node| (node.clone(), graph.add_node(node.clone())))
        .collect();

    for (from, to) in edges {
        graph.add_edge(node_indices[&from], node_indices[&to], "✔️");
    }

    for (from, to) in broken_edges {
        graph.add_edge(node_indices[&from], node_indices[&to], "❌");
    }

    let dot = Dot::new(&graph);
    dot.to_string()
}

async fn mermaid_handler() -> String {
    let account = NetworkAccount::finalize();
    let mut mermaid = String::new();
    mermaid.push_str("flowchart LR;\n");
    for route in account.complete_routes {
        mermaid.push_str(route.join("-->").as_str());
        mermaid.push('\n')
    }
    for route in account.incomplete_routes {
        mermaid.push_str(route.join("-- ❌ -->").as_str());
        mermaid.push('\n')
    }
    mermaid
}

async fn handler(State(state): State<AppState>) -> Response<String> {
    send_receive_mixnet(state).await
}

async fn sent_handler() -> Json<DashMap<i32, Vec<SentFragment>>> {
    Json((*FRAGMENTS_SENT).clone())
}

async fn recv_handler() -> Json<DashMap<i32, Vec<ReceivedFragment>>> {
    Json((*FRAGMENTS_RECEIVED).clone())
}

async fn send_receive_mixnet(state: AppState) -> Response<String> {
    // let mut client = match make_client().await {
    //     Ok(client) => client,
    //     Err(e) => {
    //         return response
    //             .status(500)
    //             .body(format!("Failed to create mixnet client: {e}"))
    //             .unwrap();
    //     }
    // };

    let msg: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let sent_msg = msg.clone();

    let client = {
        let clients = state.clients.read().await;
        Arc::clone(clients.choose(&mut rand::thread_rng()).unwrap())
    };
    // Be able to get our client address
    // println!("Our client nym address is: {our_address}");
    let recv = Arc::clone(&client);
    let sender = Arc::clone(&client);

    let recv_handle = tokio::spawn(async move {
        match timeout(Duration::from_secs(10), recv.write().await.client.next()).await {
            Ok(Some(received)) => {
                println!("Received: {}", String::from_utf8_lossy(&received.message));
            }
            Ok(None) => println!("No message received"),
            Err(e) => println!("Failed to receive message: {e}"),
        }
    });

    let send_handle = tokio::spawn(async move {
        let mixnet_sender = sender.read().await.client.split_sender();
        let our_address = *sender.read().await.client.nym_address();
        match timeout(
            Duration::from_secs(5),
            mixnet_sender.send_plain_message(our_address, &msg),
        )
        .await
        {
            Ok(_) => println!("Sent message: {msg}"),
            Err(e) => println!("Failed to send message: {e}"),
        };
    });

    let results = futures::future::join_all(vec![send_handle, recv_handle]).await;
    for result in results {
        match result {
            Ok(_) => {}
            Err(e) => {
                let response = Response::builder();
                return response
                    .status(500)
                    .body(format!("Failed to send or receive message: {e}"))
                    .unwrap();
            }
        }
    }
    // wait for both tasks to be done
    // println!("waiting for shutdown");

    // match sending_task_handle.await {
    //     Ok(_) => {}
    //     Err(e) => {
    //         let response = Response::builder();
    //         return response
    //             .status(500)
    //             .body(format!("Failed to send message: {e}"))
    //             .unwrap();
    //     }
    // };
    // match recv_handle.await {
    //     Ok(_) => {}
    //     Err(e) => {
    //         let response = Response::builder();
    //         return response
    //             .status(500)
    //             .body(format!("Failed to receive message: {e}"))
    //             .unwrap();
    //     }
    // };
    let response = Response::builder();
    response.status(200).body(sent_msg).unwrap()
}
