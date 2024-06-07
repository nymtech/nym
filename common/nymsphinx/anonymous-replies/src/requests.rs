// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{ReplySurb, ReplySurbError};
use nym_sphinx_addressing::clients::{Recipient, RecipientFormattingError};
use rand::{CryptoRng, RngCore};
use std::fmt::{Display, Formatter};
use std::mem;
use thiserror::Error;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub const SENDER_TAG_SIZE: usize = 16;

#[derive(Debug, Error)]
pub enum InvalidAnonymousSenderTagRepresentation {
    #[error("Failed to decode the base58-encoded string - {0}")]
    MalformedString(#[from] bs58::decode::Error),

    #[error(
        "Decoded AnonymousSenderTag has invalid length. Expected {expected}, but got {received}"
    )]
    InvalidLength { received: usize, expected: usize },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct AnonymousSenderTag([u8; SENDER_TAG_SIZE]);

impl From<[u8; SENDER_TAG_SIZE]> for AnonymousSenderTag {
    fn from(bytes: [u8; SENDER_TAG_SIZE]) -> Self {
        AnonymousSenderTag(bytes)
    }
}

impl Display for AnonymousSenderTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_base58_string())
    }
}

impl AnonymousSenderTag {
    pub fn new_random<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let mut bytes = [0u8; SENDER_TAG_SIZE];
        rng.fill_bytes(&mut bytes);
        AnonymousSenderTag(bytes)
    }

    pub fn to_bytes(&self) -> [u8; SENDER_TAG_SIZE] {
        self.0
    }

    pub fn from_bytes(bytes: [u8; SENDER_TAG_SIZE]) -> Self {
        AnonymousSenderTag(bytes)
    }

    pub fn to_base58_string(self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    pub fn try_from_base58_string<I: AsRef<[u8]>>(
        val: I,
    ) -> Result<Self, InvalidAnonymousSenderTagRepresentation> {
        let bytes = bs58::decode(val).into_vec()?;
        if bytes.len() != SENDER_TAG_SIZE {
            return Err(InvalidAnonymousSenderTagRepresentation::InvalidLength {
                received: bytes.len(),
                expected: SENDER_TAG_SIZE,
            });
        }

        // the unwrap here is fine as we just asserted the bytes are of exactly SENDER_TAG_SIZE length
        let byte_array: [u8; SENDER_TAG_SIZE] = bytes.try_into().unwrap();
        Ok(AnonymousSenderTag::from_bytes(byte_array))
    }
}

#[derive(Debug, Error)]
pub enum InvalidReplyRequestError {
    #[error("Did not provide sufficient number of bytes to deserialize a valid request")]
    RequestTooShortToDeserialize,

    #[error("{received} is not a valid content tag for a repliable message")]
    InvalidRepliableContentTag { received: u8 },

    #[error("{received} is not a valid content tag for a reply message")]
    InvalidReplyContentTag { received: u8 },

    #[error("failed to deserialize recipient information - {0}")]
    MalformedRecipient(#[from] RecipientFormattingError),

    #[error("failed to deserialize replySURB - {0}")]
    MalformedReplySurb(#[from] ReplySurbError),
}

#[derive(Debug)]
pub struct RepliableMessage {
    pub sender_tag: AnonymousSenderTag,
    pub content: RepliableMessageContent,
}

impl Display for RepliableMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.content {
            RepliableMessageContent::Data {
                message,
                reply_surbs,
            } => write!(
                f,
                "repliable {:.2} kiB data message with {} reply surbs attached from {}",
                message.len() as f64 / 1024.0,
                reply_surbs.len(),
                self.sender_tag,
            ),
            RepliableMessageContent::AdditionalSurbs { reply_surbs } => write!(
                f,
                "repliable additional surbs message ({} reply surbs attached) from {}",
                reply_surbs.len(),
                self.sender_tag,
            ),
            RepliableMessageContent::Heartbeat {
                additional_reply_surbs,
            } => {
                write!(
                    f,
                    "repliable heartbeat message ({} reply surbs attached) from {}",
                    additional_reply_surbs.len(),
                    self.sender_tag,
                )
            }
        }
    }
}

impl RepliableMessage {
    pub fn new_data(
        data: Vec<u8>,
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
    ) -> Self {
        RepliableMessage {
            sender_tag,
            content: RepliableMessageContent::Data {
                message: data,
                reply_surbs,
            },
        }
    }

