// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::connection_controller::ConnectionReceiver;
use crate::ordered_sender::OrderedMessageSender;
use nym_socks5_requests::{ConnectionId, SocketData};
use nym_task::connections::LaneQueueLengths;
use nym_task::TaskClient;
use tokio_util::sync::PollSender;
use std::fmt::Debug;
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpStream, sync::Notify};

mod inbound;
mod outbound;

// TODO: make this configurable
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

// Send empty keepalive messages regurarly to keep the connection alive. This should be smaller
// than [`MIX_TTL`].
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(60);

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

pub type MixProxySender<S> = PollSender<S>;
pub type MixProxyReader<S> = tokio::sync::mpsc::Receiver<S>;

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
    lane_queue_lengths: Option<LaneQueueLengths>,

    available_plaintext_per_mix_packet: usize,

    // Listens to shutdown commands from higher up
    shutdown_listener: TaskClient,
}

impl<S> ProxyRunner<S>
where
    S: Debug + Send + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        socket: TcpStream,
        local_destination_address: String, // addresses are provided for better logging
        remote_source_address: String,
        mix_receiver: ConnectionReceiver,
        mix_sender: MixProxySender<S>,
        available_plaintext_per_mix_packet: usize,
        connection_id: ConnectionId,
        lane_queue_lengths: Option<LaneQueueLengths>,
        shutdown_listener: TaskClient,
    ) -> Self {
        ProxyRunner {
            mix_receiver: Some(mix_receiver),
            mix_sender,
            socket: Some(socket),
            local_destination_address,
            remote_source_address,
            connection_id,
            lane_queue_lengths,
            available_plaintext_per_mix_packet,
            shutdown_listener,
        }
    }

    // The `adapter_fn` is used to transform whatever was read into appropriate
    // request/response as required by entity running particular side of the proxy.
    pub async fn run<F>(mut self, adapter_fn: F) -> Self
    where
        F: Fn(SocketData) -> S + Send + Sync + 'static,
    {
        let (read_half, write_half) = self.socket.take().unwrap().into_split();
        let shutdown_notify = Arc::new(Notify::new());

        // should run until either inbound closes or is notified from outbound
        let ordered_sender = OrderedMessageSender::new(
            self.local_destination_address.clone(),
            self.remote_source_address.clone(),
            self.connection_id,
            self.mix_sender.clone(),
            adapter_fn,
        );
        let inbound_future = inbound::run_inbound(
            read_half,
            ordered_sender,
            self.connection_id,
            self.available_plaintext_per_mix_packet,
            Arc::clone(&shutdown_notify),
            self.lane_queue_lengths.clone(),
            self.shutdown_listener.clone(),
        );

        let outbound_future = outbound::run_outbound(
            write_half,
            self.local_destination_address.clone(),
            self.remote_source_address.clone(),
            self.mix_receiver.take().unwrap(),
            self.connection_id,
            shutdown_notify,
            self.shutdown_listener.clone(),
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
        self.shutdown_listener.mark_as_success();
        (
            self.socket.take().unwrap(),
            self.mix_receiver.take().unwrap(),
        )
    }
}
