use std::collections::{HashMap, HashSet};

use anyhow::Result;
use log::debug;
use nym_sphinx::chunking::{SentFragment, FRAGMENTS_RECEIVED, FRAGMENTS_SENT};
use nym_topology::{gateway, mix, NymTopology};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

struct HydratedRoute {
    mix_nodes: Vec<mix::Node>,
    #[allow(dead_code)]
    gateway_node: gateway::Node,
}

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct NetworkAccount {
    complete_fragment_sets: HashSet<i32>,
    incomplete_fragment_sets: HashSet<i32>,
    missing_fragments: HashMap<i32, Vec<u8>>,
    complete_routes: Vec<Vec<u32>>,
    incomplete_routes: Vec<Vec<u32>>,
    #[serde(skip)]
    topology: NymTopology,
    tested_nodes: HashSet<u32>,
}

impl NetworkAccount {
    pub fn node_stats(&self, id: u32) -> NodeStats {
        let complete_routes = self.complete_for_id(id);
        let incomplete_routes = self.incomplete_for_id(id);
        NodeStats::new(id, complete_routes, incomplete_routes)
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
                if self.complete_fragment_sets.contains(fragment_set_id) {
                    self.complete_routes.push(mix_ids);
                } else {
                    self.incomplete_routes.push(mix_ids);
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
}

impl NodeStats {
    pub fn new(mix_id: u32, complete_routes: usize, incomplete_routes: usize) -> Self {
        NodeStats {
            mix_id,
            complete_routes,
            incomplete_routes,
        }
    }
}
