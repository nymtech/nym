use std::fs::File;
use std::path::Path;

use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};

use mixnet_contract::MixNodeBond;

use crate::country_statistics::country_nodes_distribution::{
    ConcurrentCountryNodesDistribution, CountryNodesDistribution,
};
use crate::mix_node::models::ThreadsafeMixNodeCache;
use crate::mix_nodes::location::LocationCache;
use crate::mix_nodes::models::ThreadsafeMixNodesResult;
use crate::ping::models::ThreadsafePingCache;

// TODO: change to an environment variable with a default value
const STATE_FILE: &str = "explorer-api-state.json";

#[derive(Clone)]
pub struct ExplorerApiState {
    pub(crate) country_node_distribution: ConcurrentCountryNodesDistribution,
    pub(crate) mix_nodes: ThreadsafeMixNodesResult,
    pub(crate) mix_node_cache: ThreadsafeMixNodeCache,
    pub(crate) ping_cache: ThreadsafePingCache,
}

impl ExplorerApiState {
    pub(crate) async fn get_mix_node(&self, pubkey: &str) -> Option<MixNodeBond> {
        self.mix_nodes.get_mixnode(pubkey).await
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplorerApiStateOnDisk {
    pub(crate) country_node_distribution: CountryNodesDistribution,
    pub(crate) location_cache: LocationCache,
    pub(crate) as_at: DateTime<Utc>,
}

#[derive(Clone)]
pub(crate) struct ExplorerApiStateContext {
    pub(crate) inner: ExplorerApiState,
}

impl ExplorerApiStateContext {
    pub(crate) fn new() -> Self {
        ExplorerApiStateContext {
            inner: ExplorerApiStateContext::read_from_file(),
        }
    }

    pub(crate) fn read_from_file() -> ExplorerApiState {
        let json_file = get_state_file_path();
        let json_file_path = Path::new(&json_file);
        info!("Loading state from file {:?}...", json_file);

        match File::open(json_file_path).map(serde_json::from_reader::<_, ExplorerApiStateOnDisk>) {
            Ok(Ok(state)) => {
                info!("Loaded state from file {:?}: {:?}", json_file, state);
                ExplorerApiState {
                    country_node_distribution:
                        ConcurrentCountryNodesDistribution::new_from_distribution(
                            state.country_node_distribution,
                        ),
                    mix_nodes: ThreadsafeMixNodesResult::new_with_location_cache(
                        state.location_cache,
                    ),
                    mix_node_cache: ThreadsafeMixNodeCache::new(),
                    ping_cache: ThreadsafePingCache::new(),
                }
            }
            _ => {
                warn!(
                    "Failed to load state from file {:?}, starting with empty state!",
                    json_file
                );

                ExplorerApiState {
                    country_node_distribution: ConcurrentCountryNodesDistribution::new(),
                    mix_nodes: ThreadsafeMixNodesResult::new(),
                    mix_node_cache: ThreadsafeMixNodeCache::new(),
                    ping_cache: ThreadsafePingCache::new(),
                }
            }
        }
    }

    pub(crate) async fn write_to_file(&self) {
        let json_file = get_state_file_path().to_string();
        let json_file_path = Path::new(&json_file);
        let file = File::create(json_file_path).expect("unable to create state json file");
        let state = ExplorerApiStateOnDisk {
            country_node_distribution: self.inner.country_node_distribution.get_all().await,
            location_cache: self.inner.mix_nodes.get_location_cache().await,
            as_at: Utc::now(),
        };
        serde_json::to_writer(file, &state).expect("error writing state to disk");
        info!("Saved file to '{:?}'", json_file_path.canonicalize());
    }
}

fn get_state_file_path() -> String {
    std::env::var("API_STATE_FILE").unwrap_or_else(|_| STATE_FILE.to_string())
}
