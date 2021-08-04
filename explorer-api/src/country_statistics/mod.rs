use isocountry::CountryCode;
use log::{info, trace, warn};
use mixnet_contract::MixNodeBond;
use reqwest::Error as ReqwestError;
use validator_client::Config;

use models::GeoLocation;

use crate::country_statistics::country_nodes_distribution::CountryNodesDistribution;
use crate::state::ExplorerApiStateContext;

pub mod country_nodes_distribution;
pub mod http;
mod models;

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
        let mixnode_bonds = retrieve_mixnodes().await;

        let mut distribution = CountryNodesDistribution::new();

        info!("Locating mixnodes...");
        for (i, bond) in mixnode_bonds.iter().enumerate() {
            match locate(&bond.mix_node.host).await {
                Ok(location) => {
                    let country_code = map_2_letter_to_3_letter_country_code(&location);
                    *(distribution.entry(country_code)).or_insert(0) += 1;

                    trace!(
                        "Ip {} is located in {:#?}",
                        bond.mix_node.host,
                        map_2_letter_to_3_letter_country_code(&location)
                    );

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

fn map_2_letter_to_3_letter_country_code(geo: &GeoLocation) -> String {
    match CountryCode::for_alpha2(&geo.country_code) {
        Ok(three_letter_country_code) => three_letter_country_code.alpha3().to_string(),
        Err(_e) => {
            warn!(
                "❌ Oh no! map_2_letter_to_3_letter_country_code failed for '{:#?}'",
                geo
            );
            "???".to_string()
        }
    }
}

async fn locate(ip: &str) -> Result<GeoLocation, ReqwestError> {
    let response = reqwest::get(format!("{}{}", crate::GEO_IP_SERVICE, ip)).await?;
    let location = response.json::<GeoLocation>().await?;
    Ok(location)
}

async fn retrieve_mixnodes() -> Vec<MixNodeBond> {
    let client = new_validator_client();

    info!("About to retrieve mixnode bonds...");

    let bonds: Vec<MixNodeBond> = match client.get_cached_mix_nodes().await {
        Ok(result) => result,
        Err(e) => panic!("Unable to retrieve mixnode bonds: {:?}", e),
    };
    info!("Fetched {} mixnode bonds", bonds.len());
    bonds
}

// TODO: inject constants
fn new_validator_client() -> validator_client::Client {
    let config = Config::new(vec![crate::VALIDATOR_API.to_string()], crate::CONTRACT);
    validator_client::Client::new(config)
}
