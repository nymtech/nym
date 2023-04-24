use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;

pub type InputMessageSender = tokio::sync::mpsc::Sender<InputMessage>;
pub type InputMessageReceiver = tokio::sync::mpsc::Receiver<InputMessage>;

#[derive(Debug)]
pub enum InputMessage {
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

    MessageWrapper {
        message: Box<InputMessage>,
        packet_type: PacketType,
    },
}

impl InputMessage {
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
            | InputMessage::Reply { lane, .. } => lane,
            InputMessage::MessageWrapper { message, .. } => message.lane(),
        }
    }
}
