// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::chunking;
use nym_crypto::asymmetric::encryption;
use nym_crypto::Digest;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_addressing::nodes::MAX_NODE_ADDRESS_UNPADDED_LEN;
use nym_sphinx_anonymous_replies::requests::{
    InvalidReplyRequestError, RepliableMessage, RepliableMessageContent, ReplyMessage,
    ReplyMessageContent,
};
use nym_sphinx_chunking::fragment::Fragment;
use nym_sphinx_params::{PacketSize, ReplySurbKeyDigestAlgorithm};
use rand::Rng;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NymMessageError {
    #[error("{received} is not a valid type tag for a NymMessage")]
    InvalidMessageType { received: u8 },

    #[error(transparent)]
    InvalidReplyRequest(#[from] InvalidReplyRequestError),

    #[error("The received message seems to have incorrect zero padding (no '1' byte found)")]
    InvalidMessagePadding,

    #[error("Received empty message for deserialization")]
    EmptyMessage,
}

#[repr(u8)]
enum NymMessageType {
    Plain = 0,
    Repliable = 1,
    Reply = 2,
}

impl TryFrom<u8> for NymMessageType {
    type Error = NymMessageError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (NymMessageType::Plain as u8) => Ok(Self::Plain),
            _ if value == (NymMessageType::Repliable as u8) => Ok(Self::Repliable),
            _ if value == (NymMessageType::Reply as u8) => Ok(Self::Reply),
            val => Err(NymMessageError::InvalidMessageType { received: val }),
        }
    }
}

pub type PlainMessage = Vec<u8>;

#[derive(Debug)]
pub enum NymMessage {
    Plain(PlainMessage),
    Repliable(RepliableMessage),
    Reply(ReplyMessage),
}

impl Display for NymMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NymMessage::Plain(plain_message) => write!(
                f,
                "plain {:.2} kiB message",
                plain_message.len() as f64 / 1024.0
            ),
            NymMessage::Repliable(repliable_message) => repliable_message.fmt(f),
            NymMessage::Reply(reply_message) => reply_message.fmt(f),
        }
    }
}

impl NymMessage {
    pub fn new_additional_surbs_request(recipient: Recipient, amount: u32) -> Self {
        NymMessage::Reply(ReplyMessage {
            content: ReplyMessageContent::SurbRequest {
                recipient: Box::new(recipient),
                amount,
            },
        })
    }

    pub fn new_plain(msg: Vec<u8>) -> Self {
        NymMessage::Plain(msg)
    }

    pub fn new_repliable(msg: RepliableMessage) -> Self {
        NymMessage::Repliable(msg)
    }

    pub fn new_reply(msg: ReplyMessage) -> Self {
        NymMessage::Reply(msg)
    }

    pub fn is_reply_surb_request(&self) -> bool {
        match self {
            NymMessage::Reply(reply_msg) => {
                matches!(reply_msg.content, ReplyMessageContent::SurbRequest { .. })
            }
            _ => false,
        }
    }

    pub fn into_inner_data(self) -> Vec<u8> {
        match self {
            NymMessage::Plain(data) => data,
            NymMessage::Repliable(repliable) => match repliable.content {
                RepliableMessageContent::Data { message, .. } => message,
                _ => Vec::new(),
            },
            NymMessage::Reply(reply) => match reply.content {
                ReplyMessageContent::Data { message } => message,
                _ => Vec::new(),
            },
        }
    }

    fn typ(&self) -> NymMessageType {
        match self {
            NymMessage::Plain(_) => NymMessageType::Plain,
            NymMessage::Repliable(_) => NymMessageType::Repliable,
            NymMessage::Reply(_) => NymMessageType::Reply,
        }
    }

    fn inner_bytes(self) -> Vec<u8> {
        match self {
            NymMessage::Plain(msg) => msg,
            NymMessage::Repliable(msg) => msg.into_bytes(),
            NymMessage::Reply(msg) => msg.into_bytes(),
        }
    }

    // the message is in the format of:
    // typ || msg
    fn into_bytes(self) -> Vec<u8> {
        let typ = self.typ();

        std::iter::once(typ as u8)
            .chain(self.inner_bytes())
            .collect()
    }