    pub fn new_additional_surbs(
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
    ) -> Self {
        RepliableMessage {
            sender_tag,
            content: RepliableMessageContent::AdditionalSurbs { reply_surbs },
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let content_tag = self.content.tag();

        self.sender_tag
            .to_bytes()
            .into_iter()
            .chain(std::iter::once(content_tag as u8))
            .chain(self.content.into_bytes())
            .collect()
    }

    pub fn try_from_bytes(
        bytes: &[u8],
        num_mix_hops: u8,
    ) -> Result<Self, InvalidReplyRequestError> {
        if bytes.len() < SENDER_TAG_SIZE + 1 {
            return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
        }
        let sender_tag =
            AnonymousSenderTag::from_bytes(bytes[..SENDER_TAG_SIZE].try_into().unwrap());
        let content_tag = RepliableMessageContentTag::try_from(bytes[SENDER_TAG_SIZE])?;

        let content = RepliableMessageContent::try_from_bytes(
            &bytes[SENDER_TAG_SIZE + 1..],
            num_mix_hops,
            content_tag,
        )?;

        Ok(RepliableMessage {
            sender_tag,
            content,
        })
    }

    pub fn serialized_size(&self, num_mix_hops: u8) -> usize {
        let content_type_size = 1;
        SENDER_TAG_SIZE + content_type_size + self.content.serialized_size(num_mix_hops)
    }
}

// this recovery code is shared between all variants containing reply surbs
fn recover_reply_surbs(
    bytes: &[u8],
    num_mix_hops: u8,
) -> Result<(Vec<ReplySurb>, usize), InvalidReplyRequestError> {
    let mut consumed = mem::size_of::<u32>();
    if bytes.len() < consumed {
        return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
    }
    let num_surbs = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let surb_size = ReplySurb::serialized_len(num_mix_hops);
    if bytes[consumed..].len() < num_surbs as usize * surb_size {
        return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
    }

    let mut reply_surbs = Vec::with_capacity(num_surbs as usize);
    for _ in 0..num_surbs as usize {
        let surb_bytes = &bytes[consumed..consumed + surb_size];
        let reply_surb = ReplySurb::from_bytes(surb_bytes)?;
        reply_surbs.push(reply_surb);

        consumed += surb_size;
    }

    Ok((reply_surbs, consumed))
}

#[repr(u8)]
enum RepliableMessageContentTag {
    Data = 0,
    AdditionalSurbs = 1,
    Heartbeat = 2,
}

impl TryFrom<u8> for RepliableMessageContentTag {
    type Error = InvalidReplyRequestError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (RepliableMessageContentTag::Data as u8) => Ok(Self::Data),
            _ if value == (RepliableMessageContentTag::AdditionalSurbs as u8) => {
                Ok(Self::AdditionalSurbs)
            }
            _ if value == (RepliableMessageContentTag::Heartbeat as u8) => Ok(Self::Heartbeat),
            val => Err(InvalidReplyRequestError::InvalidRepliableContentTag { received: val }),
        }
    }
}

// sent by original sender that initialised the communication that knows address of the remote
#[derive(Debug)]
pub enum RepliableMessageContent {
    Data {
        message: Vec<u8>,
        reply_surbs: Vec<ReplySurb>,
    },
    AdditionalSurbs {
        reply_surbs: Vec<ReplySurb>,
    },
    Heartbeat {
        additional_reply_surbs: Vec<ReplySurb>,
    },
}

