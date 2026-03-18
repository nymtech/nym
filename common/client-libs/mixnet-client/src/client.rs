// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
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
use tokio::io::{AsyncRead, AsyncWrite};
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
    // Identifies the `ManagedConnection` task currently owning this entry; used
    // to ensure drop-time eviction only fires on the still-owning task.
    handle_token: Arc<()>,
}

impl ConnectionSender {
    fn new(channel: mpsc::Sender<FramedNymPacket>, handle_token: Arc<()>) -> Self {
        ConnectionSender {
            channel,
            current_reconnection_attempt: Arc::new(AtomicU32::new(0)),
            handle_token,
        }
    }
}

struct ManagedConnection {
    address: SocketAddr,
    noise_config: NoiseConfig,
    message_receiver: ReceiverStream<FramedNymPacket>,
    connection_timeout: Duration,
    current_reconnection: Arc<AtomicU32>,
    active_connections: ActiveConnections,
    handle_token: Arc<()>,
}

// Evicts the cache entry on task exit (only if still owned by this task).
// Without this, a stale `ConnectionSender` survives after the peer disconnects
// and the next outbound packet is silently swallowed by the dead TCP.
struct EvictOnDrop {
    active_connections: ActiveConnections,
    address: SocketAddr,
    handle_token: Arc<()>,
}

impl Drop for EvictOnDrop {
    fn drop(&mut self) {
        let address = self.address;
        let handle_token = &self.handle_token;
        self.active_connections.remove_if(&address, |_, sender| {
            Arc::ptr_eq(&sender.handle_token, handle_token)
        });
        trace!(
            peer = %address,
            "managed connection task exited; evicted owning cache entry"
        );
    }
}

impl ManagedConnection {
    fn new(
        address: SocketAddr,
        noise_config: NoiseConfig,
        message_receiver: mpsc::Receiver<FramedNymPacket>,
        connection_timeout: Duration,
        current_reconnection: Arc<AtomicU32>,
        active_connections: ActiveConnections,
        handle_token: Arc<()>,
    ) -> Self {
        ManagedConnection {
            address,
            noise_config,
            message_receiver: ReceiverStream::new(message_receiver),
            connection_timeout,
            current_reconnection,
            active_connections,
            handle_token,
        }
    }

