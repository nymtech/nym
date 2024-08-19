use std::collections::{HashMap, HashSet};

use anyhow::Result;
use futures::{stream::FuturesUnordered, StreamExt};
use log::{debug, info};
use nym_sphinx::chunking::{SentFragment, FRAGMENTS_RECEIVED, FRAGMENTS_SENT};
use nym_topology::{gateway, mix, NymTopology};
use nym_types::monitoring::{MonitorMessage, NodeResult};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{NYM_API_URL, PRIVATE_KEY, TOPOLOGY};

struct HydratedRoute {
    mix_nodes: Vec<mix::Node>,
    gateway_node: gateway::Node,
}

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
struct GatewayStats(u32, u32, Option<String>);

impl GatewayStats {
    fn new(sent: u32, recv: u32, owner: Option<String>) -> Self {
        GatewayStats(sent, recv, owner)
    }

    fn success(&self) -> u32 {
        self.0
    }

    fn failed(&self) -> u32 {
        self.1
    }

    fn reliability(&self) -> f64 {
        self.success() as f64 / (self.success() + self.failed()) as f64
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
    topology: NymTopology,
    tested_nodes: HashSet<u32>,
    #[serde(skip)]
    mix_details: HashMap<u32, mix::Node>,
    #[serde(skip)]
    gateway_details: HashMap<String, gateway::Node>,
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
            node.owner.clone(),
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
        FRAGMENTS_SENT.clear();
        FRAGMENTS_RECEIVED.clear();
    }

    fn new() -> Self {
        let topology = TOPOLOGY.get().expect("Topology not set yet!").clone();
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

    fn hydrate_route(&self, fragment: SentFragment) -> anyhow::Result<HydratedRoute> {
        let mut rng = ChaCha8Rng::seed_from_u64(fragment.seed() as u64);
        let (nodes, gw) = self.topology.random_path_to_gateway(
            &mut rng,
            fragment.mixnet_params().hops(),
            fragment.mixnet_params().destination(),
        )?;
        Ok(HydratedRoute {
            mix_nodes: nodes,
            gateway_node: gw,
        })
    }

    fn hydrate_all_fragments(&mut self) -> Result<()> {
        for fragment_set in FRAGMENTS_SENT.iter() {
            let fragment_set_id = fragment_set.key();
            for fragment in fragment_set.value() {
                let route = self.hydrate_route(fragment.clone())?;
                let mix_ids = route
                    .mix_nodes
                    .iter()
                    .map(|n| n.mix_id)
                    .collect::<Vec<u32>>();
                self.tested_nodes.extend(&mix_ids);
                self.mix_details
                    .extend(route.mix_nodes.iter().map(|n| (n.mix_id, n.clone())));
                let gateway_stats_entry = self
                    .gateway_stats
                    .entry(route.gateway_node.identity_key.to_base58_string())
                    .or_insert(GatewayStats::new(0, 0, route.gateway_node.owner.clone()));
                self.gateway_details.insert(
                    route.gateway_node.identity_key.to_base58_string(),
                    route.gateway_node,
                );
                if self.complete_fragment_sets.contains(fragment_set_id) {
                    self.complete_routes.push(mix_ids);
                    gateway_stats_entry.incr_success();
                } else {
                    self.incomplete_routes.push(mix_ids);
                    gateway_stats_entry.incr_failure();
                }
            }
        }
        Ok(())
    }

    fn find_missing_fragments(&mut self) {
        let mut missing_fragments_map = HashMap::new();
        for fragment_set_id in &self.incomplete_fragment_sets {
            if let Some(fragment_ref) = FRAGMENTS_RECEIVED.get(fragment_set_id) {
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
    owner: Option<String>,
}

impl NodeStats {
    pub fn new(
        mix_id: u32,
        complete_routes: usize,
        incomplete_routes: usize,
        identity: String,
        owner: Option<String>,
    ) -> Self {
        NodeStats {
            mix_id,
            complete_routes,
            incomplete_routes,
            reliability: complete_routes as f64 / (complete_routes + incomplete_routes) as f64,
            identity,
            owner,
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

pub async fn submit_metrics() -> anyhow::Result<()> {
    let node_stats = monitor_mixnode_results().await?;
    let gateway_stats = monitor_gateway_results().await?;

    info!("Submitting metrics to {}", *NYM_API_URL);
    let client = reqwest::Client::new();

    let node_submit_url = format!("{}/v1/status/submit_node", &*NYM_API_URL);
    let gateway_submit_url = format!("{}/v1/status/submit_gateway", &*NYM_API_URL);

    info!("Submitting {} mixnode measurements", node_stats.len());

    node_stats
        .chunks(10)
        .map(|chunk| {
            let monitor_message =
                MonitorMessage::new(chunk.to_vec(), PRIVATE_KEY.get().expect("We've set this!"));
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
            let monitor_message =
                MonitorMessage::new(chunk.to_vec(), PRIVATE_KEY.get().expect("We've set this!"));
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
