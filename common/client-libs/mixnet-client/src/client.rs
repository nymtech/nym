// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nymsphinx::framing::codec::SphinxCodec;
use nymsphinx::framing::packet::FramedSphinxPacket;
use nymsphinx::params::PacketMode;
use nymsphinx::{addressing::nodes::NymNodeRoutingAddress, SphinxPacket};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_util::codec::Framed;

pub struct Config {
    initial_reconnection_backoff: Duration,
    maximum_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
    maximum_connection_buffer_size: usize,
}

impl Config {
    pub fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
        maximum_connection_buffer_size: usize,
    ) -> Self {
        Config {
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
            maximum_connection_buffer_size,
        }
    }
}

pub trait SendWithoutResponse {
    // Without response in this context means we will not listen for anything we might get back (not
    // that we should get anything), including any possible io errors
    fn send_without_response(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: SphinxPacket,
        packet_mode: PacketMode,
    ) -> io::Result<()>;
}

pub struct Client {
    conn_new: HashMap<NymNodeRoutingAddress, ConnectionSender>,
    config: Config,
}

struct ConnectionSender {
    channel: mpsc::Sender<FramedSphinxPacket>,
    current_reconnection_attempt: Arc<AtomicU32>,
}

impl ConnectionSender {
    fn new(channel: mpsc::Sender<FramedSphinxPacket>) -> Self {
        ConnectionSender {
            channel,
            current_reconnection_attempt: Arc::new(AtomicU32::new(0)),
        }
    }
}

impl Client {
    pub fn new(config: Config) -> Client {
        Client {
            conn_new: HashMap::new(),
            config,
        }
    }

    async fn manage_connection(
        address: SocketAddr,
        receiver: mpsc::Receiver<FramedSphinxPacket>,
        connection_timeout: Duration,
        current_reconnection: &AtomicU32,
    ) {
        let connection_fut = TcpStream::connect(address);

        let conn = match tokio::time::timeout(connection_timeout, connection_fut).await {
            Ok(stream_res) => match stream_res {
                Ok(stream) => {
                    debug!("Managed to establish connection to {}", address);
                    // if we managed to connect, reset the reconnection count (whatever it might have been)
                    current_reconnection.store(0, Ordering::Release);
                    Framed::new(stream, SphinxCodec)
                }
                Err(err) => {
                    debug!(
                        "failed to establish connection to {} (err: {})",
                        address, err
                    );
                    return;
                }
            },
            Err(_) => {
                debug!(
                    "failed to connect to {} within {:?}",
                    address, connection_timeout
                );

                // we failed to connect - increase reconnection attempt
                current_reconnection.fetch_add(1, Ordering::SeqCst);
                return;
            }
        };

        // Take whatever the receiver channel produces and put it on the connection.
        // We could have as well used conn.send_all(receiver.map(Ok)), but considering we don't care
        // about neither receiver nor the connection, it doesn't matter which one gets consumed
        if let Err(err) = receiver.map(Ok).forward(conn).await {
            warn!("Failed to forward packets to {} - {:?}", address, err);
        }

        debug!(
            "connection manager to {} is finished. Either the connection failed or mixnet client got dropped",
            address
        );
    }

    /// If we're trying to reconnect, determine how long we should wait.
    fn determine_backoff(&self, current_attempt: u32) -> Option<Duration> {
        if current_attempt == 0 {
            None
        } else {
            // according to https://github.com/tokio-rs/tokio/issues/1953 there's an undocumented
            // limit of tokio delay of about 2 years.
            // let's ensure our delay is always on a sane side of being maximum 1 hour.
            let maximum_sane_delay = Duration::from_secs(60 * 60);

            Some(std::cmp::min(
                maximum_sane_delay,
                std::cmp::min(
                    self.config
                        .initial_reconnection_backoff
                        .checked_mul(2_u32.pow(current_attempt))
                        .unwrap_or(self.config.maximum_reconnection_backoff),
                    self.config.maximum_reconnection_backoff,
                ),
            ))
        }
    }

    fn make_connection(
        &mut self,
        address: NymNodeRoutingAddress,
        pending_packet: FramedSphinxPacket,
    ) {
        let (mut sender, receiver) = mpsc::channel(self.config.maximum_connection_buffer_size);

        // this CAN'T fail because we just created the channel which has a non-zero capacity
        if self.config.maximum_connection_buffer_size > 0 {
            sender.try_send(pending_packet).unwrap();
        }

        // if we already tried to connect to `address` before, grab the current attempt count
        let current_reconnection_attempt = if let Some(existing) = self.conn_new.get_mut(&address) {
            existing.channel = sender;
            Arc::clone(&existing.current_reconnection_attempt)
        } else {
            let new_entry = ConnectionSender::new(sender);
            let current_attempt = Arc::clone(&new_entry.current_reconnection_attempt);
            self.conn_new.insert(address, new_entry);
            current_attempt
        };

        // load the actual value.
        let reconnection_attempt = current_reconnection_attempt.load(Ordering::Acquire);
        let backoff = self.determine_backoff(reconnection_attempt);

        // copy the value before moving into another task
        let initial_connection_timeout = self.config.initial_connection_timeout;

        tokio::spawn(async move {
            // before executing the manager, wait for what was specified, if anything
            if let Some(backoff) = backoff {
                trace!("waiting for {:?} before attempting connection", backoff);
                sleep(backoff).await;
            }

            Self::manage_connection(
                address.into(),
                receiver,
                initial_connection_timeout,
                &*current_reconnection_attempt,
            )
            .await
        });
    }
}

impl SendWithoutResponse for Client {
    fn send_without_response(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: SphinxPacket,
        packet_mode: PacketMode,
    ) -> io::Result<()> {
        trace!("Sending packet to {:?}", address);
        let framed_packet = FramedSphinxPacket::new(packet, packet_mode);

        if let Some(sender) = self.conn_new.get_mut(&address) {
            if let Err(err) = sender.channel.try_send(framed_packet) {
                if err.is_full() {
                    debug!("Connection to {} seems to not be able to handle all the traffic - dropping the current packet", address);
                    // it's not a 'big' error, but we did not manage to send the packet
                    // if the queue is full, we can't really do anything but to drop the packet
                    Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "connection queue is full",
                    ))
                } else if err.is_disconnected() {
                    debug!(
                        "Connection to {} seems to be dead. attempting to re-establish it...",
                        address
                    );
                    // it's not a 'big' error, but we did not manage to send the packet, but queue
                    // it up to send it as soon as the connection is re-established
                    self.make_connection(address, err.into_inner());
                    Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "reconnection attempt is in progress",
                    ))
                } else {
                    // this can't really happen, but let's safe-guard against it in case something changes in futures library
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        "unknown connection buffer error",
                    ))
                }
            } else {
                Ok(())
            }
        } else {
            // there was never a connection to begin with
            debug!("establishing initial connection to {}", address);
            // it's not a 'big' error, but we did not manage to send the packet, but queue the packet
            // for sending for as soon as the connection is created
            self.make_connection(address, framed_packet);
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "connection is in progress",
            ))
        }
    }
}
