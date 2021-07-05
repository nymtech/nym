use anyhow::Result;
use mixnet_contract::{GatewayBond, MixNodeBond};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use validator_client::Client;

pub struct ValidatorCache {
    mixnodes: RwLock<Cache<Vec<MixNodeBond>>>,
    gateways: RwLock<Cache<Vec<GatewayBond>>>,
    validator_client: Client,
}

#[derive(Default, Serialize, Clone)]
pub struct Cache<T> {
    value: T,
    #[allow(dead_code)]
    as_at: u64,
}

impl<T: Clone> Cache<T> {
    fn set(&mut self, value: T) {
        self.value = value;
        self.as_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[allow(dead_code)]
    pub fn get(&self) -> T {
        self.value.clone()
    }
}

impl ValidatorCache {
    pub fn init(validators_rest_uris: Vec<String>, mixnet_contract: String) -> Self {
        let config = validator_client::Config::new(validators_rest_uris, mixnet_contract);
        let validator_client = validator_client::Client::new(config);
        ValidatorCache {
            mixnodes: RwLock::new(Cache::default()),
            gateways: RwLock::new(Cache::default()),
            validator_client,
        }
    }

    pub async fn refresh_cache(&self) -> Result<()> {
        let (mixnodes, gateways) = tokio::join!(
            self.validator_client.get_mix_nodes(),
            self.validator_client.get_gateways()
        );
        self.mixnodes.write().await.set(mixnodes?);
        self.gateways.write().await.set(gateways?);

        Ok(())
    }

    pub async fn mixnodes(&self) -> Cache<Vec<MixNodeBond>> {
        self.mixnodes.read().await.clone()
    }

    pub async fn gateways(&self) -> Cache<Vec<GatewayBond>> {
        self.gateways.read().await.clone()
    }
}
