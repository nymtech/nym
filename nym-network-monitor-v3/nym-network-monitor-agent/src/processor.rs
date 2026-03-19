// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::listener::received::{MixnetPacketsReceiver, MixnetPacketsSender, ReceivedPacket};
use crate::test_packet::{TestPacketContent, TestPacketHeader};
use anyhow::{Context, bail};
use futures::StreamExt;
use futures::channel::mpsc::unbounded;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_types::{ProcessedPacketData, SphinxPacket};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcessedPacket {
    pub(crate) id: u64,
    pub(crate) rtt: Duration,
}

impl Display for ProcessedPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.id, humantime::format_duration(self.rtt))
    }
}

pub(crate) enum PayloadRecovery {
    ReusableHeader(TestPacketHeader),
    FullProcessing(Arc<x25519::KeyPair>),
}

impl From<TestPacketHeader> for PayloadRecovery {
    fn from(header: TestPacketHeader) -> Self {
        PayloadRecovery::ReusableHeader(header)
    }
}

impl From<Arc<x25519::KeyPair>> for PayloadRecovery {
    fn from(private_key: Arc<x25519::KeyPair>) -> Self {
        PayloadRecovery::FullProcessing(private_key)
    }
}

impl PayloadRecovery {
    pub(crate) fn recover_test_payload(
        &self,
        received: SphinxPacket,
    ) -> anyhow::Result<TestPacketContent> {
        match self {
            PayloadRecovery::ReusableHeader(header) => header.recover_payload(received.payload),
            PayloadRecovery::FullProcessing(private_key) => {
                let ProcessedPacketData::FinalHop { payload, .. } =
                    received.process(private_key.private_key().inner())?.data
                else {
                    bail!("received non final hop data")
                };
                TestPacketContent::from_bytes(&payload.recover_plaintext()?)
            }
        }
    }
}

pub(crate) struct MixnetPacketProcessor {
    payload_recovery: PayloadRecovery,
    receive_timeout: Duration,
    sender: MixnetPacketsSender,
    receiver: MixnetPacketsReceiver,
}

impl MixnetPacketProcessor {
    pub(crate) fn new(payload_recovery: PayloadRecovery, receive_timeout: Duration) -> Self {
        let (sender, receiver) = unbounded();

        Self {
            payload_recovery,
            receive_timeout,
            sender,
            receiver,
        }
    }

    pub(crate) fn sender(&self) -> MixnetPacketsSender {
        self.sender.clone()
    }

    fn process_received(&self, packet: ReceivedPacket) -> anyhow::Result<ProcessedPacket> {
        let sphinx_packet = packet
            .received
            .into_inner()
            .to_sphinx_packet()
            .context("the received packet was not a sphinx packet!")?;
        let received_content = self.payload_recovery.recover_test_payload(sphinx_packet)?;
        let latency = packet.received_at - received_content.sending_timestamp;

        Ok(ProcessedPacket {
            id: received_content.id,
            rtt: latency.unsigned_abs(),
        })
    }

    pub(crate) fn all_available(&mut self) -> Vec<anyhow::Result<ProcessedPacket>> {
        let mut packets = Vec::new();
        while let Ok(Some(pending)) = self.receiver.try_next() {
            packets.push(self.process_received(pending));
        }

        packets
    }

    pub(crate) async fn next_packet(&mut self) -> anyhow::Result<ProcessedPacket> {
        let packet = timeout(self.receive_timeout, self.receiver.next())
            .await?
            .context("stream has been exhausted")?;

        self.process_received(packet)
    }
}
