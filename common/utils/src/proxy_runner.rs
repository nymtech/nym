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

use super::read_delay_loop::try_read_data;
use crate::connection_controller::ConnectionReceiver;
use futures::channel::mpsc;
use log::*;
use simple_socks5_requests::ConnectionId;
use std::sync::Arc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::stream::StreamExt;
use tokio::sync::Notify;

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
    connection_id: ConnectionId,
}

impl<S> ProxyRunner<S>
where
    S: Send + 'static,
{
    pub fn new(
        socket: TcpStream,
        mix_receiver: ConnectionReceiver,
        mix_sender: MixProxySender<S>,
        connection_id: ConnectionId,
    ) -> Self {
        ProxyRunner {
            mix_receiver: Some(mix_receiver),
            mix_sender,
            socket: Some(socket),
            connection_id,
        }
    }

    async fn run_inbound<F>(
        mut reader: OwnedReadHalf,
        notify_closed: Arc<Notify>,
        connection_id: ConnectionId,
        mix_sender: MixProxySender<S>,
        adapter_fn: F,
    ) -> OwnedReadHalf
    where
        F: Fn(ConnectionId, Vec<u8>, bool) -> S + Send + 'static,
    {
        // TODO: to be removed with sequence numbers...
        let socket_read_timeout_duration = std::time::Duration::from_millis(500);

        let address = reader.as_ref().peer_addr().unwrap().to_string();
        loop {
            tokio::select! {
                _ = notify_closed.notified() => {
                    // the remote socket is closed, so there's no point
                    // in reading anything more because we won't be able to write to remote anyway!
                    break
                }
                // try to read from local socket and push everything to mixnet to the remote
                reading_result = try_read_data(socket_read_timeout_duration, &mut reader, &address) => {
                    let (read_data, timed_out) = match reading_result {
                        Ok(data) => data,
                        Err(err) => {
                            error!("failed to read request from the socket - {}", err);
                            break;
                        }
                    };

                    if read_data.is_empty() && timed_out {
                        // no point in writing empty data on each timeout
                        continue
                    }

                    info!(
                        "Going to send {} bytes via mixnet to remote {}. Is local closed: {}",
                        read_data.len(),
                        connection_id,
                        !timed_out
                    );

                    mix_sender.unbounded_send(adapter_fn(connection_id, read_data, !timed_out)).unwrap();

                    if !timed_out {
                        // technically we already informed it when we sent the message to mixnet above
                        info!("The local socket is closed - won't receive any more data. Informing remote about that...");
                        // no point in reading from mixnet if connection is closed!
                        notify_closed.notify();
                        break;
                    }
                }
            }
        }

        reader
    }

    async fn run_outbound(
        mut writer: OwnedWriteHalf,
        notify_closed: Arc<Notify>,
        mut mix_receiver: ConnectionReceiver,
        connection_id: ConnectionId,
    ) -> (OwnedWriteHalf, ConnectionReceiver) {
        loop {
            tokio::select! {
                _ = notify_closed.notified() => {
                    // no need to read from mixnet as we won't be able to send to socket
                    // anyway
                    break
                }
                mix_data = mix_receiver.next() => {
                    if mix_data.is_none() {
                        warn!("mix receiver is none so we already got removed somewhere. This isn't really a warning, but shouldn't happen to begin with, so please say if you see this message");
                        // we already got closed
                        // not sure if we HAVE TO notify the other task, but might as well
                        notify_closed.notify();
                        break
                    }
                    let connection_message = mix_data.unwrap();

                    info!(
                        "Going to write {} bytes received from mixnet to connection {}. Is remote closed: {}",
                        connection_message.payload.len(),
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
                        notify_closed.notify();
                        break
                    }
                }
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
        let notify_closed = Arc::new(Notify::new());
        let notify_clone = Arc::clone(&notify_closed);

        let (read_half, write_half) = self.socket.take().unwrap().into_split();
        let mix_receiver = self.mix_receiver.take().unwrap();

        // should run until either inbound closes or is notified from outbound
        let inbound_future = Self::run_inbound(
            read_half,
            notify_closed,
            self.connection_id,
            self.mix_sender.clone(),
            adapter_fn,
        );
        let outbound_future =
            Self::run_outbound(write_half, notify_clone, mix_receiver, self.connection_id);

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

        let (write_half, mix_receiver) = outbound_result.unwrap();

        self.socket = Some(write_half.reunite(inbound_result.unwrap()).unwrap());
        self.mix_receiver = Some(mix_receiver);
        self
    }

    pub fn into_inner(mut self) -> (TcpStream, ConnectionReceiver) {
        (
            self.socket.take().unwrap(),
            self.mix_receiver.take().unwrap(),
        )
    }
}
