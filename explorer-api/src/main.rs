#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_okapi;

use clap::Parser;
use dotenv::dotenv;
use log::info;
use logging::setup_logging;
use network_defaults::setup_env;
use task::TaskManager;

mod buy_terms;
pub(crate) mod cache;
mod client;
pub(crate) mod commands;
mod country_statistics;
mod gateways;
mod geo_ip;
mod guards;
mod helpers;
mod http;
mod mix_node;
pub(crate) mod mix_nodes;
mod overview;
mod ping;
mod state;
mod tasks;
mod validators;

const COUNTRY_DATA_REFRESH_INTERVAL: u64 = 60 * 15; // every 15 minutes

#[tokio::main]
async fn main() {
    dotenv().ok();
    setup_logging();
    let args = commands::Cli::parse();
    setup_env(args.config_env_file.as_ref());
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
        wait_for_signal().await;

        log::info!("Sending shutdown");
        shutdown.signal_shutdown().ok();
        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        shutdown.wait_for_shutdown().await;
        log::info!("Stopping explorer API");
    }
}

#[cfg(unix)]
async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM channel");
    let mut sigquit = signal(SignalKind::quit()).expect("Failed to setup SIGQUIT channel");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received SIGINT");
        },
        _ = sigterm.recv() => {
            log::info!("Received SIGTERM");
        }
        _ = sigquit.recv() => {
            log::info!("Received SIGQUIT");
        }
    }
}

#[cfg(not(unix))]
async fn wait_for_signal() {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received SIGINT");
        },
    }
}
