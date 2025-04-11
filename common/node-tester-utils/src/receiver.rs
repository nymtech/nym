// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkTestingError;
use crate::processor::{Received, TestPacketProcessor};
use crate::{log_err, log_info, log_warn};
use futures::channel::mpsc;
use futures::StreamExt;
use nym_crypto::asymmetric::x25519;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::receiver::{MessageReceiver, SphinxMessageReceiver};
use nym_task::TaskClient;
use serde::de::DeserializeOwned;
use std::sync::Arc;

pub type ReceivedSender<T> = mpsc::UnboundedSender<Received<T>>;
pub type ReceivedReceiver<T> = mpsc::UnboundedReceiver<Received<T>>;

// the 'Simple' bit comes from the fact that it expects all received messages to consist of a single `Fragment`
pub struct SimpleMessageReceiver<T, R: MessageReceiver = SphinxMessageReceiver> {
    message_processor: TestPacketProcessor<T, R>,

    mixnet_message_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
    acks_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,

    received_sender: ReceivedSender<T>,
    shutdown: TaskClient,
}

impl<T> SimpleMessageReceiver<T, SphinxMessageReceiver> {
    pub fn new_sphinx_receiver(
        local_encryption_keypair: Arc<x25519::KeyPair>,
        ack_key: Arc<AckKey>,
        mixnet_message_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
        acks_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
        received_sender: ReceivedSender<T>,
        shutdown: TaskClient,
    ) -> Self {
        Self::new(
            local_encryption_keypair,
            ack_key,
            mixnet_message_receiver,
            acks_receiver,
            received_sender,
            shutdown,
        )
    }
}

impl<T, R: MessageReceiver> SimpleMessageReceiver<T, R> {
    pub fn new(
        local_encryption_keypair: Arc<x25519::KeyPair>,
        ack_key: Arc<AckKey>,
        mixnet_message_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
        acks_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
        received_sender: ReceivedSender<T>,
        shutdown: TaskClient,
    ) -> Self {
        SimpleMessageReceiver {
            message_processor: TestPacketProcessor::new(local_encryption_keypair, ack_key),
            mixnet_message_receiver,
            acks_receiver,
            received_sender,
            shutdown,
        }
    }

    fn forward_received<U: Into<Received<T>>>(&self, received: U) {
        // TODO: remove the unwrap once/if we do graceful shutdowns here
        self.received_sender
            .unbounded_send(received.into())
            .expect("ReceivedReceiver has stopped receiving");
    }

    fn on_mixnet_message(&mut self, raw_message: Vec<u8>) -> Result<(), NetworkTestingError>
    where
        T: DeserializeOwned,
    {
        let recovered = self.message_processor.process_mixnet_message(raw_message)?;
        self.forward_received(recovered);
        Ok(())
    }

    fn on_ack(&mut self, raw_ack: Vec<u8>) -> Result<(), NetworkTestingError> {
        let frag_id = self.message_processor.process_ack(raw_ack)?;
        self.forward_received(frag_id);
        Ok(())
    }

    pub async fn run(&mut self)
    where
        T: DeserializeOwned,
    {
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
                    log_info!("SimpleMessageReceiver: received shutdown")
                }
                mixnet_messages = self.mixnet_message_receiver.next() => {
                    let Some(mixnet_messages) = mixnet_messages else {
                        log_err!("the mixnet messages stream has terminated!");
                        // note: this will cause global shutdown, but we have no choice if we stopped receiving mixnet messages
                        break
                    };
                    for message in mixnet_messages {
                        if let Err(err) = self.on_mixnet_message(message) {
                            log_warn!("failed to process received mixnet message: {err}")
                        }
                    }
                }
                acks = self.acks_receiver.next() => {
                    let Some(acks) = acks else {
                        log_err!("the ack messages stream has terminated!");
                        // note: this will cause global shutdown, but we have no choice if we stopped receiving mixnet messages
                        break
                    };
                    for ack in acks {
                        if let Err(err) = self.on_ack(ack) {
                            log_warn!("failed to process received ack message: {err}")
                        }
                    }
                }
            }
        }

        log_info!("SimpleMessageReceiver: Exiting")
    }
}
