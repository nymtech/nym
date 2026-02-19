// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;
pub mod header;
pub mod message;
pub mod packet;

pub use error::MalformedLpPacketError;
pub use header::{InnerHeader, LpHeader, OuterHeader};
pub use message::{ApplicationData, ForwardPacketData, LpMessage, MessageType};
pub use packet::{EncryptedLpPacket, LpPacket};

pub mod version {
    /// The current version of the Lewes Protocol that is put into each new constructed header.
    pub const CURRENT: u8 = 1;
}
