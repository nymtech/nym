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

use crate::connection_controller::ConnectionReceiver;
use futures::channel::mpsc;
use socks5_requests::ConnectionId;
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpStream, sync::Notify};

mod inbound;
mod outbound;

// TODO: make this configurable
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct ProxyMessage {
    pub data: Vec<u8>,
    pub socket_closed: bool,
}

impl From<(Vec<u8>, bool)> for ProxyMessage {
    fn from(data: (Vec<u8>, bool)) -> Self {
        ProxyMessage {
            data: data.0,
            socket_closed: data.1,
        }
    }
}

pub type MixProxySender<S> = mpsc::UnboundedSender<S>;

// TODO: when we finally get to implementing graceful shutdown,
// on Drop this guy should tell the remote that it's closed now
#[derive(Debug)]
pub struct ProxyRunner<S> {
    /// receives data from the mix network and sends that into the socket
    mix_receiver: Option<ConnectionReceiver>,

    /// sends whatever was read from the socket into the mix network
    mix_sender: MixProxySender<S>,

    socket: Option<TcpStream>,
    local_destination_address: String,
    remote_source_address: String,
    connection_id: ConnectionId,
}

impl<S> ProxyRunner<S>
where
    S: Send + 'static,
{
    pub fn new(
        socket: TcpStream,
        local_destination_address: String, // addresses are provided for better logging
        remote_source_address: String,
        mix_receiver: ConnectionReceiver,
        mix_sender: MixProxySender<S>,
        connection_id: ConnectionId,
    ) -> Self {
        ProxyRunner {
            mix_receiver: Some(mix_receiver),
            mix_sender,
            socket: Some(socket),
            local_destination_address,
            remote_source_address,
            connection_id,
        }
    }

    // The `adapter_fn` is used to transform whatever was read into appropriate
    // request/response as required by entity running particular side of the proxy.
    pub async fn run<F>(mut self, adapter_fn: F) -> Self
    where
        F: Fn(ConnectionId, Vec<u8>, bool) -> S + Send + 'static,
    {
        let (read_half, write_half) = self.socket.take().unwrap().into_split();
        let shutdown_notify = Arc::new(Notify::new());

        // should run until either inbound closes or is notified from outbound
        let inbound_future = inbound::run_inbound(
            read_half,
            self.local_destination_address.clone(),
            self.remote_source_address.clone(),
            self.connection_id,
            self.mix_sender.clone(),
            adapter_fn,
            Arc::clone(&shutdown_notify),
        );

        let outbound_future = outbound::run_outbound(
            write_half,
            self.local_destination_address.clone(),
            self.remote_source_address.clone(),
            self.mix_receiver.take().unwrap(),
            self.connection_id,
            shutdown_notify,
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

        let read_half = inbound_result.unwrap();
        let (write_half, mix_receiver) = outbound_result.unwrap();

        self.socket = Some(write_half.reunite(read_half).unwrap());
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
