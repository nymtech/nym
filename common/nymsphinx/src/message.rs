// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::chunking;
use crate::receiver::MessageRecoveryError;
use nymsphinx_anonymous_replies::requests::{
    AnonymousSenderTag, RepliableMessage, ReplyMessage, ReplyMessageContent, UnnamedRepliesError,
};
use nymsphinx_anonymous_replies::ReplySurb;
use nymsphinx_chunking::fragment::Fragment;
use rand::Rng;
use std::mem;
use std::vec::IntoIter;

#[derive(Debug)]
pub struct InvalidMessageType;

pub struct UnnamedError;

impl From<UnnamedRepliesError> for UnnamedError {
    fn from(_: UnnamedRepliesError) -> Self {
        todo!()
    }
}

// we have to attach an extra byte of information to indicate number of reply surbs,
// so might as well use this field for other purposes
#[repr(u8)]
enum NymMessageType {
    Plain = 0,
    Repliable = 1,
    Reply = 2,
    // ReplySurbRequest = 1,
}

impl TryFrom<u8> for NymMessageType {
    type Error = UnnamedError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (NymMessageType::Plain as u8) => Ok(Self::Plain),
            _ if value == (NymMessageType::Repliable as u8) => Ok(Self::Repliable),
            _ if value == (NymMessageType::Reply as u8) => Ok(Self::Reply),
            _ => Err(UnnamedError),
        }
    }
}

pub enum NymMessage {
    Plain(Vec<u8>),
    Repliable(RepliableMessage),
    Reply(ReplyMessage),
}

impl NymMessage {
    pub fn new_additional_surbs_request(amount: u32) -> Self {
        NymMessage::Reply(ReplyMessage {
            content: ReplyMessageContent::SurbRequest { amount },
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

    fn try_from_bytes(bytes: &[u8], num_mix_hops: u8) -> Result<Self, UnnamedError> {
        if bytes.is_empty() {
            return Err(UnnamedError);
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

    /// Pads the message so that after it gets chunked, it will occupy exactly N sphinx packets.
    /// Produces new_message = message || 1 || 0000....
    pub fn pad_to_full_packet_lengths(self, plaintext_per_packet: usize) -> PaddedMessage {
        let bytes = self.into_bytes();

        // 1 is added as there will always have to be at least a single byte of padding (1) added
        // to be able to later distinguish the actual padding from the underlying message
        let (_, space_left) =
            chunking::number_of_required_fragments(bytes.len() + 1, plaintext_per_packet);

        bytes
            .into_iter()
            .chain(std::iter::once(1u8))
            .chain(std::iter::repeat(0u8).take(space_left))
            .collect::<Vec<_>>()
            .into()
    }
}

//
// pub struct AnnotatedNymMessage {
//     typ: NymMessageType,
//     // data: NymMessage,
//     pub data: Vec<u8>,
// }
//
// impl AnnotatedNymMessage {
//     pub fn into_bytes(self) -> Vec<u8> {
//         std::iter::once(self.typ as u8)
//             .chain(self.data.into_iter())
//             .collect()
//     }
//
//     pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, UnnamedError> {
//         if bytes.is_empty() {
//             return Err(UnnamedError);
//         }
//         let typ = NymMessageType::try_from(bytes[0]).unwrap_or_else(|_| todo!());
//
//         Ok(AnnotatedNymMessage {
//             typ,
//             data: bytes[1..].to_vec(),
//         })
//     }
//
//     pub fn is_reply_surb_request(&self) -> bool {
//         matches!(MessageType::ReplySurbRequest, self.typ)
//     }
//
//     pub fn into_inner(self) -> Vec<u8> {
//         self.data
//     }
// }
//
// pub struct MessageWithOptionalReplySurbs {
//     pub message: AnnotatedNymMessage,
//     pub sender_tag: AnonymousSenderTag,
//     pub reply_surbs: Vec<ReplySurb>,
// }
//
// impl MessageWithOptionalReplySurbs {
//     pub fn new(message: AnnotatedNymMessage, reply_surbs: Vec<ReplySurb>) -> Self {
//         Self {
//             message,
//             reply_surbs,
//         }
//     }
//
//     // the message is in the format of:
//     // num_surbs (u32) || reply_surbs || data
//     fn into_bytes(self) -> Vec<u8> {
//         let num_surbs = self.reply_surbs.len() as u32;
//
//         num_surbs
//             .to_be_bytes()
//             .into_iter()
//             .chain(self.reply_surbs.into_iter().flat_map(|s| s.to_bytes()))
//             .chain(self.message.into_bytes().into_iter())
//             .collect()
//     }
//
//     /// Pads the message so that after it gets chunked, it will occupy exactly N sphinx packets.
//     /// Produces new_message = message || 1 || 0000....
//     pub fn pad_to_full_packet_lengths(self, plaintext_per_packet: usize) -> PaddedMessage {
//         let bytes = self.into_bytes();
//
//         // 1 is added as there will always have to be at least a single byte of padding (1) added
//         // to be able to later distinguish the actual padding from the underlying message
//         let (_, space_left) =
//             chunking::number_of_required_fragments(bytes.len() + 1, plaintext_per_packet);
//
//         bytes
//             .into_iter()
//             .chain(std::iter::once(1u8))
//             .chain(std::iter::repeat(0u8).take(space_left))
//             .collect::<Vec<_>>()
//             .into()
//     }
// }

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
    pub fn remove_padding(self, num_mix_hops: u8) -> Result<NymMessage, UnnamedError> {
        // we are looking for first occurrence of 1 in the tail and we get its index
        if let Some(padding_end) = self.0.iter().rposition(|b| *b == 1) {
            // and now we only take bytes until that point (but not including it)
            NymMessage::try_from_bytes(&self.0[..padding_end], num_mix_hops)
        } else {
            Err(UnnamedError)
        }
    }
}

impl From<Vec<u8>> for PaddedMessage {
    fn from(bytes: Vec<u8>) -> Self {
        PaddedMessage(bytes)
    }
}
