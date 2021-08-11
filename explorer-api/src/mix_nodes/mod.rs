mod utils;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use rocket::tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::mix_nodes::utils::map_2_letter_to_3_letter_country_code;
use mixnet_contract::MixNodeBond;
use validator_client::Config;

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

#[derive(Clone, Debug)]
pub(crate) struct MixNodeBondWithLocation {
    pub(crate) location: Option<Location>,
    pub(crate) bond: MixNodeBond,
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

    pub(crate) async fn get_location_cache(&self) -> LocationCache {
        self.inner.read().await.location_cache.clone()
    }

    pub(crate) async fn set_location(&self, identity_key: &str, location: Location) {
        let mut guard = self.inner.write().await;

        // cache the location for this mix node so that it can be used when the mix node list is refreshed
        guard
            .location_cache
            .insert(identity_key.to_string(), location.clone());

        // add the location to the mix node
        guard
            .value
            .entry(identity_key.to_string())
            .and_modify(|item| item.location = Some(location));
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

    pub(crate) async fn refresh_and_get(&self) -> MixNodesResult {
        self.refresh().await;
        self.inner.read().await.clone()
    }

    async fn refresh(&self) {
        // get mixnodes and cache the new value
        let value = retrieve_mixnodes().await;
        let location_cache = self.inner.read().await.location_cache.clone();
        *self.inner.write().await = MixNodesResult {
            value: value
                .into_iter()
                .map(|bond| {
                    let location = location_cache.get(&bond.mix_node.identity_key).cloned(); // add the location, if we've located this mix node before
                    (
                        bond.mix_node.identity_key.to_string(),
                        MixNodeBondWithLocation { bond, location },
                    )
                })
                .collect(),
            valid_until: SystemTime::now() + Duration::from_secs(60 * 10), // valid for 10 minutes
            location_cache,
        };
    }
}

pub(crate) async fn retrieve_mixnodes() -> Vec<MixNodeBond> {
    let client = new_validator_client();

    info!("About to retrieve mixnode bonds...");

    let bonds: Vec<MixNodeBond> = match client.get_cached_mix_nodes().await {
        Ok(result) => result,
        Err(e) => {
            error!("Unable to retrieve mixnode bonds: {:?}", e);
            vec![]
        }
    };
    info!("Fetched {} mixnode bonds", bonds.len());
    bonds
}

// TODO: inject constants
fn new_validator_client() -> validator_client::Client {
    let config = Config::new(vec![crate::VALIDATOR_API.to_string()], crate::CONTRACT);
    validator_client::Client::new(config)
}
