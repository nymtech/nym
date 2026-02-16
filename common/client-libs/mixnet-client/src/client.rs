// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use futures::StreamExt;
use nym_noise::config::NoiseConfig;
use nym_noise::upgrade_noise_initiator;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::framing::codec::NymCodec;
use nym_sphinx::framing::packet::FramedNymPacket;
use std::io;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::sleep;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::codec::Framed;
use tracing::*;

#[derive(Clone, Copy)]
pub struct Config {
    pub initial_reconnection_backoff: Duration,
    pub maximum_reconnection_backoff: Duration,
    pub initial_connection_timeout: Duration,
    pub maximum_connection_buffer_size: usize,
    pub use_legacy_packet_encoding: bool,
}

impl Config {
    pub fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
        maximum_connection_buffer_size: usize,
        use_legacy_packet_encoding: bool,
    ) -> Self {
        Config {
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
            maximum_connection_buffer_size,
            use_legacy_packet_encoding,
        }
    }
}

pub trait SendWithoutResponse {
    // Without response in this context means we will not listen for anything we might get back (not
    // that we should get anything), including any possible io errors
    fn send_without_response(&self, packet: MixPacket) -> io::Result<()>;
}

pub struct Client {
    active_connections: ActiveConnections,
    noise_config: NoiseConfig,
    connections_count: Arc<AtomicUsize>,
    config: Config,
}

#[derive(Default, Clone)]
pub struct ActiveConnections {
    inner: Arc<DashMap<SocketAddr, ConnectionSender>>,
}

impl ActiveConnections {
    pub fn pending_packets(&self) -> usize {
        self.inner
            .iter()
            .map(|sender| {
                let max_capacity = sender.channel.max_capacity();
                let capacity = sender.channel.capacity();
                max_capacity - capacity
            })
            .sum()
    }
}

impl Deref for ActiveConnections {
    type Target = DashMap<SocketAddr, ConnectionSender>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct ConnectionSender {
    channel: mpsc::Sender<FramedNymPacket>,
    current_reconnection_attempt: Arc<AtomicU32>,
}

impl ConnectionSender {
    fn new(channel: mpsc::Sender<FramedNymPacket>) -> Self {
        ConnectionSender {
            channel,
            current_reconnection_attempt: Arc::new(AtomicU32::new(0)),
        }
    }
}

struct ManagedConnection {
    address: SocketAddr,
    noise_config: NoiseConfig,
    message_receiver: ReceiverStream<FramedNymPacket>,
    connection_timeout: Duration,
    current_reconnection: Arc<AtomicU32>,
}

impl ManagedConnection {
    fn new(
        address: SocketAddr,
        noise_config: NoiseConfig,
        message_receiver: mpsc::Receiver<FramedNymPacket>,
        connection_timeout: Duration,
        current_reconnection: Arc<AtomicU32>,
    ) -> Self {
        ManagedConnection {
            address,
            noise_config,
            message_receiver: ReceiverStream::new(message_receiver),
            connection_timeout,
            current_reconnection,
        }
    }

