// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use network_defaults::DEFAULT_NETWORK;
use nymsphinx::addressing::clients::{ClientIdentity, Recipient};
use socks5_requests::Response;

use super::error::StatsError;

const REMOTE_SOURCE_OF_STATS_PROVIDER_CONFIG: &str =
    "https://nymtech.net/.wellknown/network-requester/stats-provider.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatsMessage {
    pub description: String,
    pub stats_data: Vec<StatsClientData>,
    pub interval_seconds: u32,
    pub timestamp: String,
}

impl StatsMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, StatsError> {
        Ok(bincode::serialize(self)?)
    }

    #[cfg(feature = "stats-service")]
    pub fn from_bytes(b: &[u8]) -> Result<Self, StatsError> {
        Ok(bincode::deserialize(b)?)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatsClientData {
    pub client_identity: String,
    pub request_bytes: u32,
    pub response_bytes: u32,
}

impl StatsClientData {
    pub fn new(client_identity: String, request_bytes: u32, response_bytes: u32) -> Self {
        StatsClientData {
            client_identity,
            request_bytes,
            response_bytes,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StatsData {
    client_processed_bytes: HashMap<String, u32>,
}

impl StatsData {
    pub fn new() -> Self {
        StatsData {
            client_processed_bytes: HashMap::new(),
        }
    }

    pub fn processed(&mut self, client_identity: &ClientIdentity, bytes: u32) {
        let client_identity_bs58 = client_identity.to_base58_string();
        if let Some(curr_bytes) = self.client_processed_bytes.get_mut(&client_identity_bs58) {
            *curr_bytes += bytes;
        } else {
            self.client_processed_bytes
                .insert(client_identity_bs58, bytes);
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct StatsProviderConfigEntry {
    stats_client_address: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionalStatsProviderConfig {
    mainnet: Option<StatsProviderConfigEntry>,
    sandbox: Option<StatsProviderConfigEntry>,
    qa: Option<StatsProviderConfigEntry>,
}

impl OptionalStatsProviderConfig {
    pub fn stats_client_address(&self) -> Option<String> {
        let entry_config = match DEFAULT_NETWORK {
            network_defaults::all::Network::MAINNET => self.mainnet.clone(),
            network_defaults::all::Network::SANDBOX => self.sandbox.clone(),
            network_defaults::all::Network::QA => self.qa.clone(),
            network_defaults::all::Network::CUSTOM { .. } => {
                panic!("custom network is unsupported")
            }
        };
        entry_config.map(|e| e.stats_client_address)
    }
}

pub struct Statistics {
    description: String,
    request_data: Arc<RwLock<StatsData>>,
    response_data: Arc<RwLock<StatsData>>,
    interval_seconds: u32,
    timestamp: DateTime<Utc>,
    timer_receiver: mpsc::Receiver<()>,
    stats_provider_addr: Recipient,
}

impl Statistics {
    pub async fn new(
        description: String,
        interval_seconds: Duration,
        timer_receiver: mpsc::Receiver<()>,
    ) -> Result<Self, StatsError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        let stats_provider_config: OptionalStatsProviderConfig = client
            .get(REMOTE_SOURCE_OF_STATS_PROVIDER_CONFIG.to_string())
            .send()
            .await?
            .json()
            .await?;
        let stats_provider_addr = Recipient::try_from_base58_string(
            stats_provider_config
                .stats_client_address()
                .ok_or(StatsError::InvalidClientAddress)?,
        )
        .map_err(|_| StatsError::InvalidClientAddress)?;

        Ok(Statistics {
            description,
            request_data: Arc::new(RwLock::new(StatsData::new())),
            response_data: Arc::new(RwLock::new(StatsData::new())),
            timestamp: Utc::now(),
            interval_seconds: interval_seconds.as_secs() as u32,
            timer_receiver,
            stats_provider_addr,
        })
    }

    pub fn request_data(&self) -> &Arc<RwLock<StatsData>> {
        &self.request_data
    }

    pub fn response_data(&self) -> &Arc<RwLock<StatsData>> {
        &self.response_data
    }

    pub async fn run(&mut self, mix_input_sender: &mpsc::UnboundedSender<(Response, Recipient)>) {
        loop {
            if self.timer_receiver.next().await == None {
                error!("Timer thread has died. No more statistics will be sent");
            } else {
                let stats_data = {
                    let request_data_bytes = self.request_data.read().await;
                    let response_data_bytes = self.response_data.read().await;
                    let clients: HashSet<String> = request_data_bytes
                        .client_processed_bytes
                        .keys()
                        .chain(response_data_bytes.client_processed_bytes.keys())
                        .cloned()
                        .collect();
                    clients
                        .into_iter()
                        .map(|client_identity| {
                            let request_bytes = request_data_bytes
                                .client_processed_bytes
                                .get(&client_identity)
                                .copied()
                                .unwrap_or(0);
                            let response_bytes = response_data_bytes
                                .client_processed_bytes
                                .get(&client_identity)
                                .copied()
                                .unwrap_or(0);
                            StatsClientData::new(client_identity, request_bytes, response_bytes)
                        })
                        .collect()
                };

                let stats_message = StatsMessage {
                    description: self.description.clone(),
                    stats_data,
                    interval_seconds: self.interval_seconds,
                    timestamp: self.timestamp.to_rfc3339(),
                };
                match stats_message.to_bytes() {
                    Ok(data) => {
                        trace!("Sending data to statistics service");
                        mix_input_sender
                            .unbounded_send((
                                Response::new(0, data, false),
                                self.stats_provider_addr,
                            ))
                            .unwrap();
                    }
                    Err(e) => error!("Statistics not sent: {}", e),
                }
                self.reset_stats().await;
            }
        }
    }

    async fn reset_stats(&mut self) {
        self.request_data
            .write()
            .await
            .client_processed_bytes
            .iter_mut()
            .for_each(|(_, b)| *b = 0);
        self.response_data
            .write()
            .await
            .client_processed_bytes
            .iter_mut()
            .for_each(|(_, b)| *b = 0);
        self.timestamp = Utc::now();
    }
}
