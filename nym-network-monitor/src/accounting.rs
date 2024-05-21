use std::collections::{HashMap, HashSet};

use log::debug;
use nym_sphinx::chunking::{SentFragment, FRAGMENTS_RECEIVED, FRAGMENTS_SENT};
use nym_topology::{gateway, mix, NymTopology};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct NetworkAccount {
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
    pub fn complete_routes(&self) -> &Vec<Vec<u32>> {
        &self.complete_routes
    }

    pub fn incomplete_routes(&self) -> &Vec<Vec<u32>> {
        &self.incomplete_routes
    }

    pub fn finalize() -> Self {
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

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
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