    async fn run(self) {
        let address = self.address;
        let reconnection_attempt = self.current_reconnection.load(Ordering::Acquire);
        let connect_start = tokio::time::Instant::now();
        let connection_fut = TcpStream::connect(address);

        let conn = match tokio::time::timeout(self.connection_timeout, connection_fut).await {
            Ok(stream_res) => match stream_res {
                Ok(stream) => {
                    let connect_ms = connect_start.elapsed().as_millis() as u64;
                    debug!(
                        peer = %address,
                        connect_ms,
                        "Managed to establish connection to {}", self.address
                    );

                    let noise_start = tokio::time::Instant::now();
                    let noise_stream =
                        match upgrade_noise_initiator(stream, &self.noise_config).await {
                            Ok(noise_stream) => noise_stream,
                            Err(err) => {
                                let noise_handshake_ms = noise_start.elapsed().as_millis() as u64;
                                warn!(
                                    event = "connection.failed.noise",
                                    peer = %address,
                                    error = %err,
                                    connect_ms,
                                    noise_handshake_ms,
                                    reconnection_attempt,
                                    exit_reason = "noise_error",
                                    "Failed to perform Noise initiator handshake with {address}"
                                );
                                self.current_reconnection.fetch_add(1, Ordering::SeqCst);
                                return;
                            }
                        };
                    let noise_handshake_ms = noise_start.elapsed().as_millis() as u64;
                    self.current_reconnection.store(0, Ordering::Release);
                    debug!(
                        peer = %address,
                        connect_ms,
                        noise_handshake_ms,
                        "Noise initiator handshake completed for {:?}", address
                    );
                    Framed::new(noise_stream, NymCodec)
                }
                Err(err) => {
                    let connect_ms = connect_start.elapsed().as_millis() as u64;
                    warn!(
                        event = "connection.failed.connect",
                        peer = %address,
                        error = %err,
                        connect_ms,
                        reconnection_attempt,
                        exit_reason = "connect_error",
                        "failed to establish connection to {address}"
                    );
                    return;
                }
            },
            Err(_) => {
                let connect_ms = connect_start.elapsed().as_millis() as u64;
                warn!(
                    event = "connection.failed.timeout",
                    peer = %address,
                    timeout_ms = self.connection_timeout.as_millis() as u64,
                    connect_ms,
                    reconnection_attempt,
                    exit_reason = "timeout",
                    "failed to connect to {address} within {:?}",
                    self.connection_timeout
                );
                self.current_reconnection.fetch_add(1, Ordering::SeqCst);
                return;
            }
        };

        if let Err(err) = self.message_receiver.map(Ok).forward(conn).await {
            warn!(
                event = "connection.forward_error",
                peer = %address,
                error = %err,
                exit_reason = "forward_error",
                "Failed to forward packets to {address}: {err}"
            );
        }

        debug!(
            peer = %address,
            exit_reason = "sender_dropped",
            "connection manager to {address} finished"
        );
    }
}

impl Client {
    pub fn new(
        config: Config,
        noise_config: NoiseConfig,
        connections_count: Arc<AtomicUsize>,
    ) -> Client {
        Client {
            active_connections: Default::default(),
            noise_config,
            connections_count,
            config,
        }
    }

    pub fn active_connections(&self) -> ActiveConnections {
        self.active_connections.clone()
    }

    /// If we're trying to reconnect, determine how long we should wait.
    fn determine_backoff(&self, current_attempt: u32) -> Option<Duration> {
        if current_attempt == 0 {
            None
        } else {
            let exp = 2_u32.checked_pow(current_attempt);
            let backoff = exp
                .and_then(|exp| self.config.initial_reconnection_backoff.checked_mul(exp))
                .unwrap_or(self.config.maximum_reconnection_backoff);

            Some(std::cmp::min(
                backoff,
                self.config.maximum_reconnection_backoff,
            ))
        }
    }

