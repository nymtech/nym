// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use nymsphinx::addressing::clients::{ClientEncryptionKey, ClientIdentity, Recipient};
use nymsphinx::addressing::nodes::NodeIdentity;
use socks5_requests::Response;

use super::error::StatsError;

const STATS_PROVIDER_CLIENT_IDENTITY: &str = "HqYWvCcB4sswYiyMj5Q8H5oc71kLf96vfrLK3npM7stH";
const STATS_PROVIDER_ENCRYPTION_KEY: &str = "CoeC5dcqurgdxr5zcgU77nZBSBCc8ntCiwUivQ9TX3KT";
const STATS_PROVIDER_GATEWAY_IDENTITY: &str = "E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatsMessage {
    pub description: String,
    pub request_data: StatsData,
    pub response_data: StatsData,
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
pub struct StatsData {
    total_processed_bytes: u32,
}

impl StatsData {
    pub fn new(total_processed_bytes: u32) -> Self {
        StatsData {
            total_processed_bytes,
        }
    }
    pub fn processed(&mut self, bytes: u32) {
        self.total_processed_bytes += bytes;
    }

    #[cfg(feature = "stats-service")]
    pub fn total_processed_bytes(&self) -> u32 {
        self.total_processed_bytes
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
    pub fn new(
        description: String,
        interval_seconds: Duration,
        timer_receiver: mpsc::Receiver<()>,
    ) -> Self {
        // those unwraps are ok because we set the strings in the constants above
        let stats_provider_addr = Recipient::new(
            ClientIdentity::from_base58_string(STATS_PROVIDER_CLIENT_IDENTITY).unwrap(),
            ClientEncryptionKey::from_base58_string(STATS_PROVIDER_ENCRYPTION_KEY).unwrap(),
            NodeIdentity::from_base58_string(STATS_PROVIDER_GATEWAY_IDENTITY).unwrap(),
        );
        Statistics {
            description,
            request_data: Arc::new(RwLock::new(StatsData::new(0))),
            response_data: Arc::new(RwLock::new(StatsData::new(0))),
            timestamp: Utc::now(),
            interval_seconds: interval_seconds.as_secs() as u32,
            timer_receiver,
            stats_provider_addr,
        }
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
                let stats_message = StatsMessage {
                    description: self.description.clone(),
                    request_data: self.request_data.read().await.clone(),
                    response_data: self.response_data.read().await.clone(),
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
        self.request_data.write().await.total_processed_bytes = 0;
        self.response_data.write().await.total_processed_bytes = 0;
        self.timestamp = Utc::now();
    }
}
