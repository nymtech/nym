use anyhow::Result;
use mixnet_contract::{GatewayBond, MixNodeBond};
use std::time::Instant;
use validator_client::Client;

pub struct ValidatorCache {
    mixnodes: Cache<Vec<MixNodeBond>>,
    gateways: Cache<Vec<GatewayBond>>,
    validator_client: Client,
}

#[derive(Default)]
struct Cache<T> {
    value: T,
    #[allow(dead_code)]
    as_at: Option<Instant>,
}

impl<T: Clone> Cache<T> {
    fn set(&mut self, value: T) {
        self.value = value;
        self.as_at = Some(Instant::now())
    }

    pub fn get(&self) -> T {
        self.value.clone()
    }
}

impl ValidatorCache {
    pub fn init(validators_rest_uris: Vec<String>, mixnet_contract: String) -> Self {
        let config = validator_client::Config::new(validators_rest_uris, mixnet_contract);
        let validator_client = validator_client::Client::new(config);
        ValidatorCache {
            mixnodes: Cache::default(),
            gateways: Cache::default(),
            validator_client,
        }
    }

    pub async fn cache(&mut self) -> Result<()> {
        let (mixnodes, gateways) = tokio::join!(
            self.validator_client.get_mix_nodes(),
            self.validator_client.get_gateways()
        );
        self.mixnodes.set(mixnodes?);
        self.gateways.set(gateways?);
        Ok(())
    }

    pub fn mixnodes(&self) -> Vec<MixNodeBond> {
        self.mixnodes.get()
    }

    pub fn gateways(&self) -> Vec<GatewayBond> {
        self.gateways.get()
    }
}