    fn make_connection(&self, address: SocketAddr, pending_packet: FramedNymPacket) {
        let (sender, receiver) = mpsc::channel(self.config.maximum_connection_buffer_size);

        // this CAN'T fail because we just created the channel which has a non-zero capacity
        if self.config.maximum_connection_buffer_size > 0 {
            sender.try_send(pending_packet).unwrap();
        }

        // if we already tried to connect to `address` before, grab the current attempt count
        let current_reconnection_attempt =
            if let Some(mut existing) = self.active_connections.get_mut(&address) {
                existing.channel = sender;
                Arc::clone(&existing.current_reconnection_attempt)
            } else {
                let new_entry = ConnectionSender::new(sender);
                let current_attempt = Arc::clone(&new_entry.current_reconnection_attempt);
                self.active_connections.insert(address, new_entry);
                current_attempt
            };

        // load the actual value.
        let reconnection_attempt = current_reconnection_attempt.load(Ordering::Acquire);
        let backoff = self.determine_backoff(reconnection_attempt);

        // copy the value before moving into another task
        let initial_connection_timeout = self.config.initial_connection_timeout;

        let connections_count = self.connections_count.clone();
        let noise_config = self.noise_config.clone();
        tokio::spawn(async move {
            // before executing the manager, wait for what was specified, if anything
            if let Some(backoff) = backoff {
                trace!("waiting for {:?} before attempting connection", backoff);
                sleep(backoff).await;
            }

            connections_count.fetch_add(1, Ordering::SeqCst);
            ManagedConnection::new(
                address,
                noise_config,
                receiver,
                initial_connection_timeout,
                current_reconnection_attempt,
            )
            .run()
            .await;
            connections_count.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

impl SendWithoutResponse for Client {
    fn send_without_response(&self, packet: MixPacket) -> io::Result<()> {
        let address = packet.next_hop_address();
        trace!("Sending packet to {address}");

        let framed_packet =
            FramedNymPacket::from_mix_packet(packet, self.config.use_legacy_packet_encoding);

        let Some(sender) = self.active_connections.get_mut(&address) else {
            debug!(
                event = "mixclient.try_send",
                peer = %address,
                result = "not_connected",
                "establishing initial connection to {address}"
            );
            self.make_connection(address, framed_packet);
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "connection is in progress",
            ));
        };

        let channel_capacity = sender.channel.max_capacity();
        let channel_available = sender.channel.capacity();
        let channel_used = channel_capacity - channel_available;

        let sending_res = sender.channel.try_send(framed_packet);
        drop(sender);

        sending_res.map_err(|err| {
            match err {
                TrySendError::Full(_) => {
                    warn!(
                        event = "mixclient.try_send",
                        peer = %address,
                        result = "full_dropped",
                        channel_capacity,
                        channel_used,
                        "dropping packet: connection buffer to {address} is full ({channel_used}/{channel_capacity})"
                    );
                    io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "connection queue is full",
                    )
                }
                TrySendError::Closed(dropped) => {
                    debug!(
                        event = "mixclient.try_send",
                        peer = %address,
                        result = "closed_reconnecting",
                        channel_capacity,
                        channel_used,
                        "connection to {address} dead, attempting re-establishment"
                    );
                    self.make_connection(address, dropped);
                    io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "reconnection attempt is in progress",
                    )
                }
            }
        } )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::x25519;
    use nym_noise::config::NoiseNetworkView;
    use rand::rngs::OsRng;

    fn dummy_client() -> Client {
        let mut rng = OsRng; //for test only, so we don't care if rng source isn't crypto grade
        Client::new(
            Config {
                initial_reconnection_backoff: Duration::from_millis(10_000),
                maximum_reconnection_backoff: Duration::from_millis(300_000),
                initial_connection_timeout: Duration::from_millis(1_500),
                maximum_connection_buffer_size: 128,
                use_legacy_packet_encoding: false,
            },
            NoiseConfig::new(
                Arc::new(x25519::KeyPair::new(&mut rng)),
                NoiseNetworkView::new_empty(),
                Duration::from_millis(1_500),
            ),
            Default::default(),
        )
    }

    #[test]
    fn determining_backoff_works_regardless_of_attempt() {
        let client = dummy_client();
        assert!(client.determine_backoff(0).is_none());
        assert!(client.determine_backoff(1).is_some());
        assert!(client.determine_backoff(2).is_some());
        assert_eq!(
            client.determine_backoff(16).unwrap(),
            client.config.maximum_reconnection_backoff
        );
        assert_eq!(
            client.determine_backoff(32).unwrap(),
            client.config.maximum_reconnection_backoff
        );
        assert_eq!(
            client.determine_backoff(1024).unwrap(),
            client.config.maximum_reconnection_backoff
        );
        assert_eq!(
            client.determine_backoff(65536).unwrap(),
            client.config.maximum_reconnection_backoff
        );
        assert_eq!(
            client.determine_backoff(u32::MAX).unwrap(),
            client.config.maximum_reconnection_backoff
        );
    }
}
