// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use tokio::net::tcp::{OwnedReadHalf};
use super::MixProxySender;
use crate::available_reader::AvailableReader;
use log::*;
use ordered_buffer::OrderedMessageSender;
use socks5_requests::ConnectionId;
use tokio::stream::StreamExt;

pub(super) async fn run_inbound<F, S>(
    mut reader: OwnedReadHalf,
    local_destination_address: String, // addresses are provided for better logging
    remote_source_address: String,
    connection_id: ConnectionId,
    mix_sender: MixProxySender<S>,
    adapter_fn: F,
) -> OwnedReadHalf
where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S + Send + 'static,
{
    let mut available_reader = AvailableReader::new(&mut reader);
    let mut message_sender = OrderedMessageSender::new();

    loop {
        // try to read from local socket and push everything to mixnet to the remote
        let (read_data, is_finished) = match available_reader.next().await {
            Some(data) => match data {
                Ok(data) => (data, false),
                Err(err) => {
                    error!(target: &*format!("({}) socks5 inbound", connection_id),"failed to read request from the socket - {}", err);
                    (Default::default(), true)
                }
            },
            None => (Default::default(), true),
        };

        debug!(
            target: &*format!("({}) socks5 inbound", connection_id),
            "[{} bytes]\t{} → local → mixnet → remote → {}. Local closed: {}",
            read_data.len(),
            local_destination_address,
            remote_source_address,
            is_finished
        );

        // if we're sending through the mixnet increase the sequence number...
        let ordered_msg = message_sender.wrap_message(read_data.to_vec()).into_bytes();
        mix_sender
            .unbounded_send(adapter_fn(connection_id, ordered_msg, is_finished))
            .unwrap();

        if is_finished {
            // technically we already informed it when we sent the message to mixnet above
            debug!(target: &*format!("({}) socks5 inbound", connection_id), "The local socket is closed - won't receive any more data. Informing remote about that...");
            break;
        }
    }

    reader
}