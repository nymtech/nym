use std::sync::Arc;
use std::time::{Duration, SystemTime};

use rocket::tokio::sync::RwLock;

use mixnet_contract::MixNodeBond;
use validator_client::Config;

#[derive(Clone, Debug)]
pub(crate) struct MixNodesResult {
    pub(crate) valid_until: SystemTime,
    pub(crate) value: Vec<MixNodeBond>,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeMixNodesResult {
    inner: Arc<RwLock<MixNodesResult>>,
}

impl ThreadsafeMixNodesResult {
    pub(crate) fn new() -> Self {
        ThreadsafeMixNodesResult {
            inner: Arc::new(RwLock::new(MixNodesResult {
                value: vec![],
                valid_until: SystemTime::now() - Duration::from_secs(60), // in the past
            })),
        }
    }

    pub(crate) async fn get(&self) -> MixNodesResult {
        // check ttl
        let valid_until = self.inner.clone().read().await.valid_until;

        if valid_until < SystemTime::now() {
            // force reload
            self.refresh().await;
        }

        // return in-memory cache
        self.inner.clone().read().await.clone()
    }

    pub(crate) async fn refresh_and_get(&self) -> MixNodesResult {
        self.refresh().await;
        self.inner.read().await.clone()
    }

    async fn refresh(&self) {
        // get mixnodes and cache the new value
        let value = retrieve_mixnodes().await;
        self.inner.write().await.clone_from(&MixNodesResult {
            value,
            valid_until: SystemTime::now() + Duration::from_secs(60 * 10), // valid for 10 minutes
        });
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
