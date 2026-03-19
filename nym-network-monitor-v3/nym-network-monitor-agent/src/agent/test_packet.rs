// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::TestedNodeDetails;
use anyhow::bail;
use time::OffsetDateTime;

pub(crate) struct TestPacketContent {
    id: u64,
    sending_timestamp: u64,
}

impl TestPacketContent {
    pub(crate) fn new(id: u64) -> Self {
        Self {
            id,
            sending_timestamp: OffsetDateTime::now_utc().unix_timestamp() as u64,
        }
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.sending_timestamp.to_be_bytes());
        bytes
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 16 {
            bail!("malformed test packet received")
        }

        let id = u64::from_be_bytes(bytes[0..8].try_into()?);
        let sending_timestamp = u64::from_be_bytes(bytes[8..16].try_into()?);
        Ok(Self {
            id,
            sending_timestamp,
        })
    }
}
