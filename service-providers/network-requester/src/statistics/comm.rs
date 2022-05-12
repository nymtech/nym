// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use log::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

use nymsphinx::addressing::clients::{ClientEncryptionKey, ClientIdentity, Recipient};
use nymsphinx::addressing::nodes::NodeIdentity;
use socks5_requests::Response;

use super::error::StatsError;

#[derive(Debug, Deserialize, Serialize)]
pub struct StatsData {
    description: String,
    total_processed_bytes: usize,
}

impl Display for StatsData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Processed bytes for {}: {}",
            self.description, self.total_processed_bytes
        )
    }
}

impl StatsData {
    pub fn to_bytes(&self) -> Result<Vec<u8>, StatsError> {
        Ok(bincode::serialize(self)?)
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, StatsError> {
        Ok(bincode::deserialize(b)?)
    }
}

pub struct Statistics {
    data: StatsData,
    timer_receiver: mpsc::Receiver<()>,
}

impl Statistics {
    pub fn new(description: String, timer_receiver: mpsc::Receiver<()>) -> Self {
        Statistics {
            data: StatsData {
                description,
                total_processed_bytes: 0,
            },
            timer_receiver,
        }
    }

    pub fn processed(&mut self, bytes: usize) {
        self.data.total_processed_bytes += bytes;
    }

    fn reset_stats(&mut self) {
        self.data.total_processed_bytes = 0;
    }

    pub fn maybe_send(&mut self, mix_input_sender: &mpsc::UnboundedSender<(Response, Recipient)>) {
        match self.timer_receiver.try_next() {
            Ok(Some(_)) => {
                match self.data.to_bytes() {
                    Ok(data) => {
                        trace!("Sending data to statistics service");
                        mix_input_sender
                            .unbounded_send((
                                Response::new(0, data, false),
                                Recipient::new(
                                    ClientIdentity::from_base58_string(
                                        "HqYWvCcB4sswYiyMj5Q8H5oc71kLf96vfrLK3npM7stH",
                                    )
                                    .unwrap(),
                                    ClientEncryptionKey::from_base58_string(
                                        "CoeC5dcqurgdxr5zcgU77nZBSBCc8ntCiwUivQ9TX3KT",
                                    )
                                    .unwrap(),
                                    NodeIdentity::from_base58_string(
                                        "E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM",
                                    )
                                    .unwrap(),
                                ),
                            ))
                            .unwrap();
                    }
                    Err(e) => error!("Statistics not sent: {}", e),
                }
                self.reset_stats();
            }
            Ok(None) => error!("Timer thread has died. No more statistics will be sent"),
            Err(_) => {}
        }
    }
}
