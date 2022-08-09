#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_okapi;

use clap::Parser;
use log::info;
use network_defaults::setup_env;
use task::ShutdownNotifier;

pub(crate) mod cache;
mod client;
pub(crate) mod commands;
mod country_statistics;
mod gateways;
mod helpers;
mod http;
mod mix_node;
pub(crate) mod mix_nodes;
mod overview;
mod ping;
mod state;
mod tasks;
mod validators;

const GEO_IP_SERVICE: &str = "https://api.ipbase.com/json";
const COUNTRY_DATA_REFRESH_INTERVAL: u64 = 60 * 15; // every 15 minutes

#[tokio::main]
async fn main() {
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

        let validator_api_url = self.state.inner.validator_client.api_endpoint();
        info!("Using validator API - {}", validator_api_url);

        let shutdown = ShutdownNotifier::default();

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

    async fn wait_for_interrupt(&self, mut shutdown: ShutdownNotifier) {
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

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .init();
}
