use log::{info, warn};
use reqwest::Error as ReqwestError;
use thiserror::Error;

use crate::mix_nodes::{GeoLocation, Location};
use crate::state::ExplorerApiStateContext;

pub(crate) struct GeoLocateTask {
    state: ExplorerApiStateContext,
}

impl GeoLocateTask {
    pub(crate) fn new(state: ExplorerApiStateContext) -> Self {
        GeoLocateTask { state }
    }

    pub(crate) fn start(mut self) {
        info!("Spawning mix node locator task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(std::time::Duration::from_millis(50));
            loop {
                // wait for the next interval tick
                interval_timer.tick().await;
                self.locate_mix_nodes().await;
            }
        });
    }

    async fn locate_mix_nodes(&mut self) {
        let mixnode_bonds = self.state.inner.mix_nodes.get().await.value;

        for (i, cache_item) in mixnode_bonds.values().enumerate() {
            if self
                .state
                .inner
                .mix_nodes
                .is_location_valid(&cache_item.mix_node.identity_key)
                .await
            {
                // when the cached location is valid, don't locate and continue to next mix node
                continue;
            }

            // the mix node has not been located or is the cache time has expired
            match locate(&cache_item.mix_node.host).await {
                Ok(geo_location) => {
                    let location = Location::new(geo_location);

                    trace!(
                        "{} mix nodes already located. Ip {} is located in {:#?}",
                        i,
                        cache_item.mix_node.host,
                        location.three_letter_iso_country_code,
                    );

                    if i > 0 && (i % 100) == 0 {
                        info!(
                            "Located {} mixnodes...",
                            i + 1,
                        );
                    }

                    self.state
                        .inner
                        .mix_nodes
                        .set_location(&cache_item.mix_node.identity_key, Some(location))
                        .await;

                    // one node has been located, so return out of the loop
                    return;
                }
                Err(e) => match e {
                    LocateError::ReqwestError(e) => warn!(
                        "❌ Oh no! Location for {} failed {}",
                        cache_item.mix_node.host, e
                    ),
                    LocateError::NotFound(e) => {
                            warn!(
                            "❌ Location for {} not found. Response body: {}",
                            cache_item.mix_node.host, e
                        );
                        self.state
                            .inner
                            .mix_nodes
                            .set_location(&cache_item.mix_node.identity_key, None)
                            .await;
                    },
                    LocateError::RateLimited(e) => warn!(
                        "❌ Oh no, we've been rate limited! Location for {} failed. Response body: {}",
                        cache_item.mix_node.host, e
                    ),
                },
            }
        }

        trace!("All mix nodes located");
    }
}

#[derive(Debug, Error)]
enum LocateError {
    #[error("Oops, we have made too many requests and are being rate limited. Request body: {0}")]
    RateLimited(String),

    #[error("Geolocation not found. Request body: {0}")]
    NotFound(String),

    #[error(transparent)]
    ReqwestError(#[from] ReqwestError),
}

async fn locate(ip: &str) -> Result<GeoLocation, LocateError> {
    let api_key = ::std::env::var("GEO_IP_SERVICE_API_KEY")
        .expect("Env var GEO_IP_SERVICE_API_KEY is not set");
    let uri = format!("{}/{}?apikey={}", crate::GEO_IP_SERVICE, ip, api_key);
    match reqwest::get(uri.clone()).await {
        Ok(response) => {
            if response.status() == 429 {
                return Err(LocateError::RateLimited(
                    response
                        .text()
                        .await
                        .unwrap_or_else(|_| "(the response body is empty)".to_string()),
                ));
            }
            if response.status() == 404 {
                return Err(LocateError::NotFound(
                    response
                        .text()
                        .await
                        .unwrap_or_else(|_| "(the response body is empty)".to_string()),
                ));
            }
            let location = response.json::<GeoLocation>().await?;
            Ok(location)
        }
        Err(e) => Err(LocateError::ReqwestError(e)),
    }
}
