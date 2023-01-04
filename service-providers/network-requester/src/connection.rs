// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::core::ReturnAddress;
use client_connections::LaneQueueLengths;
use proxy_helpers::connection_controller::ConnectionReceiver;
use proxy_helpers::proxy_runner::{MixProxySender, ProxyRunner};
use socks5_requests::{ConnectionId, Message as Socks5Message, RemoteAddress, Response};
use std::io;
use task::TaskClient;
use tokio::net::TcpStream;

/// A TCP connection between the Socks5 service provider, which makes
/// outbound requests on behalf of users and returns the responses through
/// the mixnet.
#[derive(Debug)]
pub(crate) struct Connection {
    id: ConnectionId,
    address: RemoteAddress,
    conn: Option<TcpStream>,
    return_address: ReturnAddress,
}

impl Connection {
    pub(crate) async fn new(
        id: ConnectionId,
        address: RemoteAddress,
        return_address: ReturnAddress,
    ) -> io::Result<Self> {
        let conn = TcpStream::connect(&address).await?;

        Ok(Connection {
            id,
            address,
            conn: Some(conn),
            return_address,
        })
    }

    pub(crate) async fn run_proxy(
        &mut self,
        mix_receiver: ConnectionReceiver,
        mix_sender: MixProxySender<(Socks5Message, ReturnAddress)>,
        lane_queue_lengths: LaneQueueLengths,
        shutdown: TaskClient,
    ) {
        let stream = self.conn.take().unwrap();
        let remote_source_address = "???".to_string(); // we don't know ip address of requester
        let connection_id = self.id;
        let return_address = self.return_address.clone();
        let (stream, _) = ProxyRunner::new(
            stream,
            self.address.clone(),
            remote_source_address,
            mix_receiver,
            mix_sender,
            connection_id,
            Some(lane_queue_lengths),
            shutdown,
        )
        .run(move |conn_id, read_data, socket_closed| {
            (
                Socks5Message::Response(Response::new(conn_id, read_data, socket_closed)),
                return_address.clone(),
            )
        })
        .await
        .into_inner();
        self.conn = Some(stream);
    }
}
