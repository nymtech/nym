use log::info;
use task::ShutdownListener;

use crate::country_statistics::country_nodes_distribution::CountryNodesDistribution;
use crate::COUNTRY_DATA_REFRESH_INTERVAL;

use crate::state::ExplorerApiStateContext;

pub(crate) struct CountryStatisticsDistributionTask {
    state: ExplorerApiStateContext,
    shutdown: ShutdownListener,
}

impl CountryStatisticsDistributionTask {
    pub(crate) fn new(state: ExplorerApiStateContext, shutdown: ShutdownListener) -> Self {
        CountryStatisticsDistributionTask { state, shutdown }
    }

    pub(crate) fn start(mut self) {
        info!("Spawning mix node country distribution task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(std::time::Duration::from_secs(
                COUNTRY_DATA_REFRESH_INTERVAL,
            ));
            while !self.shutdown.is_shutdown() {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        self.calculate_nodes_per_country().await;
                    }
                    _ = self.shutdown.recv() => {
                        trace!("Listener: Received shutdown");
                    }
                }
            }
        });
    }

    /// Retrieves the current list of mixnodes from the validators and calculates how many nodes are in each country
    async fn calculate_nodes_per_country(&mut self) {
        let cache = self.state.inner.mixnodes.get_locations().await;

        let three_letter_iso_country_codes: Vec<String> = cache
            .values()
            .flat_map(|i| {
                i.location
                    .as_ref()
                    .map(|j| j.three_letter_iso_country_code.clone())
            })
            .collect();

        let mut distribution = CountryNodesDistribution::new();

        info!("Calculating country distribution from located mixnodes...");
        for three_letter_iso_country_code in three_letter_iso_country_codes {
            *(distribution.entry(three_letter_iso_country_code)).or_insert(0) += 1;
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
