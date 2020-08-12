use futures::channel::mpsc;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::ReplySURB;

pub(crate) type InputMessageSender = mpsc::UnboundedSender<InputMessage>;
pub(crate) type InputMessageReceiver = mpsc::UnboundedReceiver<InputMessage>;

#[derive(Debug)]
pub(crate) enum InputMessage {
    Fresh {
        recipient: Recipient,
        data: Vec<u8>,
        with_reply_surb: bool,
    },
    Reply {
        reply_surb: ReplySURB,
        data: Vec<u8>,
    },
}

impl InputMessage {
    pub(crate) fn new_fresh(recipient: Recipient, data: Vec<u8>, with_reply_surb: bool) -> Self {
        InputMessage::Fresh {
            recipient,
            data,
            with_reply_surb,
        }
    }

    pub(crate) fn new_reply(reply_surb: ReplySURB, data: Vec<u8>) -> Self {
        InputMessage::Reply { reply_surb, data }
    }
}
