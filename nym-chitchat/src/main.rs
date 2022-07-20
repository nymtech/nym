use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use chitchat::transport::UdpTransport;
use chitchat::{spawn_chitchat, Chitchat, ChitchatConfig, FailureDetectorConfig, NodeId};
use cool_id_generator::Size;
use poem::listener::TcpListener;
use poem::{Route, Server};
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{OpenApi, OpenApiService};
use structopt::StructOpt;
use tokio::sync::Mutex;

use chitchat::ClusterStateSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse {
    pub cluster_id: String,
    pub cluster_state: ClusterStateSnapshot,
    pub live_nodes: Vec<NodeId>,
    pub dead_nodes: Vec<NodeId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetKeyValueResponse {
    pub status: bool,
}

struct Api {
    chitchat: Arc<Mutex<Chitchat>>,
}

#[OpenApi]
impl Api {
    /// Chitchat state
    #[oai(path = "/", method = "get")]
    async fn index(&self) -> Json<serde_json::Value> {
        let chitchat_guard = self.chitchat.lock().await;
        let response = ApiResponse {
            cluster_id: chitchat_guard.cluster_id().to_string(),
            cluster_state: chitchat_guard.state_snapshot(),
            live_nodes: chitchat_guard.live_nodes().cloned().collect::<Vec<_>>(),
            dead_nodes: chitchat_guard.dead_nodes().cloned().collect::<Vec<_>>(),
        };
        Json(serde_json::to_value(&response).unwrap())
    }

    /// Set a key & value on this node (with no validation).
    #[oai(path = "/set_kv/", method = "get")]
    async fn set_kv(&self, key: Query<String>, value: Query<String>) -> Json<serde_json::Value> {
        let mut chitchat_guard = self.chitchat.lock().await;

        let cc_state = chitchat_guard.self_node_state();
        cc_state.set(key.as_str(), value.as_str());

        Json(serde_json::to_value(&SetKeyValueResponse { status: true }).unwrap())
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "chitchat", about = "Chitchat test server.")]
struct Opt {
    /// Defines the socket addr on which we should listen to.
    #[structopt(long = "listen_addr", default_value = "127.0.0.1:10000")]
    listen_addr: SocketAddr,
    /// Defines the socket_address (host:port) other servers should use to
    /// reach this server.
    ///
    /// It defaults to the listen address, but this is only valid
    /// when all server are running on the same server.
    #[structopt(long = "public_addr")]
    public_addr: Option<SocketAddr>,

    /// Node id. Has to be unique. If None, the node_id will be generated from
    /// the public_addr and a random suffix.
    #[structopt(long = "node_id")]
    node_id: Option<String>,

    #[structopt(long = "seed")]
    seeds: Vec<String>,

    #[structopt(long = "interval_ms", default_value = "500")]
    interval: u64,
}

fn generate_server_id(public_addr: SocketAddr) -> String {
    let cool_id = cool_id_generator::get_id(Size::Medium);
    format!("server:{}-{}", public_addr, cool_id)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let public_addr = opt.public_addr.unwrap_or(opt.listen_addr);
    let node_id_str = opt
        .node_id
        .unwrap_or_else(|| generate_server_id(public_addr));
    let node_id = NodeId::new(node_id_str, public_addr);
    let config = ChitchatConfig {
        node_id,
        cluster_id: "testing".to_string(),
        gossip_interval: Duration::from_millis(opt.interval),
        listen_addr: opt.listen_addr,
        seed_nodes: opt.seeds.clone(),
        failure_detector_config: FailureDetectorConfig::default(),
    };
    let chitchat_handler = spawn_chitchat(config, Vec::new(), &UdpTransport).await?;
    let chitchat = chitchat_handler.chitchat();
    let api = Api { chitchat };
    let api_service = OpenApiService::new(api, "Hello World", "1.0")
        .server(&format!("http://{}/", opt.listen_addr));
    let docs = api_service.swagger_ui();
    let app = Route::new().nest("/", api_service).nest("/docs", docs);
    Server::new(TcpListener::bind(&opt.listen_addr))
        .run(app)
        .await?;
    Ok(())
}
