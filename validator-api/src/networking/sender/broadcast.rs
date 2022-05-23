// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::networking::message::OffchainMessage;
use crate::networking::sender::{send_single_message, ConnectionConfig, SendResponse};
use futures::channel::mpsc;
use futures::{stream, StreamExt};
use std::net::SocketAddr;

// TODO: for now just leave it here and make it configurable with proper config later
const DEFAULT_CONCURRENCY: usize = 5;

type FeedbackSender = mpsc::UnboundedSender<SendResponse>;

pub(crate) struct Broadcaster {
    addresses: Vec<SocketAddr>,
    concurrency_level: usize,
    config: ConnectionConfig,
}

impl Broadcaster {
    pub(crate) fn new(addresses: Vec<SocketAddr>, config: ConnectionConfig) -> Self {
        Broadcaster {
            addresses,
            concurrency_level: DEFAULT_CONCURRENCY,
            config,
        }
    }

    pub(crate) fn with_concurrency_level(mut self, concurrency_level: usize) -> Self {
        self.concurrency_level = concurrency_level;
        self
    }

    pub(crate) fn set_addresses(&mut self, new_addresses: Vec<SocketAddr>) {
        self.addresses = new_addresses;
    }

    fn create_broadcast_configs(
        &self,
        message: OffchainMessage,
        feedback_sender: Option<FeedbackSender>,
    ) -> Vec<BroadcastConfig> {
        self.addresses
            .iter()
            .map(|&address| BroadcastConfig {
                address,
                config: self.config,
                feedback_sender: feedback_sender.clone(),
                message: message.clone(),
            })
            .collect()
    }

    pub(crate) async fn broadcast_with_feedback(&self, msg: OffchainMessage) -> Vec<SendResponse> {
        if self.addresses.is_empty() {
            warn!("attempting to broadcast {} while no remotes are known", msg);
            return Vec::new();
        }

        debug!("broadcasting {} to {} remotes", msg, self.addresses.len());
        let (feedback_tx, mut feedback_rx) = mpsc::unbounded();

        stream::iter(self.create_broadcast_configs(msg, Some(feedback_tx)))
            .for_each_concurrent(self.concurrency_level, |cfg| cfg.send())
            .await;

        let mut responses = Vec::new();

        for _ in 0..self.addresses.len() {
            // we should have received exactly self.addresses number of responses
            // (they could be just Err failure responses, but should exist nonetheless)
            match feedback_rx.try_next() {
                Ok(Some(response)) => responses.push(response),
                Err(_) | Ok(None) => {
                    error!("somehow we received fewer feedback responses than sent messages")
                }
            }
        }

        // the channel should have been drained and all sender should have been dropped
        debug_assert!(matches!(feedback_rx.try_next(), Ok(None)));
        responses
    }

    pub(crate) async fn broadcast(&self, msg: OffchainMessage) {
        if self.addresses.is_empty() {
            warn!("attempting to broadcast {} while no remotes are known", msg);
            return;
        }

        debug!("broadcasting {} to {} remotes", msg, self.addresses.len());
        stream::iter(self.create_broadcast_configs(msg, None))
            .for_each_concurrent(self.concurrency_level, |cfg| cfg.send())
            .await
    }
}

// internal struct to have per-connection config on hand
struct BroadcastConfig {
    address: SocketAddr,
    config: ConnectionConfig,
    feedback_sender: Option<FeedbackSender>,
    message: OffchainMessage,
}

impl BroadcastConfig {
    async fn send(self) {
        let response = send_single_message(self.address, self.config, &self.message).await;
        if let Some(feedback_sender) = self.feedback_sender {
            // this can only fail if the receiver is disconnected which should never be the case
            // thus we can ignore the possible error
            let _ = feedback_sender.unbounded_send(response);
        } else if let Err(err) = response.response {
            // if we're not forwarding feedback, at least emit a warning about the failure
            warn!(
                "failed to broadcast {} to {} - {}",
                self.message, self.address, err
            )
        }
    }
}
