#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_okapi;

use clap::Parser;
use dotenvy::dotenv;
use log::info;
use nym_bin_common::logging::setup_logging;
use nym_network_defaults::setup_env;
use nym_task::TaskManager;

pub(crate) mod cache;
mod client;
pub(crate) mod commands;
mod country_statistics;
mod gateways;
mod geo_ip;
mod guards;
mod helpers;
mod http;
mod location;
mod mix_node;
pub(crate) mod mix_nodes;
mod nym_nodes;
mod overview;
mod ping;
pub(crate) mod service_providers;
mod state;
mod tasks;
mod validators;

const COUNTRY_DATA_REFRESH_INTERVAL: u64 = 60 * 15; // every 15 minutes

#[tokio::main]
async fn main() {
    dotenv().ok();
    setup_logging();
    let args = commands::Cli::parse();
    setup_env(args.config_env_file);
    let mut explorer_api = ExplorerApi::new();
    explorer_api.run().await;
}

pub struct ExplorerApi {
    state: state::ExplorerApiStateContext,
}

impl ExplorerApi {
    fn new() -> ExplorerApi {
        ExplorerApi {
            state: state::ExplorerApiStateContext::new(),
        }
    }

    async fn run(&mut self) {
        info!("Explorer API starting up...");

        let nym_api_url = self.state.inner.validator_client.api_endpoint();
        info!("Using validator API - {}", nym_api_url);

        let shutdown = TaskManager::default();

        // spawn concurrent tasks
        crate::tasks::ExplorerApiTasks::new(self.state.clone(), shutdown.subscribe()).start();
        country_statistics::distribution::CountryStatisticsDistributionTask::new(
            self.state.clone(),
            shutdown.subscribe(),
        )
        .start();
        country_statistics::geolocate::GeoLocateTask::new(self.state.clone(), shutdown.subscribe())
            .start();

        // Rocket handles shutdown on it's own, but its shutdown handling should be incorporated
        // with that of the rest of the tasks.
        // Currently it's runtime is forcefully terminated once the explorer-api exits.
        http::start(self.state.clone());

        // wait for user to press ctrl+C
        self.wait_for_interrupt(shutdown).await
    }

    async fn wait_for_interrupt(&self, mut shutdown: TaskManager) {
        let _res = shutdown.catch_interrupt().await;
        log::info!("Stopping explorer API");
    }
}
