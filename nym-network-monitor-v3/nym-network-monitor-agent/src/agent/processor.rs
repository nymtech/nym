// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::receiver::{MixnetPacketsReceiver, MixnetPacketsSender, ReceivedPacket};
use crate::agent::sphinx_helpers::TestPacketHeader;
use anyhow::Context;
use futures::StreamExt;
use futures::channel::mpsc::unbounded;
use std::fmt::Display;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::timeout;

pub(crate) struct ProcessedPacket {
    pub(crate) id: u64,
    pub(crate) rtt: Duration,
}

impl Display for ProcessedPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.id, humantime::format_duration(self.rtt))
    }
}

pub(crate) struct MixnetPacketProcessor {
    test_header: TestPacketHeader,
    receive_timeout: Duration,
    sender: MixnetPacketsSender,
    receiver: MixnetPacketsReceiver,
}

impl MixnetPacketProcessor {
    pub(crate) fn new(test_header: TestPacketHeader, receive_timeout: Duration) -> Self {
        let (sender, receiver) = unbounded();

        Self {
            test_header,
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
        let received_content = self.test_header.recover_payload(sphinx_packet.payload)?;
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