    fn try_from_bytes(bytes: &[u8], num_mix_hops: u8) -> Result<Self, NymMessageError> {
        if bytes.is_empty() {
            return Err(NymMessageError::EmptyMessage);
        }

        let typ_tag = NymMessageType::try_from(bytes[0])?;
        match typ_tag {
            NymMessageType::Plain => Ok(NymMessage::Plain(bytes[1..].to_vec())),
            NymMessageType::Repliable => Ok(NymMessage::Repliable(
                RepliableMessage::try_from_bytes(&bytes[1..], num_mix_hops)?,
            )),
            NymMessageType::Reply => Ok(NymMessage::Reply(ReplyMessage::try_from_bytes(
                &bytes[1..],
            )?)),
        }
    }

    /// Length of plaintext (from the sphinx point of view) data that is available per sphinx
    /// packet.
    pub fn available_plaintext_per_packet(&self, packet_size: PacketSize) -> usize {
        let ack_overhead = MAX_NODE_ADDRESS_UNPADDED_LEN + PacketSize::AckPacket.size();

        let variant_overhead = match self {
            // each plain or repliable packet attaches an ephemeral public key so that the recipient
            // could perform diffie-hellman with its own keys followed by a kdf to re-derive
            // the packet encryption key
            NymMessage::Plain(_) | NymMessage::Repliable(_) => encryption::PUBLIC_KEY_SIZE,
            // each reply attaches the digest of the encryption key so that the recipient could
            // lookup correct key for decryption,
            NymMessage::Reply(_) => ReplySurbKeyDigestAlgorithm::output_size(),
        };

        packet_size.plaintext_size() - ack_overhead - variant_overhead
    }

    /// Pads the message so that after it gets chunked, it will occupy exactly N sphinx packets.
    /// Produces new_message = message || 1 || 0000....
    pub fn pad_to_full_packet_lengths(self, plaintext_per_packet: usize) -> PaddedMessage {
        let self_display = self.to_string();

        let bytes = self.into_bytes();

        // 1 is added as there will always have to be at least a single byte of padding (1) added
        // to be able to later distinguish the actual padding from the underlying message
        let (packets_used, space_left) =
            chunking::number_of_required_fragments(bytes.len() + 1, plaintext_per_packet);

        let wasted_space = space_left as f32 / (bytes.len() + 1 + space_left) as f32;
        log::trace!("Padding {self_display}: {} of raw plaintext bytes are required. They're going to be put into {packets_used} sphinx packets with {space_left} bytes of leftover space. {wasted_space}% of packet capacity is going to be wasted.", bytes.len() + 1);

        bytes
            .into_iter()
            .chain(std::iter::once(1u8))
            .chain(std::iter::repeat(0u8).take(space_left))
            .collect::<Vec<_>>()
            .into()
    }
}

pub struct PaddedMessage(Vec<u8>);

impl PaddedMessage {
    pub fn new_reconstructed(bytes: Vec<u8>) -> Self {
        PaddedMessage(bytes)
    }

    /// Splits the padded message into [`Fragment`] that when serialized are going to become
    /// sphinx packet payloads.
    pub fn split_into_fragments<R: Rng>(
        self,
        rng: &mut R,
        plaintext_per_packet: usize,
    ) -> Vec<Fragment> {
        chunking::split_into_sets(rng, &self.0, plaintext_per_packet)
            .into_iter()
            .flat_map(|fragment_set| fragment_set.into_iter())
            .collect()
    }

    // reverse of NymMessage::pad_to_full_packet_lengths
    pub fn remove_padding(self, num_mix_hops: u8) -> Result<NymMessage, NymMessageError> {
        // we are looking for first occurrence of 1 in the tail and we get its index
        if let Some(padding_end) = self.0.iter().rposition(|b| *b == 1) {
            // and now we only take bytes until that point (but not including it)
            NymMessage::try_from_bytes(&self.0[..padding_end], num_mix_hops)
        } else {
            Err(NymMessageError::InvalidMessagePadding)
        }
    }
}

impl From<Vec<u8>> for PaddedMessage {
    fn from(bytes: Vec<u8>) -> Self {
        PaddedMessage(bytes)
    }
}
