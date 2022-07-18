#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_okapi;

use clap::Parser;
use log::info;
use network_defaults::setup_env;

pub(crate) mod cache;
mod client;
pub(crate) mod commands;
mod country_statistics;
mod gateways;
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

        // spawn concurrent tasks
        crate::tasks::ExplorerApiTasks::new(self.state.clone()).start();
        country_statistics::distribution::CountryStatisticsDistributionTask::new(
            self.state.clone(),
        )
        .start();
        country_statistics::geolocate::GeoLocateTask::new(self.state.clone()).start();
        http::start(self.state.clone());

        // wait for user to press ctrl+C
        self.wait_for_interrupt().await
    }

    async fn wait_for_interrupt(&self) {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }
        info!(
            "Received SIGINT - the mixnode will terminate now (threads are not yet nicely stopped, if you see stack traces that's alright)."
        );
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
