// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::networking::codec::DkgCodec;
use crate::dkg::networking::message::{ErrorReason, OffchainDkgMessage};
use crate::dkg::state::DkgState;
use futures::StreamExt;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;

const DEFAULT_MAX_CONNECTION_DURATION: Duration = Duration::new(2 * 60, 0);

#[derive(Debug)]
pub(crate) struct ConnectionHandler {
    // connection cannot exist for more than this time
    max_connection_duration: Duration,
    dkg_state: DkgState,
    conn: Framed<TcpStream, DkgCodec>,
    remote: SocketAddr,
}

impl ConnectionHandler {
    pub(crate) fn new(dkg_state: DkgState, conn: TcpStream, remote: SocketAddr) -> Self {
        ConnectionHandler {
            max_connection_duration: DEFAULT_MAX_CONNECTION_DURATION,
            dkg_state,
            remote,
            conn: Framed::new(conn, DkgCodec),
        }
    }

    async fn send_error_response<S: Into<String>>(
        &mut self,
        error: ErrorReason,
        additional_info: Option<S>,
    ) {
        //
    }

    async fn _handle_connection(&mut self) {
        debug!("Starting connection handler for {}", self.remote);

        if !self.dkg_state.is_dealers_remote_address(self.remote).await {
            let msg = format!(
                "{} is not a socket address of any known dealer. Closing the connection.",
                self.remote
            );
            warn!("{}", msg);
            self.send_error_response(ErrorReason::UnknownDealer, Some(msg))
                .await;
            return;
        }

        while let Some(framed_dkg_request) = self.conn.next().await {
            match framed_dkg_request {
                Ok(framed_dkg_request) => {
                    todo!("handle packet")
                }
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

    pub(crate) async fn handle_connection(mut self) {
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
                ErrorReason::Timeout,
                Some(format!(
                    "could not resolve connection within {:?}",
                    self.max_connection_duration
                )),
            )
            .await;
        }
    }

    async fn handle_request(&self, request: OffchainDkgMessage) {
        match request {
            OffchainDkgMessage::NewDealing { .. } => {}
            OffchainDkgMessage::RemoteDealingRequest { .. } => {}
            OffchainDkgMessage::RemoteDealingResponse { .. } => {}
            OffchainDkgMessage::ErrorResponse { .. } => {}
        }
    }
}
