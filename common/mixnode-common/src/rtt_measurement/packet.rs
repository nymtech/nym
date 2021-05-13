// Copyright 2021 Nym Technologies SA
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

use crate::rtt_measurement::error::RttError;
use crypto::asymmetric::identity::{self, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use std::convert::TryInto;

pub(crate) struct EchoPacket {
    sequence_number: u64,
    sender: identity::PublicKey,

    signature: identity::Signature,
}

impl EchoPacket {
    pub(crate) const SIZE: usize = 8 + PUBLIC_KEY_LENGTH + SIGNATURE_LENGTH;

    pub(crate) fn new(sequence_number: u64, keys: &identity::KeyPair) -> Self {
        let bytes_to_sign = sequence_number
            .to_be_bytes()
            .iter()
            .cloned()
            .chain(keys.public_key().to_bytes().iter().cloned())
            .collect::<Vec<_>>();

        let signature = keys.private_key().sign(&bytes_to_sign);

        EchoPacket {
            sequence_number,
            sender: *keys.public_key(),
            signature,
        }
    }

    // seq || sender || sig
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        self.sequence_number
            .to_be_bytes()
            .iter()
            .cloned()
            .chain(self.sender.to_bytes().iter().cloned())
            .chain(self.signature.to_bytes().iter().cloned())
            .collect()
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, RttError> {
        if bytes.len() != Self::SIZE {
            return Err(RttError::UnexpectedEchoPacketSize);
        }

        let sequence_number = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        let sender = identity::PublicKey::from_bytes(&bytes[8..8 + PUBLIC_KEY_LENGTH])
            .map_err(|_| RttError::MalformedSenderIdentity)?;
        let signature = identity::Signature::from_bytes(&bytes[8 + PUBLIC_KEY_LENGTH..])
            .map_err(|_| RttError::MalformedEchoSignature)?;

        sender
            .verify(&bytes[..Self::SIZE - SIGNATURE_LENGTH], &signature)
            .map_err(|_| RttError::InvalidEchoSignature)?;

        Ok(EchoPacket {
            sequence_number,
            sender,
            signature,
        })
    }

    pub(crate) fn construct_reply(self, private_key: &identity::PrivateKey) -> ReplyPacket {
        let bytes = self.to_bytes();
        let signature = private_key.sign(&bytes);
        ReplyPacket {
            base_packet: self,
            signature,
        }
    }
}

pub(crate) struct ReplyPacket {
    base_packet: EchoPacket,
    signature: identity::Signature,
}

impl ReplyPacket {
    pub(crate) const SIZE: usize = EchoPacket::SIZE + SIGNATURE_LENGTH;

    pub(crate) fn base_sequence_number(&self) -> u64 {
        self.base_packet.sequence_number
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        self.base_packet
            .to_bytes()
            .into_iter()
            .chain(self.signature.to_bytes().iter().cloned())
            .collect()
    }

    pub(crate) fn try_from_bytes(
        bytes: &[u8],
        remote_identity: &identity::PublicKey,
    ) -> Result<Self, RttError> {
        if bytes.len() != Self::SIZE {
            return Err(RttError::UnexpectedReplyPacketSize);
        }

        let base_packet =
            EchoPacket::try_from_bytes(&bytes[..8 + PUBLIC_KEY_LENGTH + SIGNATURE_LENGTH])?;
        let signature =
            identity::Signature::from_bytes(&bytes[8 + PUBLIC_KEY_LENGTH + SIGNATURE_LENGTH..])
                .map_err(|_| RttError::MalformedReplySignature)?;

        remote_identity
            .verify(&bytes[..Self::SIZE - SIGNATURE_LENGTH], &signature)
            .map_err(|_| RttError::InvalidReplySignature)?;

        Ok(ReplyPacket {
            base_packet,
            signature,
        })
    }
}