    async fn run(self) {
        let address = self.address;
        let _evict_guard = EvictOnDrop {
            active_connections: self.active_connections,
            address,
            handle_token: self.handle_token,
        };

        let reconnection_attempt = self.current_reconnection.load(Ordering::Acquire);
        let connect_start = tokio::time::Instant::now();
        let connection_fut = TcpStream::connect(address);

        // 1. attempt to establish the connection with timeout
        let maybe_stream = match tokio::time::timeout(self.connection_timeout, connection_fut).await
        {
            Ok(stream) => stream,
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

        // 2. check if it actually succeeded
        let stream = match maybe_stream {
            Ok(stream) => stream,
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
        };

        let connect_ms = connect_start.elapsed().as_millis() as u64;
        debug!(
            peer = %address,
            connect_ms,
            "Managed to establish connection to {}", self.address
        );

        // 3. perform noise handshake (if applicable)
        let noise_start = tokio::time::Instant::now();
        let noise_stream = match upgrade_noise_initiator(stream, &self.noise_config).await {
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
        let conn = Framed::new(noise_stream, NymCodec);

        // 4. start handling the framed stream
        run_io_loop(conn, self.message_receiver, address).await;
    }
}

// The connection is unidirectional (send-only); we read from it solely to
// notice peer FIN/RST while idle so we can evict the cache entry before the
// next outbound send finds it stale.
async fn run_io_loop<T>(
    conn: Framed<T, NymCodec>,
    mut receiver: ReceiverStream<FramedNymPacket>,
    address: SocketAddr,
) where
    T: AsyncRead + AsyncWrite + Unpin,
{
    let (mut sink, mut stream) = conn.split();

    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    None => {
                        debug!(
                            peer = %address,
                            exit_reason = "peer_closed",
                            "peer closed mixnet connection to {address}"
                        );
                        break;
                    }
                    Some(Err(err)) => {
                        warn!(
                            event = "connection.read_error",
                            peer = %address,
                            error = %err,
                            exit_reason = "read_error",
                            "read error on mixnet connection to {address}: {err}"
                        );
                        break;
                    }
                    Some(Ok(_)) => {
                        trace!(
                            peer = %address,
                            "unexpected inbound packet on mixnet connection to {address}; discarding"
                        );
                    }
                }
            }
            outgoing = receiver.next() => {
                match outgoing {
                    None => {
                        debug!(
                            peer = %address,
                            exit_reason = "sender_dropped",
                            "connection manager to {address} finished"
                        );
                        break;
                    }
                    Some(packet) => {
                        if let Err(err) = sink.send(packet).await {
                            warn!(
                                event = "connection.forward_error",
                                peer = %address,
                                error = %err,
                                exit_reason = "forward_error",
                                "Failed to forward packet to {address}: {err}"
                            );
                            break;
                        }
                    }
                }
            }
        }
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

        // Ownership token for the task we're about to spawn; lets it tell
        // on exit whether the cache entry still names it.
        let handle_token = Arc::new(());

        // if we already tried to connect to `address` before, grab the current attempt count
        let current_reconnection_attempt =
            if let Some(mut existing) = self.active_connections.get_mut(&address) {
                existing.channel = sender;
                existing.handle_token = Arc::clone(&handle_token);
                Arc::clone(&existing.current_reconnection_attempt)
            } else {
                let new_entry = ConnectionSender::new(sender, Arc::clone(&handle_token));
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
        let active_connections = self.active_connections.clone();
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
                active_connections,
                handle_token,
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

        // TODO: optimisation for the future: rather than constantly using legacy encoding,
        // use the mix packet type / flags to pick encoding per packet
        let framed_packet =
            FramedNymPacket::from_mix_packet(packet, self.config.use_legacy_packet_encoding);

        let Some(sender) = self.active_connections.get_mut(&address) else {
            // there was never a connection to begin with
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
                    trace!(
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

    fn test_addr() -> SocketAddr {
        "127.0.0.1:1".parse().unwrap()
    }

    fn insert_with_token(
        active: &ActiveConnections,
        addr: SocketAddr,
        token: Arc<()>,
    ) -> mpsc::Receiver<FramedNymPacket> {
        let (tx, rx) = mpsc::channel(1);
        active.insert(addr, ConnectionSender::new(tx, token));
        rx
    }

    #[test]
    fn evict_on_drop_removes_entry_when_token_still_matches() {
        let active = ActiveConnections::default();
        let addr = test_addr();
        let token = Arc::new(());
        let _rx = insert_with_token(&active, addr, Arc::clone(&token));

        assert!(active.get(&addr).is_some());

        {
            let _guard = EvictOnDrop {
                active_connections: active.clone(),
                address: addr,
                handle_token: token,
            };
        }

        assert!(
            active.get(&addr).is_none(),
            "owning task's drop should evict the entry"
        );
    }

    #[test]
    fn evict_on_drop_preserves_entry_replaced_by_newer_make_connection() {
        // Simulates the race: old task's run() has returned, but before its
        // drop guard fires, a concurrent `make_connection` replaced the
        // entry's channel + handle_token with a fresh task's token.
        let active = ActiveConnections::default();
        let addr = test_addr();
        let old_token = Arc::new(());
        let new_token = Arc::new(());
        let _rx_new = insert_with_token(&active, addr, Arc::clone(&new_token));

        {
            let _guard = EvictOnDrop {
                active_connections: active.clone(),
                address: addr,
                handle_token: old_token,
            };
        }

        assert!(
            active.get(&addr).is_some(),
            "old task's drop must not clobber the newer entry"
        );
    }

    #[tokio::test]
    async fn io_loop_exits_when_peer_closes_idle_connection() {
        // The fix's second half: while no packets are flowing, peer FIN/RST
        // must still be observed so the cache entry can be evicted before the
        // next send finds it stale.
        let (a, b) = tokio::io::duplex(64);
        let conn = Framed::new(a, NymCodec);
        let (_tx, rx) = mpsc::channel(1);

        let task = tokio::spawn(run_io_loop(conn, ReceiverStream::new(rx), test_addr()));

        // Simulate peer closing both directions of the connection.
        drop(b);

        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("io_loop must notice peer close while idle")
            .expect("io_loop task must not panic");
    }

    #[tokio::test]
    async fn io_loop_exits_when_sender_dropped() {
        let (a, _b) = tokio::io::duplex(64);
        let conn = Framed::new(a, NymCodec);
        let (tx, rx) = mpsc::channel(1);

        let task = tokio::spawn(run_io_loop(conn, ReceiverStream::new(rx), test_addr()));

        drop(tx);

        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("io_loop must exit when the upstream sender is dropped")
            .expect("io_loop task must not panic");
    }
}
