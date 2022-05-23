// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::networking::codec::OffchainCodec;
use crate::networking::error::NetworkingError;
use crate::networking::message::OffchainMessage;
use crate::networking::sender::ConnectionConfig;
use futures::{SinkExt, StreamExt};
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;

// this connection only exists for a single message
pub(crate) struct EphemeralConnection {
    remote: SocketAddr,
    conn: Framed<TcpStream, OffchainCodec>,
}

impl EphemeralConnection {
    pub(crate) async fn connect(
        address: SocketAddr,
        connection_timeout: Duration,
    ) -> io::Result<Self> {
        trace!("attempting to connect to {}", address);
        let conn = match timeout(connection_timeout, TcpStream::connect(address)).await {
            Err(_timeout) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("timed out while attempting to send message to {}", address),
                ))
            }
            Ok(conn_res) => conn_res?,
        };
        let framed_conn = Framed::new(conn, OffchainCodec);
        Ok(Self {
            remote: address,
            conn: framed_conn,
        })
    }

    pub(crate) fn remote(&self) -> SocketAddr {
        self.remote
    }

    async fn send(
        &mut self,
        message: &OffchainMessage,
        send_timeout: Duration,
        response_timeout: Option<Duration>,
    ) -> Result<Option<OffchainMessage>, NetworkingError> {
        trace!("attempting to send to {}", self.remote);
        match timeout(send_timeout, self.conn.send(message)).await {
            Err(_timeout) => {
                return Err(NetworkingError::Io(io::Error::new(
                    io::ErrorKind::Other,
                    "timed out while attempting to send message",
                )))
            }
            Ok(res) => res?,
        }
        if let Some(response_timeout) = response_timeout {
            match timeout(response_timeout, self.conn.next()).await {
                Err(_elapsed) => Ok(None),
                Ok(response) => response.transpose(),
            }
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn connect_and_send(
        address: SocketAddr,
        cfg: ConnectionConfig,
        message: &OffchainMessage,
    ) -> Result<Option<OffchainMessage>, NetworkingError> {
        let mut conn = EphemeralConnection::connect(address, cfg.connection_timeout).await?;
        conn.send(message, cfg.send_timeout, cfg.response_timeout)
            .await
    }
}
