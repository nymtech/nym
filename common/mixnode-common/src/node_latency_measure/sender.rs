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

use crate::node_latency_measure::packet::{EchoPacket, ReplyPacket};
use crypto::asymmetric::identity;
use log::*;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct PacketSender {
    identity: Arc<identity::KeyPair>,
    // timeout for receiving before sending new one
    batch_size: usize,
    packets_per_node: usize,
    delay_between_packets: Duration,
    packet_timeout: Duration,
}

// TODO: move elsewhere
struct NodeResult {
    minimum: Duration,
    mean: Duration,
    standard_deviation: Duration,
}

impl PacketSender {
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

    // TODO: those definitely do not fit "PacketSender"
    fn duration_mean(data: &[Duration]) -> Duration {
        let sum = data.iter().sum::<Duration>();
        let count = data.len() as u32;

        sum / count
    }

    fn duration_standard_deviation(data: &[Duration], mean: Duration) -> Duration {
        let variance_micros = data
            .iter()
            .map(|&value| {
                // make sure we don't underflow
                let diff = if mean > value {
                    mean - value
                } else {
                    value - mean
                };
                // we don't need nanos precision
                let diff_micros = diff.as_micros();
                diff_micros * diff_micros
            })
            .sum::<u128>()
            / data.len() as u128;

        // we shouldn't really overflow as our differences shouldn't be larger than couple seconds at the worst possible case scenario
        let std_deviation_micros = (variance_micros as f64).sqrt() as u64;
        Duration::from_micros(std_deviation_micros)
    }

    // TODO: split this function
    async fn send_packets_to_node(&self, address: SocketAddr) -> Result<NodeResult, ()> {
        let mut conn = match TcpStream::connect(address).await {
            Ok(conn) => conn,
            Err(err) => todo!(),
        };

        let mut results = Vec::with_capacity(self.packets_per_node);

        let mut seq = self.random_sequence_number();
        for _ in 0..self.packets_per_node {
            let packet = EchoPacket::new(seq, &self.identity);
            let start = tokio::time::Instant::now();
            // TODO: should we get the start time after or before actually sending the data?
            // there's going to definitely some scheduler and network stack bias here
            if let Err(err) = conn.write_all(packet.to_bytes().as_ref()).await {
                // todo handle err
            }

            let reply_packet_future = async {
                let mut buf = [0u8; ReplyPacket::SIZE];
                if let Err(err) = conn.read_exact(&mut buf).await {
                    // todo: handle
                }
                ReplyPacket::try_from_bytes(&buf)
            };

            let reply_packet =
                match tokio::time::timeout(self.packet_timeout, reply_packet_future).await {
                    Ok(reply_packet) => reply_packet,
                    Err(timeout) => {
                        // TODO: should we continue regardless (with the rest of the packets, or abandon the whole thing?)
                        error!(
                            "failed to receive reply to our echo packet within {:?}",
                            self.packet_timeout
                        );
                        todo!()
                    }
                };

            match reply_packet {
                Err(err) => {
                    // again, what should we do here?
                    todo!()
                }
                Ok(packet) => {
                    let time_taken = tokio::time::Instant::now().duration_since(start);
                    results.push(time_taken);
                }
            }

            // if let Err(timeout) = tokio::time::timeout(self.packet_timeout, todo!()) {
            // }

            // there's absolutely no need to put a codec on ReplyPackets as we know exactly
            // when and how many we expect to receive and can easily deal with any io errors.

            seq += 1;
        }

        let minimum = *results.iter().min().expect("didn't get any results!");

        let mean = Self::duration_mean(&results);
        let standard_deviation = Self::duration_standard_deviation(&results, mean);

        Ok(NodeResult {
            minimum,
            mean,
            standard_deviation,
        })
    }
}
