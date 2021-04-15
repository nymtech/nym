// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::{encryption, identity};
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::mem;

#[derive(Debug)]
pub(crate) enum TestPacketError {
    IncompletePacket,
    InvalidIpVersion,
    InvalidNodeKey,
}

impl From<identity::KeyRecoveryError> for TestPacketError {
    fn from(_: identity::KeyRecoveryError) -> Self {
        TestPacketError::InvalidNodeKey
    }
}

#[repr(u8)]
#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub(crate) enum IpVersion {
    V4 = 4,
    V6 = 6,
}

impl TryFrom<u8> for IpVersion {
    type Error = TestPacketError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::V4 as u8) => Ok(Self::V4),
            _ if value == (Self::V6 as u8) => Ok(Self::V6),
            _ => Err(TestPacketError::InvalidIpVersion),
        }
    }
}

impl IpVersion {
    pub(crate) fn is_v4(&self) -> bool {
        *self == IpVersion::V4
    }
}

impl From<IpVersion> for String {
    fn from(ipv: IpVersion) -> Self {
        format!("{}", ipv)
    }
}

impl Display for IpVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(Eq, Copy, Clone, Debug)]
pub(crate) struct TestPacket {
    ip_version: IpVersion,
    nonce: u64,
    pub_key: identity::PublicKey,
}

impl Display for TestPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TestPacket {{ ip: {}, pub_key: {}, nonce: {} }}",
            self.ip_version,
            self.pub_key.to_base58_string(),
            self.nonce
        )
    }
}

impl Hash for TestPacket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ip_version.hash(state);
        self.nonce.hash(state);
        self.pub_key.to_bytes().hash(state);
    }
}

impl PartialEq for TestPacket {
    fn eq(&self, other: &Self) -> bool {
        self.ip_version == other.ip_version
            && self.nonce == other.nonce
            && self.pub_key.to_bytes() == other.pub_key.to_bytes()
    }
}

impl TestPacket {
    pub(crate) fn new_v4(pub_key: identity::PublicKey, nonce: u64) -> Self {
        TestPacket {
            ip_version: IpVersion::V4,
            nonce,
            pub_key,
        }
    }

    pub(crate) fn new_v6(pub_key: identity::PublicKey, nonce: u64) -> Self {
        TestPacket {
            ip_version: IpVersion::V6,
            nonce,
            pub_key,
        }
    }

    pub(crate) fn nonce(&self) -> u64 {
        self.nonce
    }

    pub(crate) fn ip_version(&self) -> IpVersion {
        self.ip_version
    }

    pub(crate) fn pub_key_string(&self) -> String {
        self.pub_key.to_base58_string()
    }

    pub(crate) fn to_bytes(self) -> Vec<u8> {
        self.nonce
            .to_be_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(self.ip_version as u8))
            .chain(self.pub_key.to_bytes().iter().cloned())
            .collect()
    }

    pub(crate) fn try_from_bytes(b: &[u8]) -> Result<Self, TestPacketError> {
        // nonce size
        let n = mem::size_of::<u64>();

        if b.len() != n + 1 + encryption::PUBLIC_KEY_SIZE {
            return Err(TestPacketError::IncompletePacket);
        }

        // this unwrap can't fail as we've already checked for the size
        let nonce = u64::from_be_bytes(b[0..n].try_into().unwrap());
        let ip_version = IpVersion::try_from(b[n])?;
        let pub_key = identity::PublicKey::from_bytes(&b[n + 1..])?;

        Ok(TestPacket {
            ip_version,
            nonce,
            pub_key,
        })
    }
}
