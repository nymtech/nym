// Copyright 2021 Nym Technologies SA
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

use crate::rtt_measurement::error::RttError;
use crate::rtt_measurement::measurement::Measurement;
use crate::rtt_measurement::packet::{EchoPacket, ReplyPacket};
use crypto::asymmetric::identity;
use log::*;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;

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

pub(crate) struct PacketSender {
    identity: Arc<identity::KeyPair>,
    // timeout for receiving before sending new one
    packets_per_node: usize,
    packet_timeout: Duration,
    delay_between_packets: Duration,
}

impl PacketSender {
    pub(super) fn new(
        identity: Arc<identity::KeyPair>,
        packets_per_node: usize,
        packet_timeout: Duration,
        delay_between_packets: Duration,
    ) -> Self {
        PacketSender {
            identity,
            packets_per_node,
            packet_timeout,
            delay_between_packets,
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
    ) -> Result<Measurement, RttError> {
        let mut conn = TcpStream::connect(tested_node.address)
            .await
            .map_err(|err| {
                RttError::UnreachableNode(tested_node.identity.to_base58_string(), err)
            })?;

        let mut results = Vec::with_capacity(self.packets_per_node);

        let mut seq = self.random_sequence_number();
        for _ in 0..self.packets_per_node {
            let packet = EchoPacket::new(seq, &self.identity);
            let start = tokio::time::Instant::now();
            // TODO: should we get the start time after or before actually sending the data?
            // there's going to definitely some scheduler and network stack bias here
            if let Err(err) = conn.write_all(packet.to_bytes().as_ref()).await {
                let identity_string = tested_node.identity.to_base58_string();
                error!(
                    "failed to write echo packet to {} - {}. Stopping the test.",
                    identity_string, err
                );
                return Err(RttError::UnexpectedConnectionFailureWrite(
                    identity_string,
                    err,
                ));
            }

            // there's absolutely no need to put a codec on ReplyPackets as we know exactly
            // when and how many we expect to receive and can easily deal with any io errors.
            let reply_packet_future = async {
                let mut buf = [0u8; ReplyPacket::SIZE];
                if let Err(err) = conn.read_exact(&mut buf).await {
                    error!(
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

            let reply_packet =
                match tokio::time::timeout(self.packet_timeout, reply_packet_future).await {
                    Ok(reply_packet) => reply_packet,
                    Err(_) => {
                        // TODO: should we continue regardless (with the rest of the packets, or abandon the whole thing?)
                        // Note: if we decide to continue, it would increase the complexity of the whole thing
                        error!(
                        "failed to receive reply to our echo packet within {:?}. Stopping the test",
                        self.packet_timeout
                    );
                        return Err(RttError::ConnectionReadTimeout(
                            tested_node.identity.to_base58_string(),
                        ));
                    }
                };

            let reply_packet = reply_packet?;
            // make sure it's actually the expected packet...
            // note that we cannot receive packets not in order as we are not sending a next packet until
            // we have received the previous one
            if reply_packet.base_sequence_number() != seq {
                error!("Received reply packet with invalid sequence number! Got {} expected {}. Stopping the test", reply_packet.base_sequence_number(), seq);
                return Err(RttError::UnexpectedReplySequence);
            }

            let time_taken = tokio::time::Instant::now().duration_since(start);
            results.push(time_taken);

            seq += 1;
            sleep(self.delay_between_packets).await;
        }

        Ok(Measurement::new(&results))
    }
}
