// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use time::OffsetDateTime;

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

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
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
