// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::packet::EncryptedLpPacket;

use crate::packet::WirePacketFormat;

impl WirePacketFormat for EncryptedLpPacket {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(EncryptedLpPacket::decode(bytes)?)
    }
}
