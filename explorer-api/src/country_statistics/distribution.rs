use log::info;

use crate::country_statistics::country_nodes_distribution::CountryNodesDistribution;
use crate::mix_nodes::Location;
use crate::state::ExplorerApiStateContext;

pub(crate) struct CountryStatisticsDistributionTask {
    state: ExplorerApiStateContext,
}

impl CountryStatisticsDistributionTask {
    pub(crate) fn new(state: ExplorerApiStateContext) -> Self {
        CountryStatisticsDistributionTask { state }
    }

    pub(crate) fn start(mut self) {
        info!("Spawning mix node country distribution task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(std::time::Duration::from_secs(60 * 15)); // every 15 mins
            loop {
                // wait for the next interval tick
                interval_timer.tick().await;
                self.calculate_nodes_per_country().await;
            }
        });
    }

    /// Retrieves the current list of mixnodes from the validators and calculates how many nodes are in each country
    async fn calculate_nodes_per_country(&mut self) {
        let cache = self.state.inner.mix_nodes.get_location_cache().await;

        let locations: Vec<&Location> = cache.values().flat_map(|i| i.location.as_ref()).collect();

        let mut distribution = CountryNodesDistribution::new();

        info!("Calculating country distribution from located mixnodes...");

        for location in locations {
            *(distribution.entry(location.three_letter_iso_country_code.clone())).or_insert(0) += 1;
        }

        // replace the shared distribution to be the new distribution
        self.state
            .inner
            .country_node_distribution
            .set_all(distribution)
            .await;

        info!(
            "Mixnode country distribution done: {:?}",
            self.state.inner.country_node_distribution.get_all().await
        );

        // keep state on disk, so that when this process dies it can start up again and users get some data
        self.state.write_to_file().await;
    }
}
