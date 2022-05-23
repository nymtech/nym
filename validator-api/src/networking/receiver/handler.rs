// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::networking::codec::OffchainCodec;
use crate::networking::message::{ErrorResponseMessage, OffchainMessage};
// use crate::dkg::state::StateAccessor;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;

const DEFAULT_MAX_CONNECTION_DURATION: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Debug)]
pub struct ConnectionHandler {
    // connection cannot exist for more than this time
    max_connection_duration: Duration,
    // state_accessor: StateAccessor,
    conn: Framed<TcpStream, OffchainCodec>,
    remote: SocketAddr,
}

impl ConnectionHandler {
    pub(crate) fn new(conn: TcpStream, remote: SocketAddr) -> Self {
        ConnectionHandler {
            max_connection_duration: DEFAULT_MAX_CONNECTION_DURATION,
            remote,
            conn: Framed::new(conn, OffchainCodec),
        }
    }

    async fn send_response(&mut self, response_message: OffchainMessage) {
        if let Err(err) = self.conn.send(&response_message).await {
            warn!("Failed to send response back to {} - {}", self.remote, err)
        }
    }

    async fn send_error_response(&mut self, id: Option<u64>, error: ErrorResponseMessage) {
        self.send_response(OffchainMessage::new_error_response(id, error))
            .await
    }

    async fn handle_request(&mut self, request: OffchainMessage) {
        match request {
            OffchainMessage::ErrorResponse { id, .. } => {
                self.send_error_response(
                    id,
                    ErrorResponseMessage::InvalidRequest {
                        typ: "ErrorResponse".into(),
                    },
                )
                .await
            }
        }
    }

    async fn _handle_connection(&mut self) {
        debug!("Starting connection handler for {}", self.remote);

        while let Some(framed_dkg_request) = self.conn.next().await {
            trace!("received new message from {}", self.remote);
            match framed_dkg_request {
                Ok(framed_dkg_request) => self.handle_request(framed_dkg_request).await,
                Err(err) => {
                    warn!(
                        "The socket connection got corrupted with error: {:?}. Closing the socket",
                        err
                    );
                    break;
                }
            }
        }

        debug!("Closing connection from {}", self.remote);
    }

    pub async fn handle_connection(mut self) {
        let remote = self.remote;
        if timeout(self.max_connection_duration, self._handle_connection())
            .await
            .is_err()
        {
            warn!(
                "we timed out while trying to resolve connection from {}",
                remote
            );
            self.send_error_response(
                None,
                ErrorResponseMessage::Timeout {
                    timeout: self.max_connection_duration,
                },
            )
            .await;
        }
    }
}
