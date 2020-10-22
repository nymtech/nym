// Copyright 2020 Nym Technologies SA
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

use crate::available_reader::AvailableReader;
use crate::connection_controller::ConnectionReceiver;
use futures::channel::mpsc;
use log::*;
use ordered_buffer::OrderedMessageSender;
use socks5_requests::ConnectionId;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::stream::StreamExt;

#[derive(Debug)]
pub struct ProxyMessage {
    pub data: Vec<u8>,
    pub socket_closed: bool,
}

impl Into<ProxyMessage> for (Vec<u8>, bool) {
    fn into(self) -> ProxyMessage {
        ProxyMessage {
            data: self.0,
            socket_closed: self.1,
        }
    }
}

pub type MixProxySender<S> = mpsc::UnboundedSender<S>;

#[derive(Debug)]
pub struct ProxyRunner<S> {
    /// receives data from the mix network and sends that into the socket
    mix_receiver: Option<ConnectionReceiver>,

    /// sends whatever was read from the socket into the mix network
    mix_sender: MixProxySender<S>,

    socket: Option<TcpStream>,
    socket_address: String,
    connection_id: ConnectionId,

    // required for in-order delivery
    message_sender: Option<OrderedMessageSender>,
}

impl<S> ProxyRunner<S>
where
    S: Send + 'static,
{
    pub fn new(
        socket: TcpStream,
        socket_address: String, // passed explicitly for better logging
        mix_receiver: ConnectionReceiver,
        mix_sender: MixProxySender<S>,
        connection_id: ConnectionId,
        message_sender: OrderedMessageSender,
    ) -> Self {
        ProxyRunner {
            mix_receiver: Some(mix_receiver),
            mix_sender,
            socket: Some(socket),
            socket_address,
            connection_id,
            message_sender: Some(message_sender),
        }
    }

    async fn run_inbound<F>(
        mut reader: OwnedReadHalf,
        socket_addr: String,
        connection_id: ConnectionId,
        mix_sender: MixProxySender<S>,
        adapter_fn: F,
        mut message_sender: OrderedMessageSender,
    ) -> (OwnedReadHalf, OrderedMessageSender)
    where
        F: Fn(ConnectionId, Vec<u8>, bool) -> S + Send + 'static,
    {
        let mut available_reader = AvailableReader::new(&mut reader);

        loop {
            // try to read from local socket and push everything to mixnet to the remote
            let (read_data, is_finished) = match available_reader.next().await {
                Some(data) => match data {
                    Ok(data) => (data, false),
                    Err(err) => {
                        error!("failed to read request from the socket - {}", err);
                        break;
                    }
                },
                None => (Default::default(), true),
            };

            info!(
                "[{} bytes]\t{} → local → mixnet → remote (conn_id: {}). Local closed: {}",
                read_data.len(),
                socket_addr,
                connection_id,
                is_finished
            );

            // if we're sending through the mixnet increase the sequence number...
            let ordered_msg = message_sender.wrap_message(read_data.to_vec()).into_bytes();
            mix_sender
                .unbounded_send(adapter_fn(connection_id, ordered_msg, is_finished))
                .unwrap();

            if is_finished {
                // technically we already informed it when we sent the message to mixnet above
                info!("The local socket is closed - won't receive any more data. Informing remote about that...");
                break;
            }
        }

        (reader, message_sender)
    }

    async fn run_outbound(
        mut writer: OwnedWriteHalf,
        socket_addr: String,
        mut mix_receiver: ConnectionReceiver,
        connection_id: ConnectionId,
    ) -> (OwnedWriteHalf, ConnectionReceiver) {
        loop {
            let mix_data = mix_receiver.next().await;
            if mix_data.is_none() {
                warn!("mix receiver is none so we already got removed somewhere. This isn't really a warning, but shouldn't happen to begin with, so please say if you see this message");
                break;
            }
            let connection_message = mix_data.unwrap();

            info!(
                "[{} bytes]\tremote → mixnet → local → {} (conn_id: {}). Remote closed: {}",
                connection_message.payload.len(),
                socket_addr,
                connection_id,
                connection_message.socket_closed
            );

            if let Err(err) = writer.write_all(&connection_message.payload).await {
                // the other half is probably going to blow up too (if not, this task also needs to notify the other one!!)
                error!("failed to write response back to the socket - {}", err);
                break;
            }
            if connection_message.socket_closed {
                info!("Remote socket got closed - closing the local socket too");
                break;
            }
        }

        (writer, mix_receiver)
    }

    // The `adapter_fn` is used to transform whatever was read into appropriate
    // request/response as required by entity running particular side of the proxy.
    pub async fn run<F>(mut self, adapter_fn: F) -> Self
    where
        F: Fn(ConnectionId, Vec<u8>, bool) -> S + Send + 'static,
    {
        let (read_half, write_half) = self.socket.take().unwrap().into_split();

        // should run until either inbound closes or is notified from outbound
        let inbound_future = Self::run_inbound(
            read_half,
            self.socket_address.clone(),
            self.connection_id,
            self.mix_sender.clone(),
            adapter_fn,
            self.message_sender.take().unwrap(),
        );

        let outbound_future = Self::run_outbound(
            write_half,
            self.socket_address.clone(),
            self.mix_receiver.take().unwrap(),
            self.connection_id,
        );

        // TODO: this shouldn't really have to spawn tasks inside "library" code, but
        // if we used join directly, stuff would have been executed on the same thread
        // (it's not bad, but an unnecessary slowdown)
        let handle_inbound = tokio::spawn(inbound_future);
        let handle_outbound = tokio::spawn(outbound_future);

        let (inbound_result, outbound_result) =
            futures::future::join(handle_inbound, handle_outbound).await;

        if inbound_result.is_err() || outbound_result.is_err() {
            panic!("TODO: some future error?")
        }

        let (read_half, message_sender) = inbound_result.unwrap();
        let (write_half, mix_receiver) = outbound_result.unwrap();

        self.socket = Some(write_half.reunite(read_half).unwrap());
        self.mix_receiver = Some(mix_receiver);
        self.message_sender = Some(message_sender);
        self
    }

    pub fn into_inner(mut self) -> (TcpStream, ConnectionReceiver) {
        (
            self.socket.take().unwrap(),
            self.mix_receiver.take().unwrap(),
        )
    }
}
