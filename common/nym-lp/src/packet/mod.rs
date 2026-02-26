// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;
pub mod header;
pub mod message;
pub mod packet;
pub mod replay;
pub mod utils;

pub use error::MalformedLpPacketError;
pub use header::{InnerHeader, LpHeader, OuterHeader};
pub use message::{ApplicationData, ForwardPacketData, LpMessage, MessageType};
pub use packet::{EncryptedLpPacket, LpPacket};

pub mod version {
    /// The current version of the Lewes Protocol that is put into each new constructed header.
    pub const CURRENT: u8 = 1;
}

#[allow(dead_code)]
pub(crate) const UDP_HEADER_LEN: usize = 8;
#[allow(dead_code)]
pub(crate) const IP_HEADER_LEN: usize = 40; // v4 - 20, v6 - 40
#[allow(dead_code)]
pub(crate) const MTU: usize = 1500;
#[allow(dead_code)]
pub(crate) const UDP_OVERHEAD: usize = UDP_HEADER_LEN + IP_HEADER_LEN;
#[allow(dead_code)]
pub(crate) const UDP_PAYLOAD_SIZE: usize = MTU - UDP_OVERHEAD;
