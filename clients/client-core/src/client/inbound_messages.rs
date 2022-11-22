use futures::channel::mpsc;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;

pub type InputMessageSender = mpsc::UnboundedSender<InputMessage>;
pub type InputMessageReceiver = mpsc::UnboundedReceiver<InputMessage>;

#[derive(Debug)]
pub enum InputMessage {
    /// The simplest message variant where no additional information is attached.
    /// You're simply sending your `data` to specified `recipient` without any tagging.
    ///
    /// Ends up with `NymMessage::Plain` variant
    Regular { recipient: Recipient, data: Vec<u8> },

    /// Create a message used for a duplex anonymous communication where the recipient
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
    },

    /// Attempt to use our internally received and stored `ReplySurb` to send the message back
    /// to specified recipient whilst not knowing its full identity (or even gateway).
    ///
    /// Ends up with `NymMessage::Reply` variant
    Reply {
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
    },
}

impl InputMessage {
    pub fn new_regular(recipient: Recipient, data: Vec<u8>) -> Self {
        InputMessage::Regular { recipient, data }
    }

    pub fn new_anonymous(recipient: Recipient, data: Vec<u8>, reply_surbs: u32) -> Self {
        InputMessage::Anonymous {
            recipient,
            data,
            reply_surbs,
        }
    }

    pub fn new_reply(recipient_tag: AnonymousSenderTag, data: Vec<u8>) -> Self {
        InputMessage::Reply {
            recipient_tag,
            data,
        }
    }
}
