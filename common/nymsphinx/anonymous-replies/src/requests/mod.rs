// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{ReplySurb, ReplySurbError};
use nym_sphinx_addressing::clients::{Recipient, RecipientFormattingError};
use rand::{CryptoRng, RngCore};
use std::fmt::{Display, Formatter};
use std::mem;
use thiserror::Error;

use crate::requests::v1::{AdditionalSurbsV1, DataV1, HeartbeatV1};
use crate::requests::v2::{AdditionalSurbsV2, DataV2, HeartbeatV2};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub(crate) mod v1;
pub(crate) mod v2;

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
            RepliableMessageContent::Data(content) => {
                write!(f, "{content} from {}", self.sender_tag)
            }
            RepliableMessageContent::AdditionalSurbs(content) => {
                write!(f, "{content} from {}", self.sender_tag)
            }
            RepliableMessageContent::Heartbeat(content) => {
                write!(f, "{content} from {}", self.sender_tag)
            }
            RepliableMessageContent::DataV2(content) => {
                write!(f, "{content} from {}", self.sender_tag)
            }
            RepliableMessageContent::AdditionalSurbsV2(content) => {
                write!(f, "{content} from {}", self.sender_tag)
            }
            RepliableMessageContent::HeartbeatV2(content) => {
                write!(f, "{content} from {}", self.sender_tag)
            }
        }
    }
}

impl RepliableMessage {
    pub fn new_data(
        use_legacy_surb_format: bool,
        data: Vec<u8>,
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
    ) -> Self {
        let content = if use_legacy_surb_format {
            RepliableMessageContent::Data(DataV1 {
                message: data,
                reply_surbs,
            })
        } else {
            RepliableMessageContent::DataV2(DataV2 {
                message: data,
                reply_surbs,
            })
        };

        RepliableMessage {
            sender_tag,
            content,
        }
    }

    pub fn new_additional_surbs(
        use_legacy_surb_format: bool,
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
    ) -> Self {
        let content = if use_legacy_surb_format {
            RepliableMessageContent::AdditionalSurbs(AdditionalSurbsV1 { reply_surbs })
        } else {
            RepliableMessageContent::AdditionalSurbsV2(AdditionalSurbsV2 { reply_surbs })
        };

        RepliableMessage {
            sender_tag,
            content,
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

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, InvalidReplyRequestError> {
        if bytes.len() < SENDER_TAG_SIZE + 1 {
            return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
        }
        let sender_tag =
            AnonymousSenderTag::from_bytes(bytes[..SENDER_TAG_SIZE].try_into().unwrap());
        let content_tag = RepliableMessageContentTag::try_from(bytes[SENDER_TAG_SIZE])?;

        let content =
            RepliableMessageContent::try_from_bytes(&bytes[SENDER_TAG_SIZE + 1..], content_tag)?;

        Ok(RepliableMessage {
            sender_tag,
            content,
        })
    }

    pub fn serialized_size(&self) -> usize {
        let content_type_size = 1;
        SENDER_TAG_SIZE + content_type_size + self.content.serialized_size()
    }
}

#[derive(Debug)]
#[repr(u8)]
enum RepliableMessageContentTag {
    Data = 0,
    AdditionalSurbs = 1,
    Heartbeat = 2,

    // updated variants that slightly change SURB encoding
    // to allow for variable number of hops as well as using payload key seeds
    DataV2 = 3,
    AdditionalSurbsV2 = 4,
    HeartbeatV2 = 5,
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
            _ if value == (RepliableMessageContentTag::DataV2 as u8) => Ok(Self::DataV2),
            _ if value == (RepliableMessageContentTag::AdditionalSurbsV2 as u8) => {
                Ok(Self::AdditionalSurbsV2)
            }
            _ if value == (RepliableMessageContentTag::HeartbeatV2 as u8) => Ok(Self::HeartbeatV2),
            val => Err(InvalidReplyRequestError::InvalidRepliableContentTag { received: val }),
        }
    }
}

