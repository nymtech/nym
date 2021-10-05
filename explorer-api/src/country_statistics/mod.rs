use log::{info, trace, warn};
use reqwest::Error as ReqwestError;

use crate::country_statistics::country_nodes_distribution::CountryNodesDistribution;
use crate::mix_nodes::{GeoLocation, Location};
use crate::state::ExplorerApiStateContext;

pub mod country_nodes_distribution;
pub mod http;

pub(crate) struct CountryStatistics {
    state: ExplorerApiStateContext,
}

impl CountryStatistics {
    pub(crate) fn new(state: ExplorerApiStateContext) -> Self {
        CountryStatistics { state }
    }

    pub(crate) fn start(mut self) {
        info!("Spawning task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
            loop {
                // wait for the next interval tick
                interval_timer.tick().await;

                info!("Running task...");
                self.calculate_nodes_per_country().await;
                info!("Done");
            }
        });
    }

    /// Retrieves the current list of mixnodes from the validators and calculates how many nodes are in each country
    async fn calculate_nodes_per_country(&mut self) {
        // force the mixnode cache to invalidate
        let mixnode_bonds = self.state.inner.mix_nodes.refresh_and_get().await.value;

        let mut distribution = CountryNodesDistribution::new();

        info!("Locating mixnodes...");
        for (i, cache_item) in mixnode_bonds.values().enumerate() {
            match locate(&cache_item.bond.mix_node.host).await {
                Ok(geo_location) => {
                    let location = Location::new(geo_location);

                    *(distribution.entry(location.three_letter_iso_country_code.to_string()))
                        .or_insert(0) += 1;

                    trace!(
                        "Ip {} is located in {:#?}",
                        cache_item.bond.mix_node.host,
                        location.three_letter_iso_country_code,
                    );

                    self.state
                        .inner
                        .mix_nodes
                        .set_location(&cache_item.bond.mix_node.identity_key, location)
                        .await;

                    if (i % 100) == 0 {
                        info!(
                            "Located {} mixnodes in {} countries",
                            i + 1,
                            distribution.len()
                        );
                    }
                }
                Err(e) => warn!("❌ Oh no! Location failed {}", e),
            }
        }

        // replace the shared distribution to be the new distribution
        self.state
            .inner
            .country_node_distribution
            .set_all(distribution)
            .await;

        info!(
            "Locating mixnodes done: {:?}",
            self.state.inner.country_node_distribution.get_all().await
        );

        // keep state on disk, so that when this process dies it can start up again and users get some data
        self.state.write_to_file().await;
    }
}

async fn locate(ip: &str) -> Result<GeoLocation, ReqwestError> {
    let api_key = ::std::env::var("GEO_IP_SERVICE_API_KEY")
        .expect("Env var GEO_IP_SERVICE_API_KEY is not set");
    let response = reqwest::get(format!(
        "{}{}?apikey={}",
        crate::GEO_IP_SERVICE,
        ip,
        api_key
    ))
    .await?;
    let location = response.json::<GeoLocation>().await?;
    Ok(location)
}
