// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::events::DispatcherSender;
use crate::dkg::networking::codec::DkgCodec;
use crate::dkg::networking::message::{
    ErrorResponseMessage, NewDealingMessage, OffchainDkgMessage, RemoteDealingRequestMessage,
};
use crate::dkg::state::DkgState;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;

const DEFAULT_MAX_CONNECTION_DURATION: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Debug)]
pub(crate) struct ConnectionHandler {
    // connection cannot exist for more than this time
    max_connection_duration: Duration,
    dispatcher_sender: DispatcherSender,
    dkg_state: DkgState,
    conn: Framed<TcpStream, DkgCodec>,
    remote: SocketAddr,
}

impl ConnectionHandler {
    pub(crate) fn new(
        dispatcher_sender: DispatcherSender,
        dkg_state: DkgState,
        conn: TcpStream,
        remote: SocketAddr,
    ) -> Self {
        ConnectionHandler {
            max_connection_duration: DEFAULT_MAX_CONNECTION_DURATION,
            dispatcher_sender,
            dkg_state,
            remote,
            conn: Framed::new(conn, DkgCodec),
        }
    }

    async fn send_response(&mut self, response_message: OffchainDkgMessage) {
        self.conn.send(response_message).await;
    }

    async fn send_error_response(&mut self, id: Option<u64>, error: ErrorResponseMessage) {
        self.send_response(OffchainDkgMessage::new_error_response(id, error))
            .await
    }

    async fn handle_new_dealing(&self, id: u64, message: NewDealingMessage) {
        todo!()
    }

    async fn handle_remote_dealing_request(
        &mut self,
        id: u64,
        message: RemoteDealingRequestMessage,
    ) {
        // TODO: when somebody is reviewing this code, what's your opinion on accessing the DkgState here
        // vs keeping it slightly more consistent and dispatching an event to request the value from
        // something managing it instead?

        // personal note: once more parts are developed, I might change it myself before it even gets to the PR state

        let current_epoch = self.dkg_state.current_epoch().await;
        if current_epoch.id != message.epoch_id {
            return self
                .send_error_response(
                    Some(id),
                    ErrorResponseMessage::InvalidEpoch {
                        current: current_epoch.id,
                        requested: message.epoch_id,
                    },
                )
                .await;
        }

        let dealing = self.dkg_state.get_verified_dealing(message.dealer).await;

        self.send_response(todo!()).await;
        todo!()
    }

    async fn handle_request(&mut self, request: OffchainDkgMessage) {
        match request {
            OffchainDkgMessage::NewDealing { id, message } => {
                self.handle_new_dealing(id, message).await
            }
            OffchainDkgMessage::RemoteDealingRequest { id, message } => {
                self.handle_remote_dealing_request(id, message).await
            }
            OffchainDkgMessage::RemoteDealingResponse { id, .. } => {
                self.send_error_response(
                    Some(id),
                    ErrorResponseMessage::InvalidRequest {
                        typ: "RemoteDealingResponse".into(),
                    },
                )
                .await
            }
            OffchainDkgMessage::ErrorResponse { id, .. } => {
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

        let (is_dealer, epoch) = self.dkg_state.is_dealers_remote_address(self.remote).await;
        if !is_dealer {
            warn!(
                "Received a request from an unknown dealer - {}",
                self.remote
            );
            self.send_error_response(
                None,
                ErrorResponseMessage::UnknownDealer {
                    sender_address: self.remote,
                    epoch_id: epoch.id,
                },
            )
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
                None,
                ErrorResponseMessage::Timeout {
                    timeout: self.max_connection_duration,
                },
            )
            .await;
        }
    }
}
