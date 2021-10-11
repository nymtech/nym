// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::preparer::TestedNode;
use crypto::asymmetric::identity;
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::mem;
use std::str::Utf8Error;

#[repr(u8)]
#[derive(Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub(crate) enum NodeType {
    Mixnode = 0,
    Gateway = 1,
}

impl TryFrom<u8> for NodeType {
    type Error = TestPacketError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::Mixnode as u8) => Ok(Self::Mixnode),
            _ if value == (Self::Gateway as u8) => Ok(Self::Gateway),
            _ => Err(TestPacketError::InvalidNodeType),
        }
    }
}

#[derive(Debug)]
pub(crate) enum TestPacketError {
    IncompletePacket,
    InvalidIpVersion,
    InvalidNodeType,
    InvalidNodeKey,
    InvalidOwner(Utf8Error),
}

impl From<identity::KeyRecoveryError> for TestPacketError {
    fn from(_: identity::KeyRecoveryError) -> Self {
        TestPacketError::InvalidNodeKey
    }
}

impl From<Utf8Error> for TestPacketError {
    fn from(err: Utf8Error) -> Self {
        TestPacketError::InvalidOwner(err)
    }
}

#[derive(Eq, Clone, Debug)]
pub(crate) struct TestPacket {
    nonce: u64,
    pub_key: identity::PublicKey,
    owner: String,
    node_type: NodeType,
}

impl Display for TestPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TestPacket {{ pub_key: {}, owner: {}, nonce: {} }}",
            self.pub_key.to_base58_string(),
            self.owner,
            self.nonce
        )
    }
}

impl Hash for TestPacket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.nonce.hash(state);
        self.pub_key.to_bytes().hash(state);
        self.owner.hash(state);
    }
}

impl PartialEq for TestPacket {
    fn eq(&self, other: &Self) -> bool {
        self.nonce == other.nonce && self.pub_key.to_bytes() == other.pub_key.to_bytes()
    }
}

impl TestPacket {
    pub(crate) fn new(
        pub_key: identity::PublicKey,
        owner: String,
        nonce: u64,
        node_type: NodeType,
    ) -> Self {
        TestPacket {
            nonce,
            pub_key,
            owner,
            node_type,
        }
    }

    pub(crate) fn nonce(&self) -> u64 {
        self.nonce
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        self.nonce
            .to_be_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(self.node_type as u8))
            .chain(self.pub_key.to_bytes().iter().cloned())
            .chain(self.owner.as_bytes().iter().cloned())
            .collect()
    }

    pub(crate) fn try_from_bytes(b: &[u8]) -> Result<Self, TestPacketError> {
        // nonce size
        let n = mem::size_of::<u64>();

        if b.len() < n + 1 + identity::PUBLIC_KEY_LENGTH {
            return Err(TestPacketError::IncompletePacket);
        }

        // this unwrap can't fail as we've already checked for the size
        let nonce = u64::from_be_bytes(b[0..n].try_into().unwrap());
        let node_type = NodeType::try_from(b[n])?;

        let pub_key =
            identity::PublicKey::from_bytes(&b[n + 1..n + 1 + identity::PUBLIC_KEY_LENGTH])?;
        let owner = std::str::from_utf8(&b[n + 1 + identity::PUBLIC_KEY_LENGTH..])?;

        Ok(TestPacket {
            node_type,
            nonce,
            pub_key,
            owner: owner.to_owned(),
        })
    }
}

impl From<TestPacket> for TestedNode {
    fn from(packet: TestPacket) -> Self {
        TestedNode {
            identity: packet.pub_key.to_base58_string(),
            owner: packet.owner,
            node_type: packet.node_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn test_packet_roundtrip() {
        let mut rng = thread_rng();
        let dummy_keypair = identity::KeyPair::new(&mut rng);
        let owner = "some owner".to_string();
        let packet = TestPacket::new(*dummy_keypair.public_key(), owner, 42, NodeType::Mixnode);

        let bytes = packet.to_bytes();
        let recovered = TestPacket::try_from_bytes(&bytes).unwrap();
        assert_eq!(packet, recovered);
    }
}
