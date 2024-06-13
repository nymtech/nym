use hyper::body::Buf;
// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;
use serde::{Deserialize, Serialize};
use tokio_util::{
    bytes::BytesMut,
    codec::{Decoder, Encoder},
};

use crate::error::ClientCoreError;

pub type InputMessageSender = tokio_util::sync::PollSender<InputMessage>;
pub type InputMessageReceiver = tokio::sync::mpsc::Receiver<InputMessage>;

#[derive(Serialize, Deserialize, Debug)]
pub enum InputMessage {
    /// Fire an already prepared mix packets into the network.
    /// No guarantees are made about it. For example no retransmssion
    /// will be attempted if it gets dropped.
    Premade {
        msgs: Vec<MixPacket>,
        lane: TransmissionLane,
    },

    /// The simplest message variant where no additional information is attached.
    /// You're simply sending your `data` to specified `recipient` without any tagging.
    ///
    /// Ends up with `NymMessage::Plain` variant
    Regular {
        recipient: Recipient,
        data: Vec<u8>,
        lane: TransmissionLane,
        mix_hops: Option<u8>,
    },

    /// Creates a message used for a duplex anonymous communication where the recipient
    /// will never learn of our true identity. This is achieved by carefully sending `reply_surbs`.
    ///
    /// Note that if reply_surbs is set to zero then
    /// this variant requires the client having sent some reply_surbs in the past
    /// (and thus the recipient also knowing our sender tag).
    ///
    /// Ends up with `NymMessage::Repliable` variant
    Anonymous {
        recipient: Recipient,
        data: Vec<u8>,
        reply_surbs: u32,
        lane: TransmissionLane,
        mix_hops: Option<u8>,
    },

    /// Attempt to use our internally received and stored `ReplySurb` to send the message back
    /// to specified recipient whilst not knowing its full identity (or even gateway).
    ///
    /// Ends up with `NymMessage::Reply` variant
    Reply {
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
    },

    MessageWrapper {
        message: Box<InputMessage>,
        packet_type: PacketType,
    },
}

impl InputMessage {
    pub fn simple(data: &[u8], recipient: Recipient) -> Self {
        InputMessage::new_regular(recipient, data.to_vec(), TransmissionLane::General, None)
    }

    pub fn new_premade(
        msgs: Vec<MixPacket>,
        lane: TransmissionLane,
        packet_type: PacketType,
    ) -> Self {
        let message = InputMessage::Premade { msgs, lane };
        if packet_type == PacketType::Mix {
            message
        } else {
            InputMessage::new_wrapper(message, packet_type)
        }
    }

    pub fn new_wrapper(message: InputMessage, packet_type: PacketType) -> Self {
        InputMessage::MessageWrapper {
            message: Box::new(message),
            packet_type,
        }
    }

    pub fn new_regular(
        recipient: Recipient,
        data: Vec<u8>,
        lane: TransmissionLane,
        packet_type: Option<PacketType>,
    ) -> Self {
        let message = InputMessage::Regular {
            recipient,
            data,
            lane,
            mix_hops: None,
        };
        if let Some(packet_type) = packet_type {
            InputMessage::new_wrapper(message, packet_type)
        } else {
            message
        }
    }

    // IMHO `new_regular` should take `mix_hops: Option<u8>` as an argument instead of creating
    // this function, but that would potentially break backwards compatibility with the current API
    pub fn new_regular_with_custom_hops(
        recipient: Recipient,
        data: Vec<u8>,
        lane: TransmissionLane,
        packet_type: Option<PacketType>,
        mix_hops: Option<u8>,
    ) -> Self {
        let message = InputMessage::Regular {
            recipient,
            data,
            lane,
            mix_hops,
        };
        if let Some(packet_type) = packet_type {
            InputMessage::new_wrapper(message, packet_type)
        } else {
            message
        }
    }

    pub fn new_anonymous(
        recipient: Recipient,
        data: Vec<u8>,
        reply_surbs: u32,
        lane: TransmissionLane,
        packet_type: Option<PacketType>,
    ) -> Self {
        let message = InputMessage::Anonymous {
            recipient,
            data,
            reply_surbs,
            lane,
            mix_hops: None,
        };
        if let Some(packet_type) = packet_type {
            InputMessage::new_wrapper(message, packet_type)
        } else {
            message
        }
    }

    // IMHO `new_anonymous` should take `mix_hops: Option<u8>` as an argument instead of creating
    // this function, but that would potentially break backwards compatibility with the current API
    pub fn new_anonymous_with_custom_hops(
        recipient: Recipient,
        data: Vec<u8>,
        reply_surbs: u32,
        lane: TransmissionLane,
        packet_type: Option<PacketType>,
        mix_hops: Option<u8>,
    ) -> Self {
        let message = InputMessage::Anonymous {
            recipient,
            data,
            reply_surbs,
            lane,
            mix_hops,
        };
        if let Some(packet_type) = packet_type {
            InputMessage::new_wrapper(message, packet_type)
        } else {
            message
        }
    }

    pub fn new_reply(
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
        packet_type: Option<PacketType>,
    ) -> Self {
        let message = InputMessage::Reply {
            recipient_tag,
            data,
            lane,
        };
        if let Some(packet_type) = packet_type {
            InputMessage::new_wrapper(message, packet_type)
        } else {
            message
        }
    }

    pub fn lane(&self) -> &TransmissionLane {
        match self {
            InputMessage::Regular { lane, .. }
            | InputMessage::Anonymous { lane, .. }
            | InputMessage::Reply { lane, .. }
            | InputMessage::Premade { lane, .. } => lane,
            InputMessage::MessageWrapper { message, .. } => message.lane(),
        }
    }

    pub fn serialized_size(&self) -> u64 {
        bincode::serialized_size(self).expect("failed to get serialized InputMessage size") + 4
    }
}

// TODO: Tests
pub struct InputMessageCodec;

impl Encoder<InputMessage> for InputMessageCodec {
    type Error = ClientCoreError;

    fn encode(&mut self, item: InputMessage, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let encoded = bincode::serialize(&item).expect("failed to serialize InputMessage");
        let encoded_len = encoded.len() as u32;
        let mut encoded_with_len = encoded_len.to_le_bytes().to_vec();
        encoded_with_len.extend(encoded);
        buf.reserve(encoded_with_len.len());
        buf.extend_from_slice(&encoded_with_len);
        Ok(())
    }
}

impl Decoder for InputMessageCodec {
    type Item = InputMessage;
    type Error = ClientCoreError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.len() < 4 {
            return Ok(None);
        }

        let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if buf.len() < len + 4 {
            return Ok(None);
        }

        let decoded = match bincode::deserialize(&buf[4..len + 4]) {
            Ok(decoded) => decoded,
            Err(_) => return Ok(None),
        };

        buf.advance(len + 4);

        Ok(Some(decoded))
    }
}
