// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ReplySurb;
use nymsphinx_addressing::clients::Recipient;
use std::mem;

pub struct UnnamedRepliesError;

pub const SENDER_TAG_SIZE: usize = 16;
pub type AnonymousSenderTag = [u8; SENDER_TAG_SIZE];

pub struct RepliableMessage {
    pub sender_tag: AnonymousSenderTag,
    pub content: RepliableMessageContent,
}

impl RepliableMessage {
    // temporary for proof of concept re-implementation with single sender-receiver pair
    #[deprecated]
    pub fn temp_new_data(data: Vec<u8>, reply_surbs: Vec<ReplySurb>) -> Self {
        RepliableMessage {
            sender_tag: [8u8; SENDER_TAG_SIZE],
            content: RepliableMessageContent::Data {
                message: data,
                reply_surbs,
            },
        }
    }

    #[deprecated]
    pub fn temp_new_additional_surbs(reply_surbs: Vec<ReplySurb>) -> Self {
        RepliableMessage {
            sender_tag: [8u8; SENDER_TAG_SIZE],
            content: RepliableMessageContent::AdditionalSurbs { reply_surbs },
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let content_tag = self.content.tag();

        self.sender_tag
            .into_iter()
            .chain(std::iter::once(content_tag as u8))
            .chain(self.content.into_bytes())
            .collect()
    }

    pub fn try_from_bytes(bytes: &[u8], num_mix_hops: u8) -> Result<Self, UnnamedRepliesError> {
        if bytes.len() < SENDER_TAG_SIZE + 1 {
            return Err(UnnamedRepliesError);
        }
        let sender_tag = bytes[..SENDER_TAG_SIZE].try_into().unwrap();
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
}

// this recovery code is shared between all variants containing reply surbs
fn recover_reply_surbs(
    bytes: &[u8],
    num_mix_hops: u8,
) -> Result<(Vec<ReplySurb>, usize), UnnamedRepliesError> {
    let mut consumed = mem::size_of::<u32>();
    if bytes.len() < consumed {
        return Err(UnnamedRepliesError);
    }
    let num_surbs = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let surb_size = ReplySurb::serialized_len(num_mix_hops);
    if bytes[consumed..].len() < num_surbs as usize * surb_size {
        return Err(UnnamedRepliesError);
    }

    let mut reply_surbs = Vec::with_capacity(num_surbs as usize);
    for _ in 0..num_surbs as usize {
        let surb_bytes = &bytes[consumed..consumed + surb_size];
        let reply_surb = ReplySurb::from_bytes(surb_bytes).unwrap_or_else(|_| todo!());
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
    type Error = UnnamedRepliesError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (RepliableMessageContentTag::Data as u8) => Ok(Self::Data),
            _ if value == (RepliableMessageContentTag::AdditionalSurbs as u8) => {
                Ok(Self::AdditionalSurbs)
            }
            _ if value == (RepliableMessageContentTag::Heartbeat as u8) => Ok(Self::Heartbeat),
            _ => Err(UnnamedRepliesError),
        }
    }
}

// sent by original sender that initialised the communication that knows address of the remote
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
                    .chain(message.into_iter())
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
    ) -> Result<Self, UnnamedRepliesError> {
        if bytes.is_empty() {
            return Err(UnnamedRepliesError);
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
}

// sent by the remote party who does **NOT** know the original sender's identity
pub struct ReplyMessage {
    pub content: ReplyMessageContent,
}

impl ReplyMessage {
    pub fn new_data_message(message: Vec<u8>) -> Self {
        ReplyMessage {
            content: ReplyMessageContent::Data { message },
        }
    }

    pub fn new_surb_request_message(recipient: Recipient, amount: u32) -> Self {
        ReplyMessage {
            content: ReplyMessageContent::SurbRequest { recipient, amount },
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let content_tag = self.content.tag();

        std::iter::once(content_tag as u8)
            .chain(self.content.into_bytes())
            .collect()
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, UnnamedRepliesError> {
        if bytes.is_empty() {
            return Err(UnnamedRepliesError);
        }
        let tag = ReplyMessageContentTag::try_from(bytes[0])?;
        let content = ReplyMessageContent::try_from_bytes(&bytes[1..], tag)?;

        Ok(ReplyMessage { content })
    }
}

#[repr(u8)]
enum ReplyMessageContentTag {
    Data = 0,
    SurbRequest = 1,
}

impl TryFrom<u8> for ReplyMessageContentTag {
    type Error = UnnamedRepliesError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (ReplyMessageContentTag::Data as u8) => Ok(Self::Data),
            _ if value == (ReplyMessageContentTag::SurbRequest as u8) => Ok(Self::SurbRequest),
            _ => Err(UnnamedRepliesError),
        }
    }
}

pub enum ReplyMessageContent {
    // TODO: later allow to request surbs whilst sending data
    Data { message: Vec<u8> },
    SurbRequest { recipient: Recipient, amount: u32 },
}

impl ReplyMessageContent {
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            // TODO: a lot of unnecessary allocations
            ReplyMessageContent::Data { message } => message.into_iter().collect(),
            ReplyMessageContent::SurbRequest { recipient, amount } => recipient
                .to_bytes()
                .into_iter()
                .chain(amount.to_be_bytes().into_iter())
                .collect(),
        }
    }

    fn try_from_bytes(
        bytes: &[u8],
        tag: ReplyMessageContentTag,
    ) -> Result<Self, UnnamedRepliesError> {
        if bytes.is_empty() {
            return Err(UnnamedRepliesError);
        }

        match tag {
            ReplyMessageContentTag::Data => Ok(ReplyMessageContent::Data {
                message: bytes.to_vec(),
            }),
            ReplyMessageContentTag::SurbRequest => {
                if bytes.len() != Recipient::LEN + std::mem::size_of::<u32>() {
                    return Err(UnnamedRepliesError);
                }
                let mut recipient_bytes = [0u8; Recipient::LEN];
                recipient_bytes.copy_from_slice(&bytes[..Recipient::LEN]);

                Ok(ReplyMessageContent::SurbRequest {
                    recipient: Recipient::try_from_bytes(recipient_bytes)
                        .map_err(|_| UnnamedRepliesError)?,
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
}
