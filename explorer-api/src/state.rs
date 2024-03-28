use std::fs::File;
use std::path::Path;

use chrono::{DateTime, Utc};
use log::info;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};

use crate::client::ThreadsafeValidatorClient;
use crate::geo_ip::location::ThreadsafeGeoIp;
use nym_validator_client::models::MixNodeBondAnnotated;

use crate::country_statistics::country_nodes_distribution::{
    CountryNodesDistribution, ThreadsafeCountryNodesDistribution,
};
use crate::gateways::location::GatewayLocationCache;
use crate::gateways::models::ThreadsafeGatewayCache;
use crate::mix_node::models::ThreadsafeMixNodeCache;
use crate::mix_nodes::location::MixnodeLocationCache;
use crate::mix_nodes::models::ThreadsafeMixNodesCache;
use crate::ping::models::ThreadsafePingCache;
use crate::validators::models::ThreadsafeValidatorCache;

// TODO: change to an environment variable with a default value
const STATE_FILE: &str = "explorer-api-state.json";

#[derive(Clone)]
pub struct ExplorerApiState {
    pub(crate) country_node_distribution: ThreadsafeCountryNodesDistribution,
    pub(crate) gateways: ThreadsafeGatewayCache,
    pub(crate) mixnode: ThreadsafeMixNodeCache,
    pub(crate) mixnodes: ThreadsafeMixNodesCache,
    pub(crate) ping: ThreadsafePingCache,
    pub(crate) validators: ThreadsafeValidatorCache,
    pub(crate) geo_ip: ThreadsafeGeoIp,

    // TODO: discuss with @MS whether this is an appropriate spot for it
    pub(crate) validator_client: ThreadsafeValidatorClient,
}

impl ExplorerApiState {
    pub(crate) async fn get_mix_node(&self, mix_id: NodeId) -> Option<MixNodeBondAnnotated> {
        self.mixnodes.get_mixnode(mix_id).await
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplorerApiStateOnDisk {
    pub(crate) country_node_distribution: CountryNodesDistribution,
    pub(crate) mixnode_location_cache: MixnodeLocationCache,
    pub(crate) gateway_location_cache: GatewayLocationCache,
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

        if let Ok(Ok(state)) =
            File::open(json_file_path).map(serde_json::from_reader::<_, ExplorerApiStateOnDisk>)
        {
            info!("Loaded state from file {:?}: {:?}", json_file, state);
            ExplorerApiState {
                country_node_distribution:
                    ThreadsafeCountryNodesDistribution::new_from_distribution(
                        state.country_node_distribution,
                    ),
                gateways: ThreadsafeGatewayCache::new_with_location_cache(
                    state.gateway_location_cache,
                ),
                mixnode: ThreadsafeMixNodeCache::new(),
                mixnodes: ThreadsafeMixNodesCache::new_with_location_cache(
                    state.mixnode_location_cache,
                ),
                ping: ThreadsafePingCache::new(),
                validators: ThreadsafeValidatorCache::new(),
                validator_client: ThreadsafeValidatorClient::new(),
                geo_ip: ThreadsafeGeoIp::new(),
            }
        } else {
            warn!(
                "Failed to load state from file {:?}, starting with empty state!",
                json_file
            );

            ExplorerApiState {
                country_node_distribution: ThreadsafeCountryNodesDistribution::new(),
                gateways: ThreadsafeGatewayCache::new(),
                mixnode: ThreadsafeMixNodeCache::new(),
                mixnodes: ThreadsafeMixNodesCache::new(),
                ping: ThreadsafePingCache::new(),
                validators: ThreadsafeValidatorCache::new(),
                validator_client: ThreadsafeValidatorClient::new(),
                geo_ip: ThreadsafeGeoIp::new(),
            }
        }
    }

    pub(crate) async fn write_to_file(&self) {
        let json_file = get_state_file_path().to_string();
        let json_file_path = Path::new(&json_file);
        let file = File::create(json_file_path).expect("unable to create state json file");
        let state = ExplorerApiStateOnDisk {
            country_node_distribution: self.inner.country_node_distribution.get_all().await,
            mixnode_location_cache: self.inner.mixnodes.get_locations().await,
            gateway_location_cache: self.inner.gateways.get_locations().await,
            as_at: Utc::now(),
        };
        serde_json::to_writer(file, &state).expect("error writing state to disk");
        info!("Saved file to '{:?}'", json_file_path.canonicalize());
    }
}

fn get_state_file_path() -> String {
    std::env::var("API_STATE_FILE").unwrap_or_else(|_| STATE_FILE.to_string())
}
