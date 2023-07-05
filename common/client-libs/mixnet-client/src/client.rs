// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::BytesMut;
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx::framing::codec::NymCodec;
use nym_sphinx::framing::packet::FramedNymPacket;
use nym_sphinx::params::PacketType;
use nym_sphinx::NymPacket;
use quinn::{Connection, Endpoint};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::sleep;
use tokio_util::codec::{Encoder, Framed};
use tokio_util::udp::UdpFramed;

pub struct Config {
    initial_reconnection_backoff: Duration,
    maximum_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
    maximum_connection_buffer_size: usize,
    use_legacy_version: bool,
}

impl Config {
    pub fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
        maximum_connection_buffer_size: usize,
        use_legacy_version: bool,
    ) -> Self {
        Config {
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
            maximum_connection_buffer_size,
            use_legacy_version,
        }
    }
}

pub trait SendWithoutResponse {
    // Without response in this context means we will not listen for anything we might get back (not
    // that we should get anything), including any possible io errors
    fn send_without_response(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: NymPacket,
        packet_type: PacketType,
    ) -> io::Result<()>;
}

pub struct Client {
    conn_new: Option<mpsc::Sender<(FramedNymPacket, SocketAddr)>>,
    config: Config,
}

impl Client {
    pub fn new(config: Config) -> Client {
        Client {
            conn_new: None,
            config,
        }
    }

    async fn send_to_connection(address: SocketAddr, packet: FramedNymPacket) {
        let endpoint = Endpoint::client("0.0.0.0:0".parse::<SocketAddr>().unwrap()).unwrap();
        let connection = endpoint.connect(address, "mixnode").unwrap().await.unwrap();

        let mut pkt_bytes = BytesMut::new();
        match NymCodec.encode(packet, &mut pkt_bytes) {
            Ok(()) => {
                let mut send = connection.open_uni().await.unwrap();

                send.write_all(pkt_bytes.as_ref()).await.unwrap();
                send.finish().await.unwrap();
            }
            Err(err) => {
                error!("Failed to serialize packet : {err:?}");
            }
        }
    }

    // fn make_connection(&mut self, address: NymNodeRoutingAddress, pending_packet: FramedNymPacket) {
    //     let (mut sender, receiver) = mpsc::channel(self.config.maximum_connection_buffer_size);

    //     // this CAN'T fail because we just created the channel which has a non-zero capacity
    //     if self.config.maximum_connection_buffer_size > 0 {
    //         sender.try_send((pending_packet, address.into())).unwrap();
    //     }
    //     self.conn_new = Some(sender);

    //     // if we already tried to connect to `address` before, grab the current attempt count
    //     // let current_reconnection_attempt = if let Some(existing) = self.conn_new.get_mut(&address) {
    //     //     existing.channel = sender;
    //     //     Arc::clone(&existing.current_reconnection_attempt)
    //     // } else {
    //     //     let new_entry = ConnectionSender::new(sender);
    //     //     let current_attempt = Arc::clone(&new_entry.current_reconnection_attempt);
    //     //     self.conn_new.insert(address, new_entry);
    //     //     current_attempt
    //     // };

    //     // load the actual value.
    //     // let reconnection_attempt = current_reconnection_attempt.load(Ordering::Acquire);
    //     // let backoff = self.determine_backoff(reconnection_attempt);

    //     // copy the value before moving into another task
    //     // let initial_connection_timeout = self.config.initial_connection_timeout;

    //     tokio::spawn(async move {
    //         // before executing the manager, wait for what was specified, if anything
    //         // if let Some(backoff) = backoff {
    //         //     trace!("waiting for {:?} before attempting connection", backoff);
    //         //     sleep(backoff).await;
    //         // }

    //         Self::manage_connection(
    //             address.into(),
    //             receiver,
    //             //initial_connection_timeout,
    //             //&current_reconnection_attempt,
    //         )
    //         .await
    //     });
    // }
}

impl SendWithoutResponse for Client {
    fn send_without_response(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: NymPacket,
        packet_type: PacketType,
    ) -> io::Result<()> {
        debug!("Sending packet to {:?}", address);
        let framed_packet =
            FramedNymPacket::new(packet, packet_type, self.config.use_legacy_version);

        tokio::spawn(async move { Self::send_to_connection(address.into(), framed_packet).await });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_client() -> Client {
        Client::new(Config {
            initial_reconnection_backoff: Duration::from_millis(10_000),
            maximum_reconnection_backoff: Duration::from_millis(300_000),
            initial_connection_timeout: Duration::from_millis(1_500),
            maximum_connection_buffer_size: 128,
            use_legacy_version: false,
        })
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
