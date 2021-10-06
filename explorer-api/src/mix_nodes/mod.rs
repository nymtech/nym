mod utils;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use rocket::tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::mix_node::http::PrettyMixNodeBondWithLocation;
use crate::mix_nodes::utils::map_2_letter_to_3_letter_country_code;
use mixnet_contract::{Delegation, MixNodeBond, RawDelegationData, UnpackedDelegation};
use network_defaults::{
    default_api_endpoints, default_nymd_endpoints, DEFAULT_MIXNET_CONTRACT_ADDRESS,
};
use validator_client::nymd::QueryNymdClient;

pub(crate) type LocationCache = HashMap<String, Location>;

#[derive(Debug, Deserialize)]
pub(crate) struct GeoLocation {
    pub(crate) ip: String,
    pub(crate) country_code: String,
    pub(crate) country_name: String,
    pub(crate) region_code: String,
    pub(crate) region_name: String,
    pub(crate) city: String,
    pub(crate) zip_code: String,
    pub(crate) time_zone: String,
    pub(crate) latitude: f32,
    pub(crate) longitude: f32,
    pub(crate) metro_code: u32,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
pub(crate) struct Location {
    pub(crate) two_letter_iso_country_code: String,
    pub(crate) three_letter_iso_country_code: String,
    pub(crate) country_name: String,
    pub(crate) lat: f32,
    pub(crate) lng: f32,
}

impl Location {
    pub(crate) fn new(geo_location: GeoLocation) -> Self {
        let three_letter_iso_country_code = map_2_letter_to_3_letter_country_code(&geo_location);
        Location {
            country_name: geo_location.country_name,
            two_letter_iso_country_code: geo_location.country_code,
            three_letter_iso_country_code,
            lat: geo_location.latitude,
            lng: geo_location.longitude,
        }
    }
}

}

#[derive(Clone, Debug)]
pub(crate) struct MixNodesResult {
    pub(crate) valid_until: SystemTime,
    pub(crate) value: HashMap<String, MixNodeBondWithLocation>,
    location_cache: LocationCache,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeMixNodesResult {
    inner: Arc<RwLock<MixNodesResult>>,
}

impl ThreadsafeMixNodesResult {
    pub(crate) fn new() -> Self {
        ThreadsafeMixNodesResult {
            inner: Arc::new(RwLock::new(MixNodesResult {
                value: HashMap::new(),
                valid_until: SystemTime::now() - Duration::from_secs(60), // in the past
                location_cache: LocationCache::new(),
            })),
        }
    }

    pub(crate) fn new_with_location_cache(location_cache: LocationCache) -> Self {
        ThreadsafeMixNodesResult {
            inner: Arc::new(RwLock::new(MixNodesResult {
                value: HashMap::new(),
                valid_until: SystemTime::now() - Duration::from_secs(60), // in the past
                location_cache,
            })),
        }
    }

    pub(crate) async fn is_location_valid(&self, identity_key: &str) -> bool {
        self.inner
            .read()
            .await
            .location_cache
            .get(identity_key)
            .map(|cache_item| cache_item.valid_until > SystemTime::now())
            .unwrap_or(false)
    }

    pub(crate) async fn get_location_cache(&self) -> LocationCache {
        self.inner.read().await.location_cache.clone()
    }

    pub(crate) async fn set_location(&self, identity_key: &str, location: Option<Location>) {
        let mut guard = self.inner.write().await;

        // cache the location for this mix node so that it can be used when the mix node list is refreshed
        guard.location_cache.insert(
            identity_key.to_string(),
            LocationCacheItem::new_from_location(location),
        );
    }

    pub(crate) async fn get(&self) -> MixNodesResult {
        // check ttl
        let valid_until = self.inner.read().await.valid_until;

        if valid_until < SystemTime::now() {
            // force reload
            self.refresh().await;
        }

        // return in-memory cache
        self.inner.read().await.clone()
    }

    pub(crate) async fn get_mixnodes_with_location(&self) -> Vec<PrettyMixNodeBondWithLocation> {
        let guard = self.inner.read().await;
        guard
            .value
            .values()
            .map(|bond| {
                let location = guard.location_cache.get(&bond.mix_node.identity_key);
                let copy = bond.clone();
                PrettyMixNodeBondWithLocation {
                    location: location.and_then(|l| l.location.clone()),
                    bond_amount: copy.bond_amount,
                    total_delegation: copy.total_delegation,
                    owner: copy.owner,
                    layer: copy.layer,
                    mix_node: copy.mix_node,
                }
            })
            .collect()
    }

    pub(crate) async fn refresh(&self) {
        // get mixnodes and cache the new value
        let value = retrieve_mixnodes().await;
        let location_cache = self.inner.read().await.location_cache.clone();
        *self.inner.write().await = MixNodesResult {
            value: value
                .into_iter()
                .map(|bond| (bond.mix_node.identity_key.to_string(), bond))
                .collect(),
            valid_until: SystemTime::now() + Duration::from_secs(60 * 10), // valid for 10 minutes
            location_cache,
        };
    }
}

pub(crate) async fn retrieve_mixnodes() -> Vec<MixNodeBond> {
    let client = new_validator_client();

    info!("About to retrieve mixnode bonds...");

    let bonds: Vec<MixNodeBond> = match client.get_cached_mixnodes().await {
        Ok(result) => result,
        Err(e) => {
            error!("Unable to retrieve mixnode bonds: {:?}", e);
            vec![]
        }
    };
    info!("Fetched {} mixnode bonds", bonds.len());
    bonds
}

pub(crate) async fn get_single_mixnode_delegations(pubkey: &str) -> Vec<Delegation> {
    let client = new_nymd_client();
    let delegates = match client
        .get_all_nymd_single_mixnode_delegations(pubkey.to_string())
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Could not get delegations for mix node {}: {:?}", pubkey, e);
            vec![]
        }
    };
    delegates
}

pub(crate) async fn get_mixnode_delegations() -> Vec<UnpackedDelegation<RawDelegationData>> {
    let client = new_nymd_client();
    let delegates = match client.get_all_nymd_mixnode_delegations().await {
        Ok(result) => result,
        Err(e) => {
            error!("Could not get all mix delegations: {:?}", e);
            vec![]
        }
    };
    delegates
}

fn new_nymd_client() -> validator_client::Client<QueryNymdClient> {
    let mixnet_contract = DEFAULT_MIXNET_CONTRACT_ADDRESS.to_string();
    let nymd_url = default_nymd_endpoints()[0].clone();
    let api_url = default_api_endpoints()[0].clone();

    let client_config =
        validator_client::Config::new(nymd_url, api_url, Some(mixnet_contract.parse().unwrap()));

    validator_client::Client::new_query(client_config).expect("Failed to connect to nymd!")
}

// TODO: inject constants
fn new_validator_client() -> validator_client::ApiClient {
    validator_client::ApiClient::new(default_api_endpoints()[0].clone())
}
