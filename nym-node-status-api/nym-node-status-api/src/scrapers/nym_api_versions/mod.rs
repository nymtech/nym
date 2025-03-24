use crate::db::DbPool;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::NyxdClient;
use reqwest::Url;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::time::Duration;
use tracing::{debug, error, instrument};
use validator_status_check::{helpers::get_known_dealers, models::SignerStatus};

// TODO dz configurable?
const SCRAPE_INTERVAL: Duration = Duration::from_secs(60 * 60);
const COSMOS_REST_API: &str = "https://api.nymtech.net/cosmwasm/wasm/v1/contract/n19604yflqggs9mk2z26mqygq43q2kr3n932egxx630svywd5mpxjsztfpvx/smart/eyJnZXRfY3VycmVudF9kZWFsZXJzIjogeyJsaW1pdCI6IDMwfX0=";
const BUILD_INFORMATION_API: &str = "/v1/api-status/build-information";

pub struct Scraper {
    pool: SqlitePool,
    http_client: reqwest::Client,
}

#[instrument(level = "debug", name = "nym_api_versions", skip_all)]
pub(crate) async fn spawn(pool: DbPool) {
    tracing::info!("Starting Nym API scraper");

    let pool_cloned = pool.clone();
    let scraper = Scraper::new(pool_cloned);

    tokio::spawn(async move {
        loop {
            if let Err(e) = scraper.run().await {
                error!(name: "nym_api_scraper", "Failed: {}", e);
            }
            debug!(name: "nym_api_scraper", "Sleeping for {}s", SCRAPE_INTERVAL.as_secs());
            tokio::time::sleep(SCRAPE_INTERVAL).await;
        }
    });
}

impl Scraper {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            http_client: reqwest::Client::new(),
        }
    }

    async fn run(&self) -> anyhow::Result<()> {
        let dealers = get_known_dealers().await?;
        let mut signer_statuses = Vec::new();
        for dealer in dealers {
            let mut status = SignerStatus::new(dealer.announce_address);
            status.try_update_api_version().await;
            status.try_update_rpc_status().await;

            signer_statuses.push(status);
        }

        Ok(())
    }

    async fn get_build_info_from_all_signers(
        &self,
    ) -> anyhow::Result<Vec<cosmos_response::Status>> {
        let signer_address_list = {
            let response: cosmos_response::Response = self
                .http_client
                .get(COSMOS_REST_API)
                .send()
                .await
                .and_then(|res| res.error_for_status())?
                .json()
                .await?;
            response
                .data
                .dealers
                .into_iter()
                .map(|dealer| dealer.announce_address)
                .collect::<Vec<_>>()
        };

        let mut build_info = Vec::new();
        for signer_address in signer_address_list {
            let target_url = format!("{}/{}", signer_address, BUILD_INFORMATION_API);
            let signer_status = match self
                .http_client
                .get(target_url)
                .send()
                .await
                .and_then(|res| res.error_for_status())
            {
                Ok(response) => match response.json::<BuildInformation>().await {
                    Ok(build_info) => cosmos_response::Status::Ok(build_info),
                    Err(err) => cosmos_response::Status::Unreachable(err.to_string()),
                },
                Err(err) => cosmos_response::Status::Unreachable(err.to_string()),
            };
            build_info.push(signer_status);
        }

        Ok(build_info)
    }
}

// async fn get_build_info(client: &reqwest::Client, dealer_address: &str) -> anyhow::Result<String>

#[allow(dead_code)]
mod cosmos_response {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Response {
        pub data: Data,
    }

    #[derive(Debug, Deserialize)]
    pub struct Data {
        pub dealers: Vec<Dealer>,
        pub per_page: u64,
        pub start_next_after: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Dealer {
        pub address: String,
        pub bte_public_key_with_proof: String,
        pub ed25519_identity: String,
        pub announce_address: String,
        pub assigned_index: u64,
    }

    #[derive(Debug, Deserialize)]
    pub enum Status {
        Ok(super::BuildInformation),
        Unreachable(String),
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BuildInformation {
    binary_name: String,
    build_timestamp: String,
    build_version: String,
    commit_sha: String,
    commit_timestamp: String,
    commit_branch: String,
    rustc_version: String,
    rustc_channel: String,
    cargo_profile: String,
    cargo_triple: String,
}