impl RepliableMessageContent {
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            RepliableMessageContent::Data {
                message,
                reply_surbs,
            } => {
                let num_surbs = reply_surbs.len() as u32;

                num_surbs
                    .to_be_bytes()
                    .into_iter()
                    .chain(reply_surbs.into_iter().flat_map(|s| s.to_bytes()))
                    .chain(message)
                    .collect()
            }
            RepliableMessageContent::AdditionalSurbs { reply_surbs } => {
                let num_surbs = reply_surbs.len() as u32;

                num_surbs
                    .to_be_bytes()
                    .into_iter()
                    .chain(reply_surbs.into_iter().flat_map(|s| s.to_bytes()))
                    .collect()
            }
            RepliableMessageContent::Heartbeat {
                additional_reply_surbs,
            } => {
                let num_surbs = additional_reply_surbs.len() as u32;

                num_surbs
                    .to_be_bytes()
                    .into_iter()
                    .chain(
                        additional_reply_surbs
                            .into_iter()
                            .flat_map(|s| s.to_bytes()),
                    )
                    .collect()
            }
        }
    }

    fn try_from_bytes(
        bytes: &[u8],
        num_mix_hops: u8,
        tag: RepliableMessageContentTag,
    ) -> Result<Self, InvalidReplyRequestError> {
        if bytes.is_empty() {
            return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
        }

        let (reply_surbs, n) = recover_reply_surbs(bytes, num_mix_hops)?;

        match tag {
            RepliableMessageContentTag::Data => Ok(RepliableMessageContent::Data {
                message: bytes[n..].to_vec(),
                reply_surbs,
            }),
            RepliableMessageContentTag::AdditionalSurbs => {
                Ok(RepliableMessageContent::AdditionalSurbs { reply_surbs })
            }
            RepliableMessageContentTag::Heartbeat => Ok(RepliableMessageContent::Heartbeat {
                additional_reply_surbs: reply_surbs,
            }),
        }
    }

    fn tag(&self) -> RepliableMessageContentTag {
        match self {
            RepliableMessageContent::Data { .. } => RepliableMessageContentTag::Data,
            RepliableMessageContent::AdditionalSurbs { .. } => {
                RepliableMessageContentTag::AdditionalSurbs
            }
            RepliableMessageContent::Heartbeat { .. } => RepliableMessageContentTag::Heartbeat,
        }
    }

    fn serialized_size(&self, num_mix_hops: u8) -> usize {
        match self {
            RepliableMessageContent::Data {
                message,
                reply_surbs,
            } => {
                let num_reply_surbs_tag = mem::size_of::<u32>();
                num_reply_surbs_tag
                    + reply_surbs.len() * ReplySurb::serialized_len(num_mix_hops)
                    + message.len()
            }
            RepliableMessageContent::AdditionalSurbs { reply_surbs } => {
                let num_reply_surbs_tag = mem::size_of::<u32>();
                num_reply_surbs_tag + reply_surbs.len() * ReplySurb::serialized_len(num_mix_hops)
            }
            RepliableMessageContent::Heartbeat {
                additional_reply_surbs,
            } => {
                let num_reply_surbs_tag = mem::size_of::<u32>();
                num_reply_surbs_tag
                    + additional_reply_surbs.len() * ReplySurb::serialized_len(num_mix_hops)
            }
        }
    }
}

// sent by the remote party who does **NOT** know the original sender's identity
#[derive(Debug)]
pub struct ReplyMessage {
    pub content: ReplyMessageContent,
}

impl Display for ReplyMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.content {
            ReplyMessageContent::Data { message } => write!(
                f,
                "{:.2} kiB reply data message",
                message.len() as f64 / 1024.0
            ),
            ReplyMessageContent::SurbRequest { recipient, amount } => write!(
                f,
                "request for {amount} additional reply SURBs from {recipient}",
            ),
        }
    }
}

impl ReplyMessage {
    pub fn new_data_message(message: Vec<u8>) -> Self {
        ReplyMessage {
            content: ReplyMessageContent::Data { message },
        }
    }

    pub fn new_surb_request_message(recipient: Recipient, amount: u32) -> Self {
        ReplyMessage {
            content: ReplyMessageContent::SurbRequest {
                recipient: Box::new(recipient),
                amount,
            },
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let content_tag = self.content.tag();

        std::iter::once(content_tag as u8)
            .chain(self.content.into_bytes())
            .collect()
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, InvalidReplyRequestError> {
        if bytes.is_empty() {
            return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
        }
        let tag = ReplyMessageContentTag::try_from(bytes[0])?;
        let content = ReplyMessageContent::try_from_bytes(&bytes[1..], tag)?;

        Ok(ReplyMessage { content })
    }

    pub fn serialized_size(&self) -> usize {
        let content_type_size = 1;
        content_type_size + self.content.serialized_size()
    }
}

#[repr(u8)]
enum ReplyMessageContentTag {
    Data = 0,
    SurbRequest = 1,
}

impl TryFrom<u8> for ReplyMessageContentTag {
    type Error = InvalidReplyRequestError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (ReplyMessageContentTag::Data as u8) => Ok(Self::Data),
            _ if value == (ReplyMessageContentTag::SurbRequest as u8) => Ok(Self::SurbRequest),
            val => Err(InvalidReplyRequestError::InvalidReplyContentTag { received: val }),
        }
    }
}

