// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;

pub type InputMessageSender = tokio::sync::mpsc::Sender<InputMessage>;
pub type InputMessageReceiver = tokio::sync::mpsc::Receiver<InputMessage>;

#[derive(Debug)]
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
        max_retransmissions: Option<u32>,
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
        max_retransmissions: Option<u32>,
    },

    /// Attempt to use our internally received and stored `ReplySurb` to send the message back
    /// to specified recipient whilst not knowing its full identity (or even gateway).
    ///
    /// Ends up with `NymMessage::Reply` variant
    Reply {
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
        max_retransmissions: Option<u32>,
    },

    MessageWrapper {
        message: Box<InputMessage>,
        packet_type: PacketType,
    },
}

impl InputMessage {
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
            max_retransmissions: None,
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
            max_retransmissions: None,
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
            // \/ set it to SOME sane default so that if we run out of surbs and constantly
            // fail to request more, we wouldn't be stuck in limbo
            max_retransmissions: Some(10),
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

    pub fn set_max_retransmissions(&mut self, max_retransmissions: u32) -> &mut Self {
        match self {
            InputMessage::Regular {
                max_retransmissions: m,
                ..
            }
            | InputMessage::Anonymous {
                max_retransmissions: m,
                ..
            }
            | InputMessage::Reply {
                max_retransmissions: m,
                ..
            } => {
                *m = Some(max_retransmissions);
            }
            InputMessage::Premade { .. } => {}
            InputMessage::MessageWrapper { message, .. } => {
                message.set_max_retransmissions(max_retransmissions);
            }
        }

        self
    }

    pub fn with_max_retransmissions(mut self, max_retransmissions: u32) -> Self {
        self.set_max_retransmissions(max_retransmissions);
        self
    }
}
