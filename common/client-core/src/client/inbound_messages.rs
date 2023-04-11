use crate::client::real_messages_control::real_traffic_stream::RealMessage;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_task::connections::TransmissionLane;

pub type InputMessageSender = tokio::sync::mpsc::Sender<InputMessage>;
pub type InputMessageReceiver = tokio::sync::mpsc::Receiver<InputMessage>;

#[derive(Debug)]
pub enum InputMessage {
    /// Fire an already prepared mix packet into the network.
    /// No guarantees are made about it. For example no retransmssion
    /// will be attempted if it gets dropped.
    Premade {
        msg: MixPacket,
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
}

impl InputMessage {
    pub fn new_premade(msg: MixPacket, lane: TransmissionLane) -> Self {
        InputMessage::Premade { msg, lane }
    }

    pub fn new_regular(recipient: Recipient, data: Vec<u8>, lane: TransmissionLane) -> Self {
        InputMessage::Regular {
            recipient,
            data,
            lane,
        }
    }

    pub fn new_anonymous(
        recipient: Recipient,
        data: Vec<u8>,
        reply_surbs: u32,
        lane: TransmissionLane,
    ) -> Self {
        InputMessage::Anonymous {
            recipient,
            data,
            reply_surbs,
            lane,
        }
    }

    pub fn new_reply(
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
    ) -> Self {
        InputMessage::Reply {
            recipient_tag,
            data,
            lane,
        }
    }

    pub fn lane(&self) -> &TransmissionLane {
        match self {
            InputMessage::Regular { lane, .. }
            | InputMessage::Anonymous { lane, .. }
            | InputMessage::Reply { lane, .. }
            | InputMessage::Premade { lane, .. } => lane,
        }
    }
}