#[derive(Debug)]
pub enum ReplyMessageContent {
    // TODO: later allow to request surbs whilst sending data
    Data {
        message: Vec<u8>,
    },
    SurbRequest {
        recipient: Box<Recipient>,
        amount: u32,
    },
}

impl ReplyMessageContent {
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            ReplyMessageContent::Data { message } => message,
            ReplyMessageContent::SurbRequest { recipient, amount } => recipient
                .to_bytes()
                .into_iter()
                .chain(amount.to_be_bytes())
                .collect(),
        }
    }

    fn try_from_bytes(
        bytes: &[u8],
        tag: ReplyMessageContentTag,
    ) -> Result<Self, InvalidReplyRequestError> {
        if bytes.is_empty() {
            return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
        }

        match tag {
            ReplyMessageContentTag::Data => Ok(ReplyMessageContent::Data {
                message: bytes.to_vec(),
            }),
            ReplyMessageContentTag::SurbRequest => {
                if bytes.len() != Recipient::LEN + std::mem::size_of::<u32>() {
                    return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
                }
                let mut recipient_bytes = [0u8; Recipient::LEN];
                recipient_bytes.copy_from_slice(&bytes[..Recipient::LEN]);

                Ok(ReplyMessageContent::SurbRequest {
                    recipient: Box::new(Recipient::try_from_bytes(recipient_bytes)?),
                    amount: u32::from_be_bytes([
                        bytes[Recipient::LEN],
                        bytes[Recipient::LEN + 1],
                        bytes[Recipient::LEN + 2],
                        bytes[Recipient::LEN + 3],
                    ]),
                })
            }
        }
    }

    fn tag(&self) -> ReplyMessageContentTag {
        match self {
            ReplyMessageContent::Data { .. } => ReplyMessageContentTag::Data,
            ReplyMessageContent::SurbRequest { .. } => ReplyMessageContentTag::SurbRequest,
        }
    }

    pub fn serialized_size(&self) -> usize {
        match self {
            ReplyMessageContent::Data { message } => message.len(),
            ReplyMessageContent::SurbRequest { amount, .. } => {
                let amount_marker = mem::size_of_val(amount);
                Recipient::LEN + amount_marker
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fixtures {
        use crate::requests::{AnonymousSenderTag, RepliableMessageContent, ReplyMessageContent};
        use crate::{ReplySurb, SurbEncryptionKey};
        use nym_crypto::asymmetric::{encryption, identity};
        use nym_sphinx_addressing::clients::Recipient;
        use nym_sphinx_types::{
            Delay, Destination, DestinationAddressBytes, Node, NodeAddressBytes, PrivateKey,
            SURBMaterial, NODE_ADDRESS_LENGTH,
        };
        use rand::{Rng, RngCore};
        use rand_chacha::rand_core::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        pub(super) fn test_rng() -> ChaCha20Rng {
            let dummy_seed = [42u8; 32];
            ChaCha20Rng::from_seed(dummy_seed)
        }

        pub(super) fn random_vec_u8(rng: &mut ChaCha20Rng, n: usize) -> Vec<u8> {
            let mut vec = Vec::with_capacity(n);
            for _ in 0..n {
                vec.push(rng.gen())
            }
            vec
        }

        pub(super) fn sender_tag(rng: &mut ChaCha20Rng) -> AnonymousSenderTag {
            AnonymousSenderTag::new_random(rng)
        }

        pub(super) fn recipient(rng: &mut ChaCha20Rng) -> Recipient {
            let client_id = identity::KeyPair::new(rng);
            let client_enc = encryption::KeyPair::new(rng);
            let gateway_id = identity::KeyPair::new(rng);

            Recipient::new(
                *client_id.public_key(),
                *client_enc.public_key(),
                *gateway_id.public_key(),
            )
        }

        fn node(rng: &mut ChaCha20Rng) -> Node {
            let mut address_bytes = [0; NODE_ADDRESS_LENGTH];
            rng.fill_bytes(&mut address_bytes);

            let dummy_private = PrivateKey::new_with_rng(rng);
            let pub_key = (&dummy_private).into();
            Node {
                address: NodeAddressBytes::from_bytes(address_bytes),
                pub_key,
            }
        }

        pub(super) fn reply_surb(rng: &mut ChaCha20Rng, num_mix_hops: u8) -> ReplySurb {
            // due to gateway
            let num_hops = num_mix_hops + 1;
            let route = (0..num_hops).map(|_| node(rng)).collect();
            let delays = (0..num_hops)
                .map(|_| Delay::new_from_nanos(rng.next_u64()))
                .collect();
            let mut destination_bytes = [0u8; 32];
            rng.fill_bytes(&mut destination_bytes);

            let mut identifier_bytes = [0u8; 16];
            rng.fill_bytes(&mut identifier_bytes);

            let destination = Destination::new(
                DestinationAddressBytes::from_bytes(destination_bytes),
                identifier_bytes,
            );

            let surb = SURBMaterial::new(route, delays, destination)
                .construct_SURB()
                .unwrap();
            ReplySurb {
                surb,
                encryption_key: SurbEncryptionKey::new(rng),
            }
        }

        pub(super) fn reply_surbs(
            rng: &mut ChaCha20Rng,
            num_mix_hops: u8,
            n: usize,
        ) -> Vec<ReplySurb> {
            let mut surbs = Vec::with_capacity(n);
            for _ in 0..n {
                surbs.push(reply_surb(rng, num_mix_hops))
            }
            surbs
        }

        pub(super) fn repliable_content_data(
            rng: &mut ChaCha20Rng,
            num_mix_hops: u8,
            msg_len: usize,
            surbs: usize,
        ) -> RepliableMessageContent {
            RepliableMessageContent::Data {
                message: random_vec_u8(rng, msg_len),
                reply_surbs: reply_surbs(rng, num_mix_hops, surbs),
            }
        }

        pub(super) fn repliable_content_surbs(
            rng: &mut ChaCha20Rng,
            num_mix_hops: u8,
            surbs: usize,
        ) -> RepliableMessageContent {
            RepliableMessageContent::AdditionalSurbs {
                reply_surbs: reply_surbs(rng, num_mix_hops, surbs),
            }
        }

        pub(super) fn repliable_content_heartbeat(
            rng: &mut ChaCha20Rng,
            num_mix_hops: u8,
            surbs: usize,
        ) -> RepliableMessageContent {
            RepliableMessageContent::Heartbeat {
                additional_reply_surbs: reply_surbs(rng, num_mix_hops, surbs),
            }
        }

        pub(super) fn reply_content_data(
            rng: &mut ChaCha20Rng,
            msg_len: usize,
        ) -> ReplyMessageContent {
            ReplyMessageContent::Data {
                message: random_vec_u8(rng, msg_len),
            }
        }

        pub(super) fn reply_content_surbs(
            rng: &mut ChaCha20Rng,
            surbs: u32,
        ) -> ReplyMessageContent {
            ReplyMessageContent::SurbRequest {
                recipient: Box::new(recipient(rng)),
                amount: surbs,
            }
        }
    }

    #[cfg(test)]
    mod repliable_message {
        use super::*;

        #[test]
        fn serialized_size_matches_actual_serialization() {
            let mut rng = fixtures::test_rng();
            let num_mix_hops = 3;

            let data1 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data(&mut rng, num_mix_hops, 10000, 0),
            };
            assert_eq!(
                data1.serialized_size(num_mix_hops),
                data1.into_bytes().len()
            );

            let data2 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data(&mut rng, num_mix_hops, 10, 100),
            };
            assert_eq!(
                data2.serialized_size(num_mix_hops),
                data2.into_bytes().len()
            );

            let data3 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data(&mut rng, num_mix_hops, 100000, 1000),
            };
            assert_eq!(
                data3.serialized_size(num_mix_hops),
                data3.into_bytes().len()
            );

            let additional_surbs1 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_surbs(&mut rng, num_mix_hops, 1),
            };
            assert_eq!(
                additional_surbs1.serialized_size(num_mix_hops),
                additional_surbs1.into_bytes().len()
            );

            let additional_surbs2 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_surbs(&mut rng, num_mix_hops, 1000),
            };
            assert_eq!(
                additional_surbs2.serialized_size(num_mix_hops),
                additional_surbs2.into_bytes().len()
            );

            let heartbeat1 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_heartbeat(&mut rng, num_mix_hops, 1),
            };
            assert_eq!(
                heartbeat1.serialized_size(num_mix_hops),
                heartbeat1.into_bytes().len()
            );

            let heartbeat2 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_heartbeat(&mut rng, num_mix_hops, 1000),
            };
            assert_eq!(
                heartbeat2.serialized_size(num_mix_hops),
                heartbeat2.into_bytes().len()
            );
        }
    }

    #[cfg(test)]
    mod repliable_message_content {
        use super::*;

        #[test]
        fn serialized_size_matches_actual_serialization() {
            let mut rng = fixtures::test_rng();
            let num_mix_hops = 3;

            let data1 = fixtures::repliable_content_data(&mut rng, num_mix_hops, 10000, 0);
            assert_eq!(
                data1.serialized_size(num_mix_hops),
                data1.into_bytes().len()
            );

            let data2 = fixtures::repliable_content_data(&mut rng, num_mix_hops, 10, 100);
            assert_eq!(
                data2.serialized_size(num_mix_hops),
                data2.into_bytes().len()
            );

            let data3 = fixtures::repliable_content_data(&mut rng, num_mix_hops, 100000, 1000);
            assert_eq!(
                data3.serialized_size(num_mix_hops),
                data3.into_bytes().len()
            );

            let additional_surbs1 = fixtures::repliable_content_surbs(&mut rng, num_mix_hops, 1);
            assert_eq!(
                additional_surbs1.serialized_size(num_mix_hops),
                additional_surbs1.into_bytes().len()
            );

            let additional_surbs2 = fixtures::repliable_content_surbs(&mut rng, num_mix_hops, 1000);
            assert_eq!(
                additional_surbs2.serialized_size(num_mix_hops),
                additional_surbs2.into_bytes().len()
            );

            let heartbeat1 = fixtures::repliable_content_heartbeat(&mut rng, num_mix_hops, 1);
            assert_eq!(
                heartbeat1.serialized_size(num_mix_hops),
                heartbeat1.into_bytes().len()
            );

            let heartbeat2 = fixtures::repliable_content_heartbeat(&mut rng, num_mix_hops, 1000);
            assert_eq!(
                heartbeat2.serialized_size(num_mix_hops),
                heartbeat2.into_bytes().len()
            );
        }
    }

    #[cfg(test)]
    mod reply_message {
        use super::*;

        #[test]
        fn serialized_size_matches_actual_serialization() {
            let mut rng = fixtures::test_rng();

            let data1 = ReplyMessage {
                content: fixtures::reply_content_data(&mut rng, 100),
            };
            assert_eq!(data1.serialized_size(), data1.into_bytes().len());

            let data2 = ReplyMessage {
                content: fixtures::reply_content_data(&mut rng, 100000),
            };
            assert_eq!(data2.serialized_size(), data2.into_bytes().len());

            let surbs1 = ReplyMessage {
                content: fixtures::reply_content_surbs(&mut rng, 12),
            };
            assert_eq!(surbs1.serialized_size(), surbs1.into_bytes().len());

            let surbs2 = ReplyMessage {
                content: fixtures::reply_content_surbs(&mut rng, 1000),
            };
            assert_eq!(surbs2.serialized_size(), surbs2.into_bytes().len());
        }
    }

    #[cfg(test)]
    mod reply_message_content {
        use super::*;

        #[test]
        fn serialized_size_matches_actual_serialization() {
            let mut rng = fixtures::test_rng();

            let data1 = fixtures::reply_content_data(&mut rng, 100);
            assert_eq!(data1.serialized_size(), data1.into_bytes().len());

            let data2 = fixtures::reply_content_data(&mut rng, 100000);
            assert_eq!(data2.serialized_size(), data2.into_bytes().len());

            let surbs1 = fixtures::reply_content_surbs(&mut rng, 12);
            assert_eq!(surbs1.serialized_size(), surbs1.into_bytes().len());

            let surbs2 = fixtures::reply_content_surbs(&mut rng, 1000);
            assert_eq!(surbs2.serialized_size(), surbs2.into_bytes().len());
        }
    }
}
