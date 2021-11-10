// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::preparer::TestedNode;
use crypto::asymmetric::identity;
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::mem;
use std::str::Utf8Error;
use topology::{gateway, mix};

#[repr(u8)]
#[derive(Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub(crate) enum NodeType {
    Mixnode = 0,
    Gateway = 1,
}

impl NodeType {
    pub(crate) fn is_mixnode(&self) -> bool {
        matches!(self, NodeType::Mixnode)
    }
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
    pub(crate) route_id: u64,
    pub(crate) test_nonce: u64,
    pub(crate) pub_key: identity::PublicKey,
    pub(crate) owner: String,
    pub(crate) node_type: NodeType,
}

impl Display for TestPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TestPacket {{ pub_key: {}, owner: {}, route: {} test nonce: {} }}",
            self.pub_key.to_base58_string(),
            self.owner,
            self.route_id,
            self.test_nonce
        )
    }
}

impl Hash for TestPacket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.route_id.hash(state);
        self.test_nonce.hash(state);
        self.pub_key.to_bytes().hash(state);
        self.owner.hash(state);
        self.node_type.hash(state);
    }
}

impl PartialEq for TestPacket {
    fn eq(&self, other: &Self) -> bool {
        self.route_id == other.route_id
            && self.test_nonce == other.test_nonce
            && self.pub_key.to_bytes() == other.pub_key.to_bytes()
            && self.owner == other.owner
            && self.node_type == other.node_type
    }
}

impl TestPacket {
    pub(crate) fn from_mixnode(mix: &mix::Node, route_id: u64, test_nonce: u64) -> Self {
        TestPacket {
            pub_key: mix.identity_key,
            owner: mix.owner.clone(),
            route_id,
            test_nonce,
            node_type: NodeType::Mixnode,
        }
    }

    pub(crate) fn from_gateway(gateway: &gateway::Node, route_id: u64, test_nonce: u64) -> Self {
        TestPacket {
            pub_key: gateway.identity_key,
            owner: gateway.owner.clone(),
            route_id,
            test_nonce,
            node_type: NodeType::Gateway,
        }
    }

    pub(crate) fn new(
        pub_key: identity::PublicKey,
        owner: String,
        route_id: u64,
        test_nonce: u64,
        node_type: NodeType,
    ) -> Self {
        TestPacket {
            route_id,
            test_nonce,
            pub_key,
            owner,
            node_type,
        }
    }

    pub(crate) fn test_nonce(&self) -> u64 {
        self.test_nonce
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        IntoIterator::into_iter(self.route_id.to_be_bytes())
            .chain(IntoIterator::into_iter(self.test_nonce.to_be_bytes()))
            .chain(std::iter::once(self.node_type as u8))
            .chain(self.pub_key.to_bytes().iter().cloned())
            .chain(self.owner.as_bytes().iter().cloned())
            .collect()
    }

    pub(crate) fn try_from_bytes(b: &[u8]) -> Result<Self, TestPacketError> {
        // route id + test nonce size
        let n = mem::size_of::<u64>();

        if b.len() < 2 * n + 1 + identity::PUBLIC_KEY_LENGTH {
            return Err(TestPacketError::IncompletePacket);
        }

        // those unwraps can't fail as we've already checked for the size
        let route_id = u64::from_be_bytes(b[0..n].try_into().unwrap());
        let test_nonce = u64::from_be_bytes(b[n..2 * n].try_into().unwrap());
        let node_type = NodeType::try_from(b[2 * n])?;

        let pub_key = identity::PublicKey::from_bytes(
            &b[2 * n + 1..2 * n + 1 + identity::PUBLIC_KEY_LENGTH],
        )?;
        let owner = std::str::from_utf8(&b[2 * n + 1 + identity::PUBLIC_KEY_LENGTH..])?;

        Ok(TestPacket {
            route_id,
            node_type,
            test_nonce,
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

    #[test]
    fn test_packet_roundtrip() {
        let mut rng = rand_07::thread_rng();
        let dummy_keypair = identity::KeyPair::new(&mut rng);
        let owner = "some owner".to_string();
        let packet = TestPacket::new(
            *dummy_keypair.public_key(),
            owner,
            42,
            123,
            NodeType::Mixnode,
        );

        let bytes = packet.to_bytes();
        let recovered = TestPacket::try_from_bytes(&bytes).unwrap();
        assert_eq!(packet, recovered);
    }
}
