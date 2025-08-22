// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use nym_task::ShutdownToken;

use celes::Country;
use nym_validator_client::models::NymNodeDescription;
use std::collections::HashMap;
use std::time::Duration;
use std::{net::IpAddr, sync::Arc};
use tokio::sync::RwLock;
use tokio::time::interval;
use url::Url;

use nym_http_api_client::Client;
use nym_validator_client::client::NymApiClientExt;
use tracing::{error, info, trace, warn};

const NETWORK_CACHE_TTL: Duration = Duration::from_secs(600);

type IpToCountryMap = HashMap<IpAddr, Option<Country>>;

// SW this should use a proper NS API client once it exists
struct NodesQuerier {
    client: Client,
}

impl NodesQuerier {
    async fn current_nymnodes(&self) -> Result<Vec<NymNodeDescription>> {
        Ok(self
            .client
            .get_all_described_nodes()
            .await
            .inspect_err(|err| error!("failed to get network nodes: {err}"))?)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NetworkView {
    inner: Arc<RwLock<NetworkViewInner>>,
}

impl NetworkView {
    fn new_empty() -> Self {
        NetworkView {
            inner: Arc::new(RwLock::new(NetworkViewInner {
                network_nodes: HashMap::new(),
            })),
        }
    }

    pub(crate) async fn get_country_by_ip(&self, ip_addr: &IpAddr) -> Option<Option<Country>> {
        self.inner.read().await.network_nodes.get(ip_addr).copied()
    }
}

#[derive(Debug)]
struct NetworkViewInner {
    network_nodes: IpToCountryMap,
}

pub struct NetworkRefresher {
    querier: Option<NodesQuerier>,
    full_refresh_interval: Duration,
    shutdown_token: ShutdownToken,

    network: NetworkView,
}

impl NetworkRefresher {
    pub(crate) async fn initialise_new(
        maybe_nym_api_url: Option<Url>,
        shutdown_token: ShutdownToken,
    ) -> Self {
        let node_querier = match maybe_nym_api_url {
            Some(url) => match Self::build_http_api_client(url) {
                Ok(client) => Some(NodesQuerier { client }),
                Err(e) => {
                    warn!("Failed to build Nym API client, no network view will be availabe : {e}");
                    None
                }
            },
            None => {
                warn!("No Nym API specified, network view is unavailable");
                None
            }
        };

        let mut this = NetworkRefresher {
            querier: node_querier,
            full_refresh_interval: NETWORK_CACHE_TTL,
            shutdown_token,
            network: NetworkView::new_empty(),
        };

        if let Err(e) = this.refresh_network_nodes().await {
            warn!("Failed to fetch initial network nodes : {e}");
        }
        this
    }

    fn build_http_api_client(url: Url) -> Result<Client> {
        Ok(Client::builder::<_, anyhow::Error>(url)?
            .no_hickory_dns()
            .with_user_agent("node-statistics-api")
            .build::<anyhow::Error>()?)
    }

    async fn refresh_network_nodes(&mut self) -> Result<()> {
        if let Some(querier) = &self.querier {
            let nodes = querier.current_nymnodes().await?;

            // collect all known/allowed nodes information
            let known_nodes = nodes
                .iter()
                .flat_map(|n| {
                    n.description
                        .host_information
                        .ip_address
                        .clone()
                        .into_iter()
                        .zip(std::iter::repeat(n.description.auxiliary_details.location))
                })
                .collect::<HashMap<_, _>>();

            let mut network_guard = self.network.inner.write().await;
            network_guard.network_nodes = known_nodes;
        }

        Ok(())
    }

    pub(crate) fn network_view(&self) -> NetworkView {
        self.network.clone()
    }

    pub(crate) async fn run(&mut self) {
        info!("NetworkRefresher started successfully");
        let mut full_refresh_interval = interval(self.full_refresh_interval);
        full_refresh_interval.reset();

        while !self.shutdown_token.is_cancelled() {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                   trace!("NetworkRefresher: Received shutdown");
                }
                _ = full_refresh_interval.tick() => {
                    if self.refresh_network_nodes().await.is_err() {
                        warn!("Failed to refresh network nodes, we're gonna keep the same set");
                    }
                }
            }
        }
        trace!("NetworkRefresher: Exiting");
    }

    pub(crate) fn start(mut self) {
        tokio::spawn(async move { self.run().await });
    }
}
