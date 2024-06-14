// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::proxy_runner::MixProxySender;
use bytes::Bytes;
use futures::SinkExt;
use log::{debug, error};
use nym_socks5_requests::{ConnectionId, SocketData};
use std::io;

pub(crate) struct OrderedMessageSender<F, S: Send + 'static> {
    connection_id: ConnectionId,
    // addresses are provided for better logging
    local_destination_address: String,
    remote_source_address: String,
    mixnet_sender: MixProxySender<S>,

    next_message_seq: u64,
    mix_message_adapter: F,
}

impl<F, S: Send + 'static> OrderedMessageSender<F, S>
where
    F: Fn(SocketData) -> S,
{
    pub(crate) fn new(
        local_destination_address: String,
        remote_source_address: String,
        connection_id: ConnectionId,
        mixnet_sender: MixProxySender<S>,
        mix_message_adapter: F,
    ) -> Self {
        OrderedMessageSender {
            local_destination_address,
            remote_source_address,
            connection_id,
            mixnet_sender,
            next_message_seq: 0,
            mix_message_adapter,
        }
    }

    fn sequence(&mut self) -> u64 {
        let next = self.next_message_seq;
        self.next_message_seq += 1;
        next
    }

    fn construct_message(&mut self, data: Vec<u8>, local_socket_closed: bool) -> S {
        let data = SocketData::new(
            self.sequence(),
            self.connection_id,
            local_socket_closed,
            data,
        );
        (self.mix_message_adapter)(data)
    }

    async fn send_message(&mut self, message: S) {
        if self.mixnet_sender.send(message).await.is_err() {
            panic!("BatchRealMessageReceiver has stopped receiving!")
        }
    }

    pub(crate) async fn send_empty_close(&mut self) {
        let message = self.construct_message(Vec::new(), true);
        self.send_message(message).await
    }

    pub(crate) async fn send_empty_keepalive(&mut self) {
        log::trace!("Sending keepalive for connection: {}", self.connection_id);
        let message = self.construct_message(Vec::new(), false);
        self.send_message(message).await
    }

    pub(crate) fn process_data(&self, read_data: Option<io::Result<Bytes>>) -> ProcessedData {
        let (read_data, is_finished) = match read_data {
            Some(data) => match data {
                Ok(data) => (data, false),
                Err(err) => {
                    error!(target: &*format!("({}) socks5 inbound", self.connection_id), "failed to read request from the socket - {err}");
                    (Default::default(), true)
                }
            },
            None => (Default::default(), true),
        };

        ProcessedData {
            data: read_data,
            is_done: is_finished,
        }
    }

    fn log_sent_message(&self, data: &ProcessedData) {
        debug!(
            target: &*format!("({}) socks5 inbound", self.connection_id),
            "[{} bytes]\t{} → local → mixnet → remote → {}. Local closed: {}",
            data.data.len(),
            self.local_destination_address,
            self.remote_source_address,
            data.is_done
        );
    }

    /// Send data read from local socket into the mixnet
    pub(crate) async fn send_data(&mut self, data: ProcessedData) {
        self.log_sent_message(&data);
        let message = self.construct_message(data.data.into(), data.is_done);
        self.send_message(message).await;
    }
}

// helper wrapper to keep track of field meanings
pub(crate) struct ProcessedData {
    data: Bytes,
    pub(crate) is_done: bool,
}
