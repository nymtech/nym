// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::verloc::error::RttError;
use crate::verloc::packet::{EchoPacket, ReplyPacket};
use log::*;
use nym_crypto::asymmetric::identity;
use nym_task::TaskClient;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, io};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;
use nym_node_http_api::state::metrics::VerlocMeasurement;

#[derive(Copy, Clone)]
pub(crate) struct TestedNode {
    pub(crate) address: SocketAddr,
    pub(crate) identity: identity::PublicKey,
}

impl TestedNode {
    pub(crate) fn new(address: SocketAddr, identity: identity::PublicKey) -> Self {
        TestedNode { address, identity }
    }
}

impl fmt::Display for TestedNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TestedNode(id: {}, address: {})",
            self.identity, self.address
        )
    }
}

pub(crate) struct PacketSender {
    identity: Arc<identity::KeyPair>,
    // timeout for receiving before sending new one
    packets_per_node: usize,
    packet_timeout: Duration,
    connection_timeout: Duration,
    delay_between_packets: Duration,
    shutdown_listener: TaskClient,
}

impl PacketSender {
    pub(super) fn new(
        identity: Arc<identity::KeyPair>,
        packets_per_node: usize,
        packet_timeout: Duration,
        connection_timeout: Duration,
        delay_between_packets: Duration,
        shutdown_listener: TaskClient,
    ) -> Self {
        PacketSender {
            identity,
            packets_per_node,
            packet_timeout,
            connection_timeout,
            delay_between_packets,
            shutdown_listener,
        }
    }

    fn random_sequence_number(&self) -> u64 {
        let mut rng = thread_rng();
        loop {
            let r: u64 = rng.gen();
            // make sure we can actually increment it packets_per_node times
            if r < (u64::MAX - self.packets_per_node as u64) {
                return r;
            }
        }
    }

    // TODO: split this function
    pub(super) async fn send_packets_to_node(
        self: Arc<Self>,
        tested_node: TestedNode,
    ) -> Result<VerlocMeasurement, RttError> {
        let mut shutdown_listener = self.shutdown_listener.fork(tested_node.address.to_string());
        shutdown_listener.mark_as_success();

        let mut conn = match tokio::time::timeout(
            self.connection_timeout,
            TcpStream::connect(tested_node.address),
        )
        .await
        {
            Err(_timeout) => {
                return Err(RttError::UnreachableNode(
                    tested_node.identity.to_base58_string(),
                    io::ErrorKind::TimedOut.into(),
                ))
            }
            Ok(Err(err)) => {
                return Err(RttError::UnreachableNode(
                    tested_node.identity.to_base58_string(),
                    err,
                ))
            }
            Ok(Ok(conn)) => conn,
        };

        let mut results = Vec::with_capacity(self.packets_per_node);

        let mut seq = self.random_sequence_number();
        for _ in 0..self.packets_per_node {
            let packet = EchoPacket::new(seq, &self.identity);
            let start = tokio::time::Instant::now();
            // TODO: should we get the start time after or before actually sending the data?
            // there's going to definitely some scheduler and network stack bias here
            let packet_bytes = packet.to_bytes();

            tokio::select! {
                write = tokio::time::timeout(self.packet_timeout, conn.write_all(packet_bytes.as_ref())) => {
                    match write {
                        Err(_timeout) => {
                            let identity_string = tested_node.identity.to_base58_string();
                            debug!(
                                "failed to write echo packet to {} within {:?}. Stopping the test.",
                                identity_string, self.packet_timeout
                            );
                            return Err(RttError::UnexpectedConnectionFailureWrite(
                                identity_string,
                                io::ErrorKind::TimedOut.into(),
                            ));
                        }
                        Ok(Err(err)) => {
                            let identity_string = tested_node.identity.to_base58_string();
                            debug!(
                                "failed to write echo packet to {} - {}. Stopping the test.",
                                identity_string, err
                            );
                            return Err(RttError::UnexpectedConnectionFailureWrite(
                                identity_string,
                                err,
                            ));
                        }
                        Ok(Ok(_)) => {}
                        }
                },
                _ = shutdown_listener.recv() => {
                    log::trace!("PacketSender: Received shutdown while sending");
                    return Err(RttError::ShutdownReceived);
                },
            }

            // there's absolutely no need to put a codec on ReplyPackets as we know exactly
            // when and how many we expect to receive and can easily deal with any io errors.
            let reply_packet_future = async {
                let mut buf = [0u8; ReplyPacket::SIZE];
                if let Err(err) = conn.read_exact(&mut buf).await {
                    debug!(
                        "failed to read reply packet from {} - {}. Stopping the test.",
                        tested_node.identity.to_base58_string(),
                        err
                    );
                    return Err(RttError::UnexpectedConnectionFailureRead(
                        tested_node.identity.to_base58_string(),
                        err,
                    ));
                }
                ReplyPacket::try_from_bytes(&buf, &tested_node.identity)
            };

            let reply_packet = tokio::select! {
                reply = tokio::time::timeout(self.packet_timeout, reply_packet_future) => {
                    match reply {
                        Ok(reply_packet) => reply_packet,
                        Err(_timeout) => {
                            // TODO: should we continue regardless (with the rest of the packets, or abandon the whole thing?)
                            // Note: if we decide to continue, it would increase the complexity of the whole thing
                            debug!(
                                "failed to receive reply to our echo packet within {:?}. Stopping the test",
                                self.packet_timeout
                            );
                            return Err(RttError::ConnectionReadTimeout(
                                tested_node.identity.to_base58_string(),
                            ));
                        }
                    }
                },
                _ = shutdown_listener.recv() => {
                    log::trace!("PacketSender: Received shutdown while waiting for reply");
                    return Err(RttError::ShutdownReceived);
                }
            };

            let reply_packet = reply_packet?;
            // make sure it's actually the expected packet...
            // note that we cannot receive packets not in order as we are not sending a next packet until
            // we have received the previous one
            if reply_packet.base_sequence_number() != seq {
                debug!("Received reply packet with invalid sequence number! Got {} expected {}. Stopping the test", reply_packet.base_sequence_number(), seq);
                return Err(RttError::UnexpectedReplySequence);
            }

            let time_taken = tokio::time::Instant::now().duration_since(start);
            results.push(time_taken);

            seq += 1;
            sleep(self.delay_between_packets).await;
        }

        Ok(VerlocMeasurement::new(&results))
    }
}
