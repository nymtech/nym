use crate::state::ExplorerApiStateContext;
use mixnet_contract::MixNodeBond;
use reqwest::Url;

pub(crate) struct MixNodesTasks {
    state: ExplorerApiStateContext,
    validator_api_client: validator_client::ApiClient,
}

impl MixNodesTasks {
    pub(crate) fn new(state: ExplorerApiStateContext, validator_api_endpoint: Url) -> Self {
        MixNodesTasks {
            state,
            validator_api_client: validator_client::ApiClient::new(validator_api_endpoint),
        }
    }

    async fn retrieve_mixnodes(&self) -> Vec<MixNodeBond> {
        info!("About to retrieve mixnode bonds...");

        let bonds = match self.validator_api_client.get_cached_mixnodes().await {
            Ok(result) => result,
            Err(e) => {
                error!("Unable to retrieve mixnode bonds: {:?}", e);
                vec![]
            }
        };
        info!("Fetched {} mixnode bonds", bonds.len());
        bonds
    }

    async fn update_mixnode_cache(&self) {
        let bonds = self.retrieve_mixnodes().await;
        self.state.inner.mix_nodes.update_cache(bonds).await;
    }

    pub(crate) fn start(self) {
        info!("Spawning mix nodes task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                // wait for the next interval tick
                interval_timer.tick().await;

                info!("Updating mix node cache...");
                self.update_mixnode_cache().await;
                info!("Done");
            }
        });
    }
}
