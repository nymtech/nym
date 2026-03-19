// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::{Context, bail};
use nym_sphinx_params::PacketSize;
use nym_sphinx_types::{Payload, PayloadKey, SphinxHeader, SphinxPacket};
use time::OffsetDateTime;

pub(crate) struct TestPacketHeader {
    pub(crate) header: SphinxHeader,
    pub(crate) payload_key: Vec<PayloadKey>,
}

impl Clone for TestPacketHeader {
    fn clone(&self) -> Self {
        TestPacketHeader {
            header: SphinxHeader {
                shared_secret: self.header.shared_secret,
                routing_info: self.header.routing_info.clone(),
            },
            payload_key: self.payload_key.clone(),
        }
    }
}

impl TestPacketHeader {
    pub(crate) fn create_test_packet(
        &self,
        content: TestPacketContent,
    ) -> anyhow::Result<SphinxPacket> {
        let payload = Payload::encapsulate_message(
            &content.to_bytes(),
            &self.payload_key,
            PacketSize::AckPacket.payload_size(),
        )?;
        Ok(SphinxPacket {
            header: SphinxHeader {
                shared_secret: self.header.shared_secret,
                routing_info: self.header.routing_info.clone(),
            },
            payload,
        })
    }

    pub(crate) fn recover_payload(&self, received: Payload) -> anyhow::Result<TestPacketContent> {
        let key = self
            .payload_key
            .last()
            .context("no payload keys generated")?;

        let payload = received.unwrap(key)?.recover_plaintext()?;
        TestPacketContent::from_bytes(&payload)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct TestPacketContent {
    pub(crate) id: u64,
    pub(crate) sending_timestamp: OffsetDateTime,
}

impl TestPacketContent {
    pub(crate) fn new(id: u64) -> Self {
        Self {
            id,
            sending_timestamp: OffsetDateTime::now_utc(),
        }
    }

    pub(crate) fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.sending_timestamp.unix_timestamp().to_be_bytes());
        bytes
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 16 {
            bail!("malformed test packet received")
        }

        let id = u64::from_be_bytes(bytes[0..8].try_into()?);
        let sending_timestamp = i64::from_be_bytes(bytes[8..16].try_into()?);
        Ok(Self {
            id,
            sending_timestamp: OffsetDateTime::from_unix_timestamp(sending_timestamp)?,
        })
    }
}
