// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crypto::asymmetric::encryption;
use crypto::asymmetric::encryption::EncryptionKeyError;
use directory_client::mixmining::MixStatus;
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter};
use std::mem;

#[derive(Debug)]
pub(crate) enum TestPacketError {
    IncompletePacket,
    InvalidIpVersion,
    InvalidNodeKey,
}

impl From<encryption::EncryptionKeyError> for TestPacketError {
    fn from(_: EncryptionKeyError) -> Self {
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

impl Into<String> for IpVersion {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl Display for IpVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub(crate) struct TestPacket {
    ip_version: IpVersion,
    nonce: u64,
    pub_key: encryption::PublicKey, // TODO: eventually this will get replaced with identity::PublicKey
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

impl TestPacket {
    pub(crate) fn new(pub_key: encryption::PublicKey, ip_version: IpVersion, nonce: u64) -> Self {
        TestPacket {
            pub_key,
            ip_version,
            nonce,
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

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
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
        let pub_key = encryption::PublicKey::from_bytes(&b[n + 1..])?;

        Ok(TestPacket {
            ip_version,
            nonce,
            pub_key,
        })
    }

    pub(crate) fn into_up_mixstatus(self) -> MixStatus {
        MixStatus {
            pub_key: self.pub_key.to_base58_string(),
            ip_version: self.ip_version.into(),
            up: true,
        }
    }

    pub(crate) fn into_down_mixstatus(self) -> MixStatus {
        MixStatus {
            pub_key: self.pub_key.to_base58_string(),
            ip_version: self.ip_version.into(),
            up: false,
        }
    }
}
