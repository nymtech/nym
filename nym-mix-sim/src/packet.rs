// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;
use std::fmt::Debug;

use nym_lp::packet::utils::format_debug_bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct SimplePacket {
    id: Uuid,
    pub data: Vec<u8>,
}

impl Debug for SimplePacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SimplePacket {{")?;
        writeln!(f, "    id: {:?},", self.id)?;
        writeln!(f, "    data:")?;
        for line in format_debug_bytes(&self.data)?.lines() {
            writeln!(f, "        {line}")?;
        }
        write!(f, "}}")
    }
}

impl SimplePacket {
    const SIZE: usize = 64;

    pub fn new(data: [u8; Self::SIZE - 16]) -> Self {
        Self {
            id: Uuid::new_v4(),
            data: data.to_vec(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // simple length prefixed serialization
        let mut bytes = Vec::with_capacity(Self::SIZE);

        bytes.extend_from_slice(&self.id.to_bytes_le()); // 16 bytes
        bytes.extend_from_slice(&self.data); // 48 bytes

        bytes
    }

    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != Self::SIZE {
            return Err(anyhow::anyhow!(
                "Length mismatch to deserialize a Payload : Expected {}, got {}",
                Self::SIZE,
                bytes.len()
            ));
        }
        #[allow(clippy::unwrap_used)]
        let uuid = Uuid::from_bytes_le(bytes[0..16].try_into().unwrap());
        let data = bytes[16..Self::SIZE].to_vec();
        Ok(SimplePacket { id: uuid, data })
    }
}

impl<Ts> WirePacketFormat<Ts> for SimplePacket {
    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Self::try_from_bytes(bytes)
    }

    fn process(mut self, _: Ts) -> anyhow::Result<Self> {
        self.data = self.data.into_iter().map(|b| b + 1).collect();
        Ok(self)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

pub trait WirePacketFormat<Ts>: Debug + Sized + Send + 'static {
    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self>;
    fn process(self, timestamp: Ts) -> anyhow::Result<Self>;
    fn to_bytes(&self) -> Vec<u8>;
}
