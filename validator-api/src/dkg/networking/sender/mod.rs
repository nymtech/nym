// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::dkg::networking::codec::DkgCodec;
use crate::dkg::networking::message::OffchainDkgMessage;
use futures::channel::mpsc;
use futures::{stream, SinkExt, StreamExt};
use log::{debug, info, trace, warn};
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;

// TODO: for now just leave them here and make it configurable with proper config later
const DEFAULT_CONCURRENCY: usize = 5;
const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_SEND_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) struct SendResponse {
    source: SocketAddr,
    response: Result<Option<OffchainDkgMessage>, DkgError>,
}

type FeedbackSender = mpsc::UnboundedSender<SendResponse>;

pub(crate) struct Broadcaster {
    addresses: Vec<SocketAddr>,
    concurrency_level: usize,
    connection_timeout: Duration,
    response_timeout: Duration,
    send_timeout: Duration,
}

impl Broadcaster {
    pub(crate) fn new(addresses: Vec<SocketAddr>) -> Self {
        Broadcaster {
            addresses,
            concurrency_level: DEFAULT_CONCURRENCY,
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT,
            send_timeout: DEFAULT_SEND_TIMEOUT,
        }
    }

    pub(crate) fn with_concurrency_level(mut self, concurrency_level: usize) -> Self {
        self.concurrency_level = concurrency_level;
        self
    }

    pub(crate) fn with_connection_timeout(mut self, connection_timeout: Duration) -> Self {
        self.connection_timeout = connection_timeout;
        self
    }

    pub(crate) fn with_response_timeout(mut self, response_timeout: Duration) -> Self {
        self.response_timeout = response_timeout;
        self
    }

    pub(crate) fn with_send_timeout(mut self, send_timeout: Duration) -> Self {
        self.send_timeout = send_timeout;
        self
    }

    pub(crate) fn set_addresses(&mut self, new_addresses: Vec<SocketAddr>) {
        self.addresses = new_addresses;
    }

    fn create_broadcast_configs(
        &self,
        message: OffchainDkgMessage,
        feedback_sender: Option<FeedbackSender>,
    ) -> Vec<BroadcastConfig> {
        self.addresses
            .iter()
            .map(|&address| BroadcastConfig {
                address,
                connection_timeout: self.connection_timeout,
                response_timeout: Some(self.response_timeout),
                send_timeout: self.send_timeout,
                feedback_sender: feedback_sender.clone(),
                message: message.clone(),
            })
            .collect()
    }

    pub(crate) async fn broadcast_with_feedback(
        &self,
        msg: OffchainDkgMessage,
    ) -> Result<(), DkgError> {
        if self.addresses.is_empty() {
            warn!("attempting to broadcast {} while no remotes are known", msg);
            return Ok(());
        }

        debug!("broadcasting {} to {} remotes", msg, self.addresses.len());
        let (feedback_tx, mut feedback_rx) = mpsc::unbounded();

        stream::iter(self.create_broadcast_configs(msg, Some(feedback_tx)))
            .for_each_concurrent(self.concurrency_level, |cfg| cfg.send())
            .await;

        let mut failures = 0;

        for _ in 0..self.addresses.len() {
            // we should have received exactly self.addresses number of responses
            // (they could be just Err failure responses, but should exist nonetheless)
            match feedback_rx.try_next() {
                Ok(Some(response)) => {
                    match response.response {
                        Err(err) => {
                            failures += 1;
                            warn!("we failed to broadcast to {} - {}", response.source, err)
                        }
                        Ok(Some(res)) => {
                            // TODO: figure out what to do with the replies exactly in the broadcast case...
                            if let OffchainDkgMessage::ErrorResponse { message, .. } = res {
                                warn!(
                                    "we received an error response from {} - {}",
                                    response.source, message
                                )
                            } else {
                                info!(
                                    "{} provided a non-error response to our broadcast! - {}",
                                    response.source, res
                                )
                            }
                        }
                        // the expected case
                        Ok(None) => debug!("{} didn't provide any reply", response.source),
                    }
                }
                Err(_) | Ok(None) => {
                    error!("somehow we received fewer feedback responses than sent messages")
                }
            }
        }

        // the channel should have been drained and all sender should have been dropped
        debug_assert!(matches!(feedback_rx.try_next(), Ok(None)));

        // if we failed to send to everyone, return an error, otherwise, assuming at least a single
        // receiver got our dealing, it can be gossiped through (assuming the problem was on our side,
        // i.e. the other receivers will actually want to receive the dealing)
        if failures == self.addresses.len() {
            error!("we failed to broadcast to every single receiver");
            return Err(DkgError::FullBroadcastFailure { total: failures });
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn broadcast(&self, msg: OffchainDkgMessage) {
        if self.addresses.is_empty() {
            warn!("attempting to broadcast {} while no remotes are known", msg);
            return;
        }

        debug!("broadcasting {} to {} remotes", msg, self.addresses.len());
        stream::iter(self.create_broadcast_configs(msg, None))
            .for_each_concurrent(self.concurrency_level, |cfg| cfg.send())
            .await
    }
}

// internal struct to have per-connection config on hand
struct BroadcastConfig {
    address: SocketAddr,
    connection_timeout: Duration,
    response_timeout: Option<Duration>,
    send_timeout: Duration,
    feedback_sender: Option<FeedbackSender>,
    message: OffchainDkgMessage,
}

impl BroadcastConfig {
    async fn send(self) {
        let response = send_message(
            self.address,
            &self.message,
            self.connection_timeout,
            self.send_timeout,
            self.response_timeout,
        )
        .await;
        if let Some(feedback_sender) = self.feedback_sender {
            // this can only fail if the receiver is disconnected which should never be the case
            // thus we can ignore the possible error
            let _ = feedback_sender.unbounded_send(SendResponse {
                source: self.address,
                response,
            });
        } else if let Err(err) = response {
            // if we're not forwarding feedback, at least emit a warning about the failure
            warn!(
                "failed to broadcast {} to {} - {}",
                self.message, self.address, err
            )
        }
    }
}

// this connection only exists for a single message
pub(crate) struct EphemeralConnection {
    conn: Framed<TcpStream, DkgCodec>,
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
        let framed_conn = Framed::new(conn, DkgCodec);
        Ok(Self { conn: framed_conn })
    }

    pub(crate) fn remote(&self) -> io::Result<SocketAddr> {
        self.conn.get_ref().peer_addr()
    }

    pub(crate) async fn send(
        &mut self,
        message: &OffchainDkgMessage,
        send_timeout: Duration,
        response_timeout: Option<Duration>,
    ) -> Result<Option<OffchainDkgMessage>, DkgError> {
        trace!("attempting to send to {}", self.remote()?);
        match timeout(send_timeout, self.conn.send(message)).await {
            Err(_timeout) => {
                return Err(DkgError::Networking(io::Error::new(
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
}

pub(crate) async fn send_message(
    address: SocketAddr,
    message: &OffchainDkgMessage,
    connection_timeout: Duration,
    send_timeout: Duration,
    response_timeout: Option<Duration>,
) -> Result<Option<OffchainDkgMessage>, DkgError> {
    let mut conn = EphemeralConnection::connect(address, connection_timeout).await?;
    conn.send(message, send_timeout, response_timeout).await
}
