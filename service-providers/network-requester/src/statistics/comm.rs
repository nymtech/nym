// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::StreamExt;
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
use statistics::{StatsMessage, StatsServiceData};

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
pub struct StatisticsCollector {
    pub(crate) request_stats_data: Arc<RwLock<StatsData>>,
    pub(crate) response_stats_data: Arc<RwLock<StatsData>>,
    pub(crate) connected_services: Arc<RwLock<HashMap<ConnectionId, RemoteAddress>>>,
}

impl StatisticsCollector {
    pub fn from(stats: &StatisticsSender) -> Self {
        Self {
            request_stats_data: Arc::clone(&stats.request_data),
            response_stats_data: Arc::clone(&stats.response_data),
            connected_services: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

pub struct StatisticsSender {
    request_data: Arc<RwLock<StatsData>>,
    response_data: Arc<RwLock<StatsData>>,
    interval_seconds: u32,
    timestamp: DateTime<Utc>,
    timer_receiver: mpsc::Receiver<()>,
    stats_provider_addr: Recipient,
}

impl StatisticsSender {
    pub async fn new(
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

        Ok(StatisticsSender {
            request_data: Arc::new(RwLock::new(StatsData::new())),
            response_data: Arc::new(RwLock::new(StatsData::new())),
            timestamp: Utc::now(),
            interval_seconds: interval_seconds.as_secs() as u32,
            timer_receiver,
            stats_provider_addr,
        })
    }

    pub async fn run(
        &mut self,
        mix_input_sender: &mpsc::UnboundedSender<(Socks5Message, Recipient)>,
    ) {
        loop {
            if self.timer_receiver.next().await == None {
                error!("Timer thread has died. No more statistics will be sent");
            } else {
                let stats_data = {
                    let request_data_bytes = self.request_data.read().await;
                    let response_data_bytes = self.response_data.read().await;
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
                            StatsServiceData::new(requested_service, request_bytes, response_bytes)
                        })
                        .collect()
                };

                let stats_message = StatsMessage {
                    stats_data,
                    interval_seconds: self.interval_seconds,
                    timestamp: self.timestamp.to_rfc3339(),
                };
                match stats_message.to_json() {
                    Ok(msg) => {
                        trace!("Connecting to statistics service");
                        let mut rng = rand::rngs::OsRng;
                        let conn_id = rng.next_u64();
                        let connect_req = Request::new_connect(
                            conn_id,
                            String::from("127.0.0.1:8080"),
                            self.stats_provider_addr,
                        );
                        mix_input_sender
                            .unbounded_send((
                                Socks5Message::Request(connect_req),
                                self.stats_provider_addr,
                            ))
                            .unwrap();

                        trace!("Sending data to statistics service");
                        let req = reqwest::Request::new(
                            reqwest::Method::POST,
                            reqwest::Url::parse("http://localhost:8080/statistics").unwrap(),
                        );
                        let data = format!(
                            "{} {} {:?}\n{}",
                            req.method().as_str(),
                            req.url().as_str(),
                            req.version(),
                            msg
                        )
                        .into_bytes();
                        let mut message_sender = OrderedMessageSender::new();
                        let ordered_msg = message_sender.wrap_message(data).into_bytes();
                        let send_req = Request::new_send(conn_id, ordered_msg, true);
                        mix_input_sender
                            .unbounded_send((
                                Socks5Message::Request(send_req),
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
