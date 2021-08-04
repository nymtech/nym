use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

use log::info;

use crate::country_statistics::country_nodes_distribution::{
    ConcurrentCountryNodesDistribution, CountryNodesDistribution,
};

// TODO: change to an environment variable with a default value
const STATE_FILE: &str = "explorer-api-state.json";

#[derive(Clone)]
pub struct ExplorerApiState {
    pub(crate) country_node_distribution: ConcurrentCountryNodesDistribution,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplorerApiStateOnDisk {
    pub(crate) country_node_distribution: CountryNodesDistribution,
    pub(crate) as_at: DateTime<Utc>,
}

#[derive(Clone)]
pub(crate) struct ExplorerApiStateContext {
    pub(crate) inner: ExplorerApiState,
    state_file: String,
}

impl ExplorerApiStateContext {
    pub(crate) fn new() -> Self {
        ExplorerApiStateContext {
            inner: ExplorerApiStateContext::read_from_file(),
            state_file: std::env::var("API_STATE_FILE").unwrap_or_else(|_| STATE_FILE.to_string()),
        }
    }

    pub(crate) fn read_from_file() -> ExplorerApiState {
        let json_file = get_state_file_path();
        let json_file_path = Path::new(&json_file);
        info!("Loading state from file {:?}...", json_file);
        match File::open(json_file_path) {
            Ok(file) => {
                let state: ExplorerApiStateOnDisk =
                    serde_json::from_reader(file).expect("error while reading json");
                info!("Loaded state from file {:?}: {:?}", json_file, state);
                ExplorerApiState {
                    country_node_distribution: ConcurrentCountryNodesDistribution::attach(
                        state.country_node_distribution,
                    ),
                }
            }
            Err(_e) => {
                warn!(
                    "Failed to load state from file {:?}, starting with empty state!",
                    json_file
                );
                ExplorerApiState {
                    country_node_distribution: ConcurrentCountryNodesDistribution::new(),
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
            as_at: Utc::now(),
        };
        serde_json::to_writer(file, &state).expect("error writing state to disk");
        info!("Saved file to '{:?}'", json_file_path.canonicalize());
    }
}

fn get_state_file_path() -> String {
    std::env::var("API_STATE_FILE").unwrap_or_else(|_| STATE_FILE.to_string())
}
