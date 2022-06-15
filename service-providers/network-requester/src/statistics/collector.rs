// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::channel::mpsc;
use log::*;
use rand::RngCore;
use serde::Deserialize;
use sqlx::types::chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use network_defaults::DEFAULT_NETWORK;
use nymsphinx::addressing::clients::Recipient;
use ordered_buffer::OrderedMessageSender;
use socks5_requests::{ConnectionId, Message as Socks5Message, RemoteAddress, Request};
use statistics_common::api::{
    build_statistics_request_bytes, DEFAULT_STATISTICS_SERVICE_ADDRESS,
    DEFAULT_STATISTICS_SERVICE_PORT,
};
use statistics_common::{
    collector::StatisticsCollector, error::StatsError as CommonStatsError, StatsMessage,
    StatsServiceData,
};

use super::error::StatsError;

const REMOTE_SOURCE_OF_STATS_PROVIDER_CONFIG: &str =
    "https://nymtech.net/.wellknown/network-requester/stats-provider.json";

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

    pub fn processed(&mut self, remote_addr: &str, bytes: u32) {
        if let Some(curr_bytes) = self.client_processed_bytes.get_mut(remote_addr) {
            *curr_bytes += bytes;
        } else {
            self.client_processed_bytes
                .insert(remote_addr.to_string(), bytes);
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
        };
        entry_config.map(|e| e.stats_client_address)
    }
}

#[derive(Clone)]
pub struct ServiceStatisticsCollector {
    pub(crate) request_stats_data: Arc<RwLock<StatsData>>,
    pub(crate) response_stats_data: Arc<RwLock<StatsData>>,
    pub(crate) connected_services: Arc<RwLock<HashMap<ConnectionId, RemoteAddress>>>,
    stats_provider_addr: Recipient,
    mix_input_sender: mpsc::UnboundedSender<(Socks5Message, Recipient)>,
}

impl ServiceStatisticsCollector {
    pub async fn new(
        stats_provider_addr: Option<Recipient>,
        mix_input_sender: mpsc::UnboundedSender<(Socks5Message, Recipient)>,
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
        let stats_provider_addr = stats_provider_addr.unwrap_or(
            Recipient::try_from_base58_string(
                stats_provider_config
                    .stats_client_address()
                    .ok_or(StatsError::InvalidClientAddress)?,
            )
            .map_err(|_| StatsError::InvalidClientAddress)?,
        );

        Ok(ServiceStatisticsCollector {
            request_stats_data: Arc::new(RwLock::new(StatsData::new())),
            response_stats_data: Arc::new(RwLock::new(StatsData::new())),
            connected_services: Arc::new(RwLock::new(HashMap::new())),
            stats_provider_addr,
            mix_input_sender,
        })
    }
}

#[async_trait]
impl StatisticsCollector for ServiceStatisticsCollector {
    async fn create_stats_message(
        &self,
        interval: Duration,
        timestamp: DateTime<Utc>,
    ) -> StatsMessage {
        let stats_data = {
            let request_data_bytes = self.request_stats_data.read().await;
            let response_data_bytes = self.response_stats_data.read().await;
            let services: HashSet<String> = request_data_bytes
                .client_processed_bytes
                .keys()
                .chain(response_data_bytes.client_processed_bytes.keys())
                .cloned()
                .collect();
            services
                .into_iter()
                .map(|requested_service| {
                    let request_bytes = request_data_bytes
                        .client_processed_bytes
                        .get(&requested_service)
                        .copied()
                        .unwrap_or(0);
                    let response_bytes = response_data_bytes
                        .client_processed_bytes
                        .get(&requested_service)
                        .copied()
                        .unwrap_or(0);
                    statistics_common::StatsData::Service(StatsServiceData::new(
                        requested_service,
                        request_bytes,
                        response_bytes,
                    ))
                })
                .collect()
        };

        StatsMessage {
            stats_data,
            interval_seconds: interval.as_secs() as u32,
            timestamp: timestamp.to_rfc3339(),
        }
    }

    async fn send_stats_message(
        &self,
        stats_message: StatsMessage,
    ) -> Result<(), CommonStatsError> {
        let msg = build_statistics_request_bytes(stats_message)?;

        trace!("Connecting to statistics service");
        let mut rng = rand::rngs::OsRng;
        let conn_id = rng.next_u64();
        let connect_req = Request::new_connect(
            conn_id,
            format!(
                "{}:{}",
                DEFAULT_STATISTICS_SERVICE_ADDRESS, DEFAULT_STATISTICS_SERVICE_PORT
            ),
            self.stats_provider_addr,
        );
        self.mix_input_sender
            .unbounded_send((
                Socks5Message::Request(connect_req),
                self.stats_provider_addr,
            ))
            .unwrap();

        trace!("Sending data to statistics service");
        let mut message_sender = OrderedMessageSender::new();
        let ordered_msg = message_sender.wrap_message(msg).into_bytes();
        let send_req = Request::new_send(conn_id, ordered_msg, true);
        self.mix_input_sender
            .unbounded_send((Socks5Message::Request(send_req), self.stats_provider_addr))
            .unwrap();

        Ok(())
    }

    async fn reset_stats(&mut self) {
        self.request_stats_data
            .write()
            .await
            .client_processed_bytes
            .iter_mut()
            .for_each(|(_, b)| *b = 0);
        self.response_stats_data
            .write()
            .await
            .client_processed_bytes
            .iter_mut()
            .for_each(|(_, b)| *b = 0);
    }
}
