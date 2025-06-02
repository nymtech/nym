use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use futures::{pin_mut, stream::FuturesUnordered, StreamExt};
use log::{debug, error, info, warn};
use nym_sphinx::chunking::{monitoring, SentFragment};
use nym_topology::{NymRouteProvider, RoutingNode};
use nym_types::monitoring::{MonitorMessage, MonitorResults, NodeResult, RouteResult};
use nym_validator_client::nym_api::routes::{
    API_VERSION, STATUS, SUBMIT_GATEWAY, SUBMIT_NODE, SUBMIT_ROUTE,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_postgres::{binary_copy::BinaryCopyInWriter, types::Type, Client, NoTls};
use utoipa::ToSchema;

use crate::{NYM_API_URLS, PRIVATE_KEY, TOPOLOGY};

struct HydratedRoute {
    mix_nodes: Vec<RoutingNode>,
    gateway_node: RoutingNode,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
struct AccountingRoute {
    mix_nodes: (u32, u32, u32),
    gateway_node: u32,
    success: bool,
}

impl AccountingRoute {
    fn from_complete(route: &HydratedRoute) -> Self {
        Self {
            mix_nodes: (
                route.mix_nodes[0].node_id,
                route.mix_nodes[1].node_id,
                route.mix_nodes[2].node_id,
            ),
            gateway_node: route.gateway_node.node_id,
            success: true,
        }
    }

    fn from_incomplete(route: &HydratedRoute) -> Self {
        Self {
            mix_nodes: (
                route.mix_nodes[0].node_id,
                route.mix_nodes[1].node_id,
                route.mix_nodes[2].node_id,
            ),
            gateway_node: route.gateway_node.node_id,
            success: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
struct GatewayStats(u32, u32);

impl GatewayStats {
    fn new(success: u32, failure: u32) -> Self {
        GatewayStats(success, failure)
    }

    fn success(&self) -> u32 {
        self.0
    }

    fn failure(&self) -> u32 {
        self.1
    }

    fn reliability(&self) -> f64 {
        self.success() as f64 / (self.success() + self.failure()) as f64
    }

    fn incr_success(&mut self) {
        self.0 += 1;
    }

    fn incr_failure(&mut self) {
        self.1 += 1;
    }
}

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct NetworkAccount {
    complete_fragment_sets: HashSet<i32>,
    incomplete_fragment_sets: HashSet<i32>,
    missing_fragments: HashMap<i32, Vec<u8>>,
    complete_routes: Vec<Vec<u32>>,
    gateway_stats: HashMap<String, GatewayStats>,
    incomplete_routes: Vec<Vec<u32>>,
    #[serde(skip)]
    topology: NymRouteProvider,
    tested_nodes: HashSet<u32>,
    #[serde(skip)]
    mix_details: HashMap<u32, RoutingNode>,
    #[serde(skip)]
    gateway_details: HashMap<String, RoutingNode>,
    #[serde(skip)]
    accounting_routes: Vec<AccountingRoute>,
}

impl NetworkAccount {
    pub fn tested_nodes(&self) -> &HashSet<u32> {
        &self.tested_nodes
    }

    pub fn node_stats(&self, id: u32) -> NodeStats {
        let complete_routes = self.complete_for_id(id);
        let incomplete_routes = self.incomplete_for_id(id);
        let node = self
            .mix_details
            .get(&id)
            .expect("Has to be in here, since we've put it in!");
        NodeStats::new(
            id,
            complete_routes,
            incomplete_routes,
            node.identity_key.to_base58_string(),
        )
    }

    fn complete_for_id(&self, id: u32) -> usize {
        self.complete_routes()
            .iter()
            .filter(|r| r.contains(&id))
            .count()
    }

    fn incomplete_for_id(&self, id: u32) -> usize {
        self.incomplete_routes()
            .iter()
            .filter(|r| r.contains(&id))
            .count()
    }

    pub fn complete_routes(&self) -> &Vec<Vec<u32>> {
        &self.complete_routes
    }

    pub fn incomplete_routes(&self) -> &Vec<Vec<u32>> {
        &self.incomplete_routes
    }

    pub fn finalize() -> Result<Self> {
        let mut account = NetworkAccount::new();
        account.find_missing_fragments();
        account.hydrate_all_fragments()?;
        Ok(account)
    }

    pub fn empty_buffers() {
        monitoring::FRAGMENTS_SENT.clear();
        monitoring::FRAGMENTS_RECEIVED.clear();
    }

    fn new() -> Self {
        let topology = TOPOLOGY.get().expect("Topology not set yet!").clone();
        let mut account = NetworkAccount {
            topology: NymRouteProvider::new(topology, true),
            ..Default::default()
        };
        for fragment_set in monitoring::FRAGMENTS_SENT.iter() {
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

            let recv = monitoring::FRAGMENTS_RECEIVED.get(fragment_set.key());
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

    fn hydrate_route(&self, fragment: SentFragment) -> anyhow::Result<HydratedRoute> {
        let mut rng = ChaCha8Rng::seed_from_u64(fragment.seed() as u64);
        let (nodes, gw) = self
            .topology
            .random_path_to_egress(&mut rng, fragment.mixnet_params().destination())?;
        Ok(HydratedRoute {
            mix_nodes: nodes.into_iter().cloned().collect(),
            gateway_node: gw.clone(),
        })
    }

    fn hydrate_all_fragments(&mut self) -> Result<()> {
        for fragment_set in monitoring::FRAGMENTS_SENT.iter() {
            let fragment_set_id = fragment_set.key();
            for fragment in fragment_set.value() {
                let route = self.hydrate_route(fragment.clone())?;
                let mix_ids = route
                    .mix_nodes
                    .iter()
                    .map(|n| n.node_id)
                    .collect::<Vec<u32>>();
                self.tested_nodes.extend(&mix_ids);
                self.mix_details
                    .extend(route.mix_nodes.iter().map(|n| (n.node_id, n.clone())));
                let gateway_stats_entry = self
                    .gateway_stats
                    .entry(route.gateway_node.identity_key.to_base58_string())
                    .or_insert(GatewayStats::new(0, 0));
                self.gateway_details.insert(
                    route.gateway_node.identity_key.to_base58_string(),
                    route.gateway_node.clone(),
                );
                if self.complete_fragment_sets.contains(fragment_set_id) {
                    self.complete_routes.push(mix_ids);
                    gateway_stats_entry.incr_success();
                    self.accounting_routes
                        .push(AccountingRoute::from_complete(&route));
                } else {
                    self.incomplete_routes.push(mix_ids);
                    gateway_stats_entry.incr_failure();
                    self.accounting_routes
                        .push(AccountingRoute::from_incomplete(&route));
                }
            }
        }
        Ok(())
    }

    fn find_missing_fragments(&mut self) {
        let mut missing_fragments_map = HashMap::new();
        for fragment_set_id in &self.incomplete_fragment_sets {
            if let Some(fragment_ref) = monitoring::FRAGMENTS_RECEIVED.get(fragment_set_id) {
                if let Some(ref_fragment) = fragment_ref.value().first() {
                    let ref_header = ref_fragment.header();
                    let ref_id_set = (0..ref_header.total_fragments()).collect::<HashSet<u8>>();
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
                }
            };
        }
        self.missing_fragments = missing_fragments_map;
    }

    fn push_complete(&mut self, id: i32) {
        self.complete_fragment_sets.insert(id);
    }

    fn push_incomplete(&mut self, id: i32) {
        self.incomplete_fragment_sets.insert(id);
    }
}

#[derive(Serialize, Debug, Default, ToSchema)]
pub struct NetworkAccountStats {
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

#[derive(Serialize, Debug, ToSchema)]
pub struct NodeStats {
    mix_id: u32,
    complete_routes: usize,
    incomplete_routes: usize,
    reliability: f64,
    identity: String,
}

impl NodeStats {
    pub fn new(
        mix_id: u32,
        complete_routes: usize,
        incomplete_routes: usize,
        identity: String,
    ) -> Self {
        NodeStats {
            mix_id,
            complete_routes,
            incomplete_routes,
            reliability: complete_routes as f64 / (complete_routes + incomplete_routes) as f64,
            identity,
        }
    }

    pub fn reliability(&self) -> f64 {
        self.reliability
    }

    pub fn into_node_results(self) -> NodeResult {
        NodeResult {
            node_id: self.mix_id,
            identity: self.identity,
            reliability: (self.reliability * 100.) as u8,
        }
    }
}

pub async fn all_node_stats() -> anyhow::Result<Vec<NodeStats>> {
    let account = NetworkAccount::finalize()?;
    Ok(account
        .tested_nodes()
        .iter()
        .map(|id| account.node_stats(*id))
        .collect::<Vec<NodeStats>>())
}

pub async fn monitor_gateway_results() -> anyhow::Result<Vec<NodeResult>> {
    let account = NetworkAccount::finalize()?;
    Ok(account
        .gateway_stats
        .iter()
        .map(into_gateway_result)
        .collect())
}

pub async fn monitor_mixnode_results() -> anyhow::Result<Vec<NodeResult>> {
    let stats = all_node_stats().await?;
    Ok(stats
        .into_iter()
        .map(NodeStats::into_node_results)
        .collect())
}

async fn submit_node_stats_to_db(client: Arc<Client>) -> anyhow::Result<()> {
    let client = Arc::clone(&client);
    let node_stats = all_node_stats().await?;

    let sink = client
        .copy_in("COPY node_stats (node_id, identity, reliability, complete_routes, incomplete_routes) FROM STDIN BINARY")
        .await?;

    let writer = BinaryCopyInWriter::new(
        sink,
        &[Type::INT4, Type::TEXT, Type::FLOAT8, Type::INT8, Type::INT8],
    );
    pin_mut!(writer);

    for stat in node_stats {
        writer
            .as_mut()
            .write(&[
                &(stat.mix_id as i32),
                &stat.identity,
                &stat.reliability,
                &(stat.complete_routes as i64),
                &(stat.incomplete_routes as i64),
            ])
            .await?;
    }

    writer.finish().await?;

    Ok(())
}

async fn submit_gateway_stats_to_db(client: Arc<Client>) -> anyhow::Result<()> {
    let client = Arc::clone(&client);
    let network_account = NetworkAccount::finalize()?;
    let gateway_stats = network_account.gateway_stats;

    let sink = client
        .copy_in("COPY gateway_stats (identity, reliability, success, failure) FROM STDIN BINARY")
        .await?;

    let writer = BinaryCopyInWriter::new(sink, &[Type::TEXT, Type::FLOAT8, Type::INT8, Type::INT8]);
    pin_mut!(writer);

    for (key, stats) in gateway_stats {
        writer
            .as_mut()
            .write(&[
                &key,
                &stats.reliability(),
                &(stats.success() as i64),
                &(stats.failure() as i64),
            ])
            .await?;
    }

    writer.finish().await?;

    Ok(())
}

async fn db_connection(database_url: Option<&String>) -> Result<Option<(Client, JoinHandle<()>)>> {
    if let Some(database_url) = database_url {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;

        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error: {}", e);
            }
        });

        Ok(Some((client, handle)))
    } else {
        Ok(None)
    }
}

pub async fn submit_metrics_to_db(database_url: Option<&String>) -> anyhow::Result<()> {
    if let Some((client, handle)) = db_connection(database_url).await? {
        let client = Arc::new(client);
        let client2 = Arc::clone(&client);
        let client3 = Arc::clone(&client);
        submit_node_stats_to_db(client).await?;
        submit_gateway_stats_to_db(client2).await?;
        submit_accounting_routes_to_db(client3).await?;
        handle.abort();
    }
    Ok(())
}

async fn submit_accounting_routes_to_db(client: Arc<Client>) -> anyhow::Result<()> {
    let client = Arc::clone(&client);
    let network_account = NetworkAccount::finalize()?;
    let accounting_routes = network_account.accounting_routes;

    let sink = client
        .copy_in("COPY routes (layer1, layer2, layer3, gw, success) FROM STDIN BINARY")
        .await?;

    let writer = BinaryCopyInWriter::new(
        sink,
        &[Type::INT4, Type::INT4, Type::INT4, Type::INT4, Type::BOOL],
    );
    pin_mut!(writer);

    for route in accounting_routes {
        writer
            .as_mut()
            .write(&[
                &(route.mix_nodes.0 as i32),
                &(route.mix_nodes.1 as i32),
                &(route.mix_nodes.2 as i32),
                &(route.gateway_node as i32),
                &route.success,
            ])
            .await?;
    }

    writer.finish().await?;

    Ok(())
}

pub async fn submit_metrics(database_url: Option<&String>) -> anyhow::Result<()> {
    if let Err(e) = submit_metrics_to_db(database_url).await {
        error!("Error submitting metrics to db: {}", e);
    }

    if let Some(private_key) = PRIVATE_KEY.get() {
        if let Some(nym_api_urls) = NYM_API_URLS.get() {
            info!("Submitting metrics to {} nym apis", nym_api_urls.len());
            for nym_api_url in nym_api_urls {
                info!("Submitting metrics to {}", nym_api_url);
                let node_stats = monitor_mixnode_results().await?;
                let gateway_stats = monitor_gateway_results().await?;
                let client = reqwest::Client::new();

                let node_submit_url =
                    format!("{}/{API_VERSION}/{STATUS}/{SUBMIT_NODE}", nym_api_url);
                let gateway_submit_url = format!(
                    "{}/{API_VERSION}/{STATUS}/{SUBMIT_GATEWAY}",
                    nym_api_url
                );
                let route_submit_url =
                    format!("{}/{API_VERSION}/{STATUS}/{SUBMIT_ROUTE}", nym_api_url);

                info!("Submitting {} mixnode measurements", node_stats.len());

                node_stats
                    .chunks(10)
                    .map(|chunk| {
                        let monitor_results = MonitorResults::Node(chunk.to_vec());
                        let monitor_message = MonitorMessage::new(monitor_results, private_key);
                        client.post(&node_submit_url).json(&monitor_message).send()
                    })
                    .collect::<FuturesUnordered<_>>()
                    .collect::<Vec<Result<_, _>>>()
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()?;

                info!("Submitting {} gateway measurements", gateway_stats.len());

                gateway_stats
                    .chunks(10)
                    .map(|chunk| {
                        let monitor_results = MonitorResults::Node(chunk.to_vec());
                        let monitor_message = MonitorMessage::new(
                            monitor_results,
                            PRIVATE_KEY.get().expect("We've set this!"),
                        );
                        client
                            .post(&gateway_submit_url)
                            .json(&monitor_message)
                            .send()
                    })
                    .collect::<FuturesUnordered<_>>()
                    .collect::<Vec<Result<_, _>>>()
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()?;

                let network_account = NetworkAccount::finalize()?;
                let accounting_routes = network_account.accounting_routes;
                info!("Submitting {} accounting routes", accounting_routes.len());
                match accounting_routes
                    .chunks(10)
                    .map(|chunk| {
                        let route_results = chunk
                            .iter()
                            .map(|route| {
                                RouteResult::new(
                                    route.mix_nodes.0,
                                    route.mix_nodes.1,
                                    route.mix_nodes.2,
                                    route.gateway_node,
                                    route.success,
                                )
                            })
                            .collect::<Vec<RouteResult>>();
                        let monitor_results = MonitorResults::Route(route_results);
                        let monitor_message = MonitorMessage::new(monitor_results, private_key);
                        client.post(&route_submit_url).json(&monitor_message).send()
                    })
                    .collect::<FuturesUnordered<_>>()
                    .collect::<Vec<Result<_, _>>>()
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(_) => info!(
                        "Successfully submitted accounting routes to {}",
                        nym_api_url
                    ),
                    Err(e) => error!(
                        "Error submitting accounting routes to {}: {}",
                        nym_api_url, e
                    ),
                };
            }
        }
    } else {
        warn!("No private key or nym api urls found");
    }

    NetworkAccount::empty_buffers();

    Ok(())
}

fn into_gateway_result((key, stats): (&String, &GatewayStats)) -> NodeResult {
    NodeResult {
        identity: key.clone(),
        reliability: (stats.reliability() * 100.) as u8,
        node_id: 0,
    }
}
