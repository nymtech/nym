// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures::channel::mpsc;
use futures::SinkExt;
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
use tokio::stream::StreamExt;
use tokio::time::delay_for;
use tokio_util::codec::Framed;

pub struct Config {
    initial_reconnection_backoff: Duration,
    maximum_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
}

impl Config {
    pub fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
    ) -> Self {
        Config {
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
        }
    }
}

const MAX_CONN_BUF: usize = 32;

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
        mut receiver: mpsc::Receiver<FramedSphinxPacket>,
        connection_timeout: Duration,
        current_reconnection: &AtomicU32,
    ) -> io::Result<()> {
        let mut conn = match std::net::TcpStream::connect_timeout(&address, connection_timeout) {
            Ok(stream) => {
                let tokio_stream = tokio::net::TcpStream::from_std(stream).unwrap();
                debug!("Managed to establish connection to {}", address);
                // if we managed to connect, reset the reconnection count (whatever it might have been)
                current_reconnection.store(0, Ordering::Release);

                Framed::new(tokio_stream, SphinxCodec)
            }
            Err(err) => {
                debug!(
                    "failed to connect to {} within {:?}",
                    address, connection_timeout
                );

                // we failed to connect - increase reconnection attempt
                current_reconnection.fetch_add(1, Ordering::SeqCst);

                return Err(err);
            }
        };

        while let Some(packet) = receiver.next().await {
            if let Err(err) = conn.send(packet).await {
                // I've put this as a warning rather than debug because this implies we managed
                // to connect to this destination but it failed later
                warn!("Failed to forward packet to {} - {:?}", address, err);
                // there's no point in draining the channel, it's incredibly unlikely further
                // messages might succeed
                break;
            } else {
                trace!("managed to forward packet to {}", address)
            }
        }

        // if we got here it means the mixnet client was dropped
        debug!(
            "connection manager to {} is finished. Presumably mixnet client got dropped",
            address
        );
        Ok(())
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
                        .unwrap_or_else(|| self.config.maximum_reconnection_backoff),
                    self.config.maximum_reconnection_backoff,
                ),
            ))
        }
    }

    fn make_connection(&mut self, address: NymNodeRoutingAddress) {
        let (sender, receiver) = mpsc::channel(MAX_CONN_BUF);

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
                delay_for(backoff).await;
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

    // without response in this context means we will not listen for anything we might get back
    // (not that we should get anything), including any possible io errors
    pub fn send_without_response(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: SphinxPacket,
        packet_mode: PacketMode,
    ) -> io::Result<()> {
        trace!("Sending packet to {:?}", address);

        if let Some(sender) = self.conn_new.get_mut(&address) {
            let framed_packet = FramedSphinxPacket::new(packet, packet_mode);
            if let Err(err) = sender.channel.try_send(framed_packet) {
                if err.is_full() {
                    debug!("Connection to {} seems to not be able to handle all the traffic - dropping the current packet", address);
                    // it's not a 'big' error, but we did not manage to send  the packet
                    Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "connection queue is full",
                    ))
                } else if err.is_disconnected() {
                    warn!(
                        "Connection to {} seems to be dead. attempting to re-establish it...",
                        address
                    );
                    self.make_connection(address);
                    // it's not a 'big' error, but we did not manage to send the packet
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
            self.make_connection(address);
            // it's not a 'big' error, but we did not manage to send the packet
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "connection is in progress",
            ))
        }
    }
}
