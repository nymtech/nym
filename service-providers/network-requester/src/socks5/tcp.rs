// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::reply;
use crate::reply::MixnetMessage;
use client_connections::LaneQueueLengths;
use nym_task::TaskClient;
use proxy_helpers::connection_controller::ConnectionReceiver;
use proxy_helpers::proxy_runner::{MixProxySender, ProxyRunner};
use service_providers_common::interface::RequestVersion;
use socks5_requests::{ConnectionId, RemoteAddress, Socks5Request};
use std::io;
use tokio::net::TcpStream;

/// An outbound TCP connection between the Socks5 service provider, which makes
/// requests on behalf of users and returns the responses through
/// the mixnet.
#[derive(Debug)]
pub(crate) struct Connection {
    id: ConnectionId,
    address: RemoteAddress,
    conn: Option<TcpStream>,
    return_address: reply::MixnetAddress,
}

impl Connection {
    pub(crate) async fn new(
        id: ConnectionId,
        address: RemoteAddress,
        return_address: reply::MixnetAddress,
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
        remote_version: RequestVersion<Socks5Request>,
        mix_receiver: ConnectionReceiver,
        mix_sender: MixProxySender<MixnetMessage>,
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
            MixnetMessage::new_network_data_response_content(
                return_address.clone(),
                remote_version.clone(),
                conn_id,
                read_data,
                socket_closed,
            )
        })
        .await
        .into_inner();
        self.conn = Some(stream);
    }
}
