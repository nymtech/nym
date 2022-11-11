use futures::channel::mpsc;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::anonymous_replies::ReplySurb;

pub type InputMessageSender = mpsc::UnboundedSender<InputMessage>;
pub type InputMessageReceiver = mpsc::UnboundedReceiver<InputMessage>;

#[derive(Debug)]
pub enum InputMessage {
    Regular {
        recipient: Recipient,
        data: Vec<u8>,
        reply_surbs: u32,
    },
    Reply {
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
    },
    ReplyWithSurb {
        recipient_tag: AnonymousSenderTag,
        reply_surb: ReplySurb,
        data: Vec<u8>,
    },
}

impl InputMessage {
    pub fn new_regular(recipient: Recipient, data: Vec<u8>) -> Self {
        InputMessage::Regular {
            recipient,
            data,
            reply_surbs: 0,
        }
    }

    pub fn new_anonymous(recipient: Recipient, data: Vec<u8>, reply_surbs: u32) -> Self {
        InputMessage::Regular {
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

    pub fn new_reply_with_surb(
        recipient_tag: AnonymousSenderTag,
        reply_surb: ReplySurb,
        data: Vec<u8>,
    ) -> Self {
        InputMessage::ReplyWithSurb {
            recipient_tag,
            reply_surb,
            data,
        }
    }
}
