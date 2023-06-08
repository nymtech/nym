// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::proxy_runner::MixProxySender;
use bytes::Bytes;
use log::{debug, error};
use nym_socks5_requests::{ConnectionId, SocketData};
use std::io;

pub struct OrderedMessageSender<S, F> {
    connection_id: ConnectionId,
    mixnet_sender: MixProxySender<S>,

    next_message_seq: u64,
    mix_message_adapter: F,
}

impl<S, F> OrderedMessageSender<S, F>
where
    F: Fn(SocketData) -> S,
{
    pub fn new(
        connection_id: ConnectionId,
        mixnet_sender: MixProxySender<S>,
        mix_message_adapter: F,
    ) -> Self {
        OrderedMessageSender {
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

    async fn send_message(&self, message: S) {
        if self.mixnet_sender.send(message).await.is_err() {
            panic!("BatchRealMessageReceiver has stopped receiving!")
        }
    }

    pub async fn send_empty_close(&mut self) {
        let message = self.construct_message(Vec::new(), true);
        self.send_message(message).await
    }

    pub async fn send_empty_keepalive(&mut self) {
        log::trace!("Sending keepalive for connection: {}", self.connection_id);
        let message = self.construct_message(Vec::new(), false);
        self.send_message(message).await
    }

    pub async fn deal_with_data(
        &mut self,
        read_data: Option<io::Result<Bytes>>,
        local_destination_address: &str,
        remote_source_address: &str,
    ) -> bool {
        let connection_id = self.connection_id;
        let (read_data, is_finished) = match read_data {
            Some(data) => match data {
                Ok(data) => (data, false),
                Err(err) => {
                    error!(target: &*format!("({connection_id}) socks5 inbound"), "failed to read request from the socket - {err}");
                    (Default::default(), true)
                }
            },
            None => (Default::default(), true),
        };

        debug!(
            target: &*format!("({connection_id}) socks5 inbound"),
            "[{} bytes]\t{} → local → mixnet → remote → {}. Local closed: {}",
            read_data.len(),
            local_destination_address,
            remote_source_address,
            is_finished
        );
        let message = self.construct_message(read_data.into(), is_finished);
        self.send_message(message).await;

        is_finished
    }
}
