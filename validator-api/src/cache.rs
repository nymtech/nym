use anyhow::Result;
use mixnet_contract::MixNodeBond;
use std::time::Instant;
use validator_client::Client;

pub struct MixNodeCache {
    value: Vec<MixNodeBond>,
    as_at: Instant,
    validator_client: Client,
}
impl MixNodeCache {
    pub fn init(
        value: Vec<MixNodeBond>,
        validators_rest_uris: Vec<String>,
        mixnet_contract: String,
    ) -> Self {
        let config = validator_client::Config::new(validators_rest_uris, mixnet_contract);
        let validator_client = validator_client::Client::new(config);
        MixNodeCache {
            value,
            as_at: Instant::now(),
            validator_client,
        }
    }

    fn set_value(&mut self, value: Vec<MixNodeBond>) {
        self.value = value;
        self.as_at = Instant::now()
    }

    pub fn value(&self) -> Vec<MixNodeBond> {
        self.value.clone()
    }

    pub async fn cache(&mut self) -> Result<()> {
        let mixnodes = self.validator_client.get_mix_nodes().await?;
        self.set_value(mixnodes);
        Ok(())
    }
}
