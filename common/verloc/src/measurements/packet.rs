// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::VerlocError;
use nym_crypto::asymmetric::ed25519::{self, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};

pub struct EchoPacket {
    sequence_number: u64,
    sender: ed25519::PublicKey,

    signature: ed25519::Signature,
}

impl EchoPacket {
    pub(crate) const SIZE: usize = 8 + PUBLIC_KEY_LENGTH + SIGNATURE_LENGTH;

    pub(crate) fn new(sequence_number: u64, keys: &ed25519::KeyPair) -> Self {
        let bytes_to_sign = sequence_number
            .to_be_bytes()
            .iter()
            .cloned()
            .chain(keys.public_key().to_bytes().iter().cloned())
            .collect::<Vec<_>>();

        let signature = keys.private_key().sign(bytes_to_sign);

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

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, VerlocError> {
        if bytes.len() != Self::SIZE {
            return Err(VerlocError::UnexpectedEchoPacketSize);
        }

        // SAFETY: we have ensured our packet has correct size
        #[allow(clippy::unwrap_used)]
        let sequence_number = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        let sender = ed25519::PublicKey::from_bytes(&bytes[8..8 + PUBLIC_KEY_LENGTH])
            .map_err(|_| VerlocError::MalformedSenderIdentity)?;
        let signature = ed25519::Signature::from_bytes(&bytes[8 + PUBLIC_KEY_LENGTH..])
            .map_err(|_| VerlocError::MalformedEchoSignature)?;

        sender
            .verify(&bytes[..Self::SIZE - SIGNATURE_LENGTH], &signature)
            .map_err(|_| VerlocError::InvalidEchoSignature)?;

        Ok(EchoPacket {
            sequence_number,
            sender,
            signature,
        })
    }

    pub(crate) fn construct_reply(self, private_key: &ed25519::PrivateKey) -> ReplyPacket {
        let bytes = self.to_bytes();
        let signature = private_key.sign(bytes);
        ReplyPacket {
            base_packet: self,
            signature,
        }
    }
}

pub struct ReplyPacket {
    base_packet: EchoPacket,
    signature: ed25519::Signature,
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
        remote_ed25519: &ed25519::PublicKey,
    ) -> Result<Self, VerlocError> {
        if bytes.len() != Self::SIZE {
            return Err(VerlocError::UnexpectedReplyPacketSize);
        }

        let base_packet =
            EchoPacket::try_from_bytes(&bytes[..8 + PUBLIC_KEY_LENGTH + SIGNATURE_LENGTH])?;
        let signature =
            ed25519::Signature::from_bytes(&bytes[8 + PUBLIC_KEY_LENGTH + SIGNATURE_LENGTH..])
                .map_err(|_| VerlocError::MalformedReplySignature)?;

        remote_ed25519
            .verify(&bytes[..Self::SIZE - SIGNATURE_LENGTH], &signature)
            .map_err(|_| VerlocError::InvalidReplySignature)?;

        Ok(ReplyPacket {
            base_packet,
            signature,
        })
    }
}