// sent by original sender that initialised the communication that knows address of the remote
#[derive(Debug)]
pub enum RepliableMessageContent {
    Data(DataV1),
    AdditionalSurbs(AdditionalSurbsV1),
    Heartbeat(HeartbeatV1),

    DataV2(DataV2),
    AdditionalSurbsV2(AdditionalSurbsV2),
    HeartbeatV2(HeartbeatV2),
}

impl RepliableMessageContent {
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            RepliableMessageContent::Data(content) => content.into_bytes(),
            RepliableMessageContent::AdditionalSurbs(content) => content.into_bytes(),
            RepliableMessageContent::Heartbeat(content) => content.into_bytes(),
            RepliableMessageContent::DataV2(content) => content.into_bytes(),
            RepliableMessageContent::AdditionalSurbsV2(content) => content.into_bytes(),
            RepliableMessageContent::HeartbeatV2(content) => content.into_bytes(),
        }
    }

    fn try_from_bytes(
        bytes: &[u8],
        tag: RepliableMessageContentTag,
    ) -> Result<Self, InvalidReplyRequestError> {
        if bytes.is_empty() {
            return Err(InvalidReplyRequestError::RequestTooShortToDeserialize);
        }

        match tag {
            RepliableMessageContentTag::Data => {
                Ok(RepliableMessageContent::Data(DataV1::from_bytes(bytes)?))
            }
            RepliableMessageContentTag::AdditionalSurbs => Ok(
                RepliableMessageContent::AdditionalSurbs(AdditionalSurbsV1::from_bytes(bytes)?),
            ),
            RepliableMessageContentTag::Heartbeat => Ok(RepliableMessageContent::Heartbeat(
                HeartbeatV1::from_bytes(bytes)?,
            )),
            RepliableMessageContentTag::DataV2 => {
                Ok(RepliableMessageContent::DataV2(DataV2::from_bytes(bytes)?))
            }
            RepliableMessageContentTag::AdditionalSurbsV2 => Ok(
                RepliableMessageContent::AdditionalSurbsV2(AdditionalSurbsV2::from_bytes(bytes)?),
            ),
            RepliableMessageContentTag::HeartbeatV2 => Ok(RepliableMessageContent::HeartbeatV2(
                HeartbeatV2::from_bytes(bytes)?,
            )),
        }
    }

    fn tag(&self) -> RepliableMessageContentTag {
        match self {
            RepliableMessageContent::Data { .. } => RepliableMessageContentTag::Data,
            RepliableMessageContent::AdditionalSurbs { .. } => {
                RepliableMessageContentTag::AdditionalSurbs
            }
            RepliableMessageContent::Heartbeat { .. } => RepliableMessageContentTag::Heartbeat,
            RepliableMessageContent::DataV2(_) => RepliableMessageContentTag::DataV2,
            RepliableMessageContent::AdditionalSurbsV2(_) => {
                RepliableMessageContentTag::AdditionalSurbsV2
            }
            RepliableMessageContent::HeartbeatV2(_) => RepliableMessageContentTag::HeartbeatV2,
        }
    }

    fn serialized_size(&self) -> usize {
        match self {
            RepliableMessageContent::Data(content) => content.serialized_len(),
            RepliableMessageContent::AdditionalSurbs(content) => content.serialized_len(),
            RepliableMessageContent::Heartbeat(content) => content.serialized_len(),
            RepliableMessageContent::DataV2(content) => content.serialized_len(),
            RepliableMessageContent::AdditionalSurbsV2(content) => content.serialized_len(),
            RepliableMessageContent::HeartbeatV2(content) => content.serialized_len(),
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
        use crate::requests::v1::{AdditionalSurbsV1, DataV1, HeartbeatV1};
        use crate::requests::v2::{AdditionalSurbsV2, DataV2, HeartbeatV2};
        use crate::requests::{AnonymousSenderTag, RepliableMessageContent, ReplyMessageContent};
        use crate::{ReplySurb, SurbEncryptionKey};
        use nym_crypto::asymmetric::{ed25519, x25519};
        use nym_sphinx_addressing::clients::Recipient;
        use nym_sphinx_types::{
            Delay, Destination, DestinationAddressBytes, Node, NodeAddressBytes, PrivateKey,
            SURBMaterial, NODE_ADDRESS_LENGTH, X25519_WITH_EXPLICIT_PAYLOAD_KEYS_VERSION,
        };
        use rand::{Rng, RngCore};
        use rand_chacha::rand_core::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        pub(crate) const LEGACY_HOPS: u8 = 4;

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
            let client_id = ed25519::KeyPair::new(rng);
            let client_enc = x25519::KeyPair::new(rng);
            let gateway_id = ed25519::KeyPair::new(rng);

            Recipient::new(
                *client_id.public_key(),
                *client_enc.public_key(),
                *gateway_id.public_key(),
            )
        }

        fn node(rng: &mut ChaCha20Rng) -> Node {
            let mut address_bytes = [0; NODE_ADDRESS_LENGTH];
            rng.fill_bytes(&mut address_bytes);

            let dummy_private = PrivateKey::random_from_rng(rng);
            let pub_key = (&dummy_private).into();
            Node {
                address: NodeAddressBytes::from_bytes(address_bytes),
                pub_key,
            }
        }

        pub(super) fn reply_surb(rng: &mut ChaCha20Rng, legacy: bool, hops: u8) -> ReplySurb {
            let route = (0..hops).map(|_| node(rng)).collect();
            let delays = (0..hops)
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

            let mut surb_material = SURBMaterial::new(route, delays, destination);
            if legacy {
                surb_material =
                    surb_material.with_version(X25519_WITH_EXPLICIT_PAYLOAD_KEYS_VERSION);
            }

            ReplySurb {
                surb: surb_material.construct_SURB().unwrap(),
                encryption_key: SurbEncryptionKey::new(rng),
            }
        }

        pub(super) fn reply_surbs(
            rng: &mut ChaCha20Rng,
            n: usize,
            legacy: bool,
            hops: u8,
        ) -> Vec<ReplySurb> {
            let mut surbs = Vec::with_capacity(n);
            for _ in 0..n {
                surbs.push(reply_surb(rng, legacy, hops))
            }
            surbs
        }

        pub(super) fn repliable_content_data_v1(
            rng: &mut ChaCha20Rng,
            msg_len: usize,
            surbs: usize,
        ) -> RepliableMessageContent {
            RepliableMessageContent::Data(DataV1 {
                message: random_vec_u8(rng, msg_len),
                reply_surbs: reply_surbs(rng, surbs, true, LEGACY_HOPS),
            })
        }

        pub(super) fn repliable_content_surbs_v1(
            rng: &mut ChaCha20Rng,
            surbs: usize,
        ) -> RepliableMessageContent {
            RepliableMessageContent::AdditionalSurbs(AdditionalSurbsV1 {
                reply_surbs: reply_surbs(rng, surbs, true, LEGACY_HOPS),
            })
        }

        pub(super) fn repliable_content_heartbeat_v1(
            rng: &mut ChaCha20Rng,
            surbs: usize,
        ) -> RepliableMessageContent {
            RepliableMessageContent::Heartbeat(HeartbeatV1 {
                additional_reply_surbs: reply_surbs(rng, surbs, true, LEGACY_HOPS),
            })
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

        pub(super) fn repliable_content_data_v2(
            rng: &mut ChaCha20Rng,
            msg_len: usize,
            surbs: usize,
            surb_hops: u8,
        ) -> RepliableMessageContent {
            RepliableMessageContent::DataV2(DataV2 {
                message: random_vec_u8(rng, msg_len),
                reply_surbs: reply_surbs(rng, surbs, false, surb_hops),
            })
        }

        pub(super) fn repliable_content_surbs_v2(
            rng: &mut ChaCha20Rng,
            surbs: usize,
            surb_hops: u8,
        ) -> RepliableMessageContent {
            RepliableMessageContent::AdditionalSurbsV2(AdditionalSurbsV2 {
                reply_surbs: reply_surbs(rng, surbs, false, surb_hops),
            })
        }

        pub(super) fn repliable_content_heartbeat_v2(
            rng: &mut ChaCha20Rng,
            surbs: usize,
            surb_hops: u8,
        ) -> RepliableMessageContent {
            RepliableMessageContent::HeartbeatV2(HeartbeatV2 {
                additional_reply_surbs: reply_surbs(rng, surbs, false, surb_hops),
            })
        }
    }

    #[cfg(test)]
    mod repliable_message {
        use super::*;
        use crate::requests::tests::fixtures::LEGACY_HOPS;

        #[test]
        fn serialized_size_matches_actual_serialization_for_v1_messages() {
            let mut rng = fixtures::test_rng();

            let data1 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data_v1(&mut rng, 10000, 0),
            };
            assert_eq!(data1.serialized_size(), data1.into_bytes().len());

            let data2 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data_v1(&mut rng, 10, 100),
            };
            assert_eq!(data2.serialized_size(), data2.into_bytes().len());

            let data3 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data_v1(&mut rng, 100000, 1000),
            };
            assert_eq!(data3.serialized_size(), data3.into_bytes().len());

            let additional_surbs1 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_surbs_v1(&mut rng, 1),
            };
            assert_eq!(
                additional_surbs1.serialized_size(),
                additional_surbs1.into_bytes().len()
            );

            let additional_surbs2 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_surbs_v1(&mut rng, 1000),
            };
            assert_eq!(
                additional_surbs2.serialized_size(),
                additional_surbs2.into_bytes().len()
            );

            let heartbeat1 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_heartbeat_v1(&mut rng, 1),
            };
            assert_eq!(heartbeat1.serialized_size(), heartbeat1.into_bytes().len());

            let heartbeat2 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_heartbeat_v1(&mut rng, 1000),
            };
            assert_eq!(heartbeat2.serialized_size(), heartbeat2.into_bytes().len());
        }

        #[test]
        fn serialized_size_matches_actual_serialization_for_v2_messages() {
            let mut rng = fixtures::test_rng();

            let data1 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data_v2(&mut rng, 10000, 0, LEGACY_HOPS),
            };
            assert_eq!(data1.serialized_size(), data1.into_bytes().len());

            let data2 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data_v2(&mut rng, 10, 100, LEGACY_HOPS),
            };
            assert_eq!(data2.serialized_size(), data2.into_bytes().len());

            let data3 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data_v2(&mut rng, 100000, 1000, LEGACY_HOPS),
            };
            assert_eq!(data3.serialized_size(), data3.into_bytes().len());

            let data4 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_data_v2(&mut rng, 100000, 1000, 1),
            };
            assert_eq!(data4.serialized_size(), data4.into_bytes().len());

            let additional_surbs1 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_surbs_v2(&mut rng, 1, LEGACY_HOPS),
            };
            assert_eq!(
                additional_surbs1.serialized_size(),
                additional_surbs1.into_bytes().len()
            );

            let additional_surbs2 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_surbs_v2(&mut rng, 1000, LEGACY_HOPS),
            };
            assert_eq!(
                additional_surbs2.serialized_size(),
                additional_surbs2.into_bytes().len()
            );

            let additional_surbs3 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_surbs_v2(&mut rng, 1000, 1),
            };
            assert_eq!(
                additional_surbs3.serialized_size(),
                additional_surbs3.into_bytes().len()
            );

            let heartbeat1 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_heartbeat_v2(&mut rng, 1, LEGACY_HOPS),
            };
            assert_eq!(heartbeat1.serialized_size(), heartbeat1.into_bytes().len());

            let heartbeat2 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_heartbeat_v2(&mut rng, 1000, LEGACY_HOPS),
            };
            assert_eq!(heartbeat2.serialized_size(), heartbeat2.into_bytes().len());

            let heartbeat3 = RepliableMessage {
                sender_tag: fixtures::sender_tag(&mut rng),
                content: fixtures::repliable_content_heartbeat_v2(&mut rng, 1000, 1),
            };
            assert_eq!(heartbeat3.serialized_size(), heartbeat3.into_bytes().len());
        }
    }

    #[cfg(test)]
    mod repliable_message_content {
        use super::*;
        use crate::requests::tests::fixtures::LEGACY_HOPS;

        #[test]
        fn serialized_size_matches_actual_serialization_for_v1_messages() {
            let mut rng = fixtures::test_rng();

            let data1 = fixtures::repliable_content_data_v1(&mut rng, 10000, 0);
            assert_eq!(data1.serialized_size(), data1.into_bytes().len());

            let data2 = fixtures::repliable_content_data_v1(&mut rng, 10, 100);
            assert_eq!(data2.serialized_size(), data2.into_bytes().len());

            let data3 = fixtures::repliable_content_data_v1(&mut rng, 100000, 1000);
            assert_eq!(data3.serialized_size(), data3.into_bytes().len());

            let additional_surbs1 = fixtures::repliable_content_surbs_v1(&mut rng, 1);
            assert_eq!(
                additional_surbs1.serialized_size(),
                additional_surbs1.into_bytes().len()
            );

            let additional_surbs2 = fixtures::repliable_content_surbs_v1(&mut rng, 1000);
            assert_eq!(
                additional_surbs2.serialized_size(),
                additional_surbs2.into_bytes().len()
            );

            let heartbeat1 = fixtures::repliable_content_heartbeat_v1(&mut rng, 1);
            assert_eq!(heartbeat1.serialized_size(), heartbeat1.into_bytes().len());

            let heartbeat2 = fixtures::repliable_content_heartbeat_v1(&mut rng, 1000);
            assert_eq!(heartbeat2.serialized_size(), heartbeat2.into_bytes().len());
        }

        #[test]
        fn serialized_size_matches_actual_serialization_for_v2_messages() {
            let mut rng = fixtures::test_rng();

            let data1 = fixtures::repliable_content_data_v2(&mut rng, 10000, 0, LEGACY_HOPS);
            assert_eq!(data1.serialized_size(), data1.into_bytes().len());

            let data2 = fixtures::repliable_content_data_v2(&mut rng, 10, 100, LEGACY_HOPS);
            assert_eq!(data2.serialized_size(), data2.into_bytes().len());

            let data3 = fixtures::repliable_content_data_v2(&mut rng, 100000, 1000, LEGACY_HOPS);
            assert_eq!(data3.serialized_size(), data3.into_bytes().len());

            let data4 = fixtures::repliable_content_data_v2(&mut rng, 100000, 1000, 1);
            assert_eq!(data4.serialized_size(), data4.into_bytes().len());

            let additional_surbs1 = fixtures::repliable_content_surbs_v2(&mut rng, 1, LEGACY_HOPS);
            assert_eq!(
                additional_surbs1.serialized_size(),
                additional_surbs1.into_bytes().len()
            );

            let additional_surbs2 =
                fixtures::repliable_content_surbs_v2(&mut rng, 1000, LEGACY_HOPS);
            assert_eq!(
                additional_surbs2.serialized_size(),
                additional_surbs2.into_bytes().len()
            );

            let additional_surbs3 = fixtures::repliable_content_surbs_v2(&mut rng, 1000, 1);
            assert_eq!(
                additional_surbs3.serialized_size(),
                additional_surbs3.into_bytes().len()
            );

            let heartbeat1 = fixtures::repliable_content_heartbeat_v2(&mut rng, 1, LEGACY_HOPS);
            assert_eq!(heartbeat1.serialized_size(), heartbeat1.into_bytes().len());

            let heartbeat2 = fixtures::repliable_content_heartbeat_v2(&mut rng, 1000, LEGACY_HOPS);
            assert_eq!(heartbeat2.serialized_size(), heartbeat2.into_bytes().len());

            let heartbeat3 = fixtures::repliable_content_heartbeat_v2(&mut rng, 1000, 1);
            assert_eq!(heartbeat3.serialized_size(), heartbeat3.into_bytes().len());
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
