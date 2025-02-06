// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::VerlocError;
use crate::measurements::packet::{EchoPacket, ReplyPacket};
use crate::models::VerlocMeasurement;
use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownToken;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, io};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::{debug, trace};

#[derive(Copy, Clone)]
pub(crate) struct TestedNode {
    pub(crate) address: SocketAddr,
    pub(crate) identity: ed25519::PublicKey,
}

impl TestedNode {
    pub(crate) fn new(address: SocketAddr, identity: ed25519::PublicKey) -> Self {
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

pub struct PacketSender {
    identity: Arc<ed25519::KeyPair>,
    // timeout for receiving before sending new one
    packets_per_node: usize,
    packet_timeout: Duration,
    connection_timeout: Duration,
    delay_between_packets: Duration,
    shutdown_token: ShutdownToken,
}

impl PacketSender {
    pub fn new(
        identity: Arc<ed25519::KeyPair>,
        packets_per_node: usize,
        packet_timeout: Duration,
        connection_timeout: Duration,
        delay_between_packets: Duration,
        shutdown_token: ShutdownToken,
    ) -> Self {
        PacketSender {
            identity,
            packets_per_node,
            packet_timeout,
            connection_timeout,
            delay_between_packets,
            shutdown_token,
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
    ) -> Result<VerlocMeasurement, VerlocError> {
        let mut conn = match tokio::time::timeout(
            self.connection_timeout,
            TcpStream::connect(tested_node.address),
        )
        .await
        {
            Err(_timeout) => {
                return Err(VerlocError::UnreachableNode {
                    identity: tested_node.identity.to_string(),
                    err: io::ErrorKind::TimedOut.into(),
                    address: tested_node.address,
                })
            }
            Ok(Err(err)) => {
                return Err(VerlocError::UnreachableNode {
                    identity: tested_node.identity.to_string(),
                    err,
                    address: tested_node.address,
                })
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
                            let identity = tested_node.identity;
                            debug!(
                                "failed to write echo packet to {identity} within {:?}. Stopping the test.",
                                self.packet_timeout
                            );
                            return Err(VerlocError::UnexpectedConnectionFailureWrite{
                                identity: identity.to_string(),
                                err:io::ErrorKind::TimedOut.into(),
                                address: tested_node.address
                            });
                        }
                        Ok(Err(err)) => {
                            let identity = tested_node.identity;
                            debug!(
                                "failed to write echo packet to {identity}: {err}. Stopping the test.",
                            );
                            return Err(VerlocError::UnexpectedConnectionFailureWrite{
                                identity: identity.to_string(),
                                err,
                                address: tested_node.address
                            });
                        }
                        Ok(Ok(_)) => {}
                        }
                },
                _ = self.shutdown_token.cancelled() => {
                    trace!("PacketSender: Received shutdown while sending");
                    return Err(VerlocError::ShutdownReceived);
                },
            }

            // there's absolutely no need to put a codec on ReplyPackets as we know exactly
            // when and how many we expect to receive and can easily deal with any io errors.
            let reply_packet_future = async {
                let mut buf = [0u8; ReplyPacket::SIZE];
                if let Err(err) = conn.read_exact(&mut buf).await {
                    let identity = tested_node.identity;
                    debug!(
                        "failed to read reply packet from {identity}: {err}. Stopping the test.",
                    );
                    return Err(VerlocError::UnexpectedConnectionFailureRead {
                        identity: identity.to_string(),
                        err,
                        address: tested_node.address,
                    });
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
                            return Err(VerlocError::ConnectionReadTimeout{
                                identity: tested_node.identity.to_string(),
                                address: tested_node.address
                            });
                        }
                    }
                },
                _ = self.shutdown_token.cancelled() => {
                    trace!("PacketSender: Received shutdown while waiting for reply");
                    return Err(VerlocError::ShutdownReceived);
                }
            };

            let reply_packet = reply_packet?;
            // make sure it's actually the expected packet...
            // note that we cannot receive packets not in order as we are not sending a next packet until
            // we have received the previous one
            if reply_packet.base_sequence_number() != seq {
                debug!("Received reply packet with invalid sequence number! Got {} expected {}. Stopping the test", reply_packet.base_sequence_number(), seq);
                return Err(VerlocError::UnexpectedReplySequence);
            }

            let time_taken = tokio::time::Instant::now().duration_since(start);
            results.push(time_taken);

            seq += 1;
            sleep(self.delay_between_packets).await;
        }

        Ok(VerlocMeasurement::new(&results))
    }
}
