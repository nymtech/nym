use futures::channel::mpsc;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::ReplySurb;

pub type InputMessageSender = mpsc::UnboundedSender<InputMessage>;
pub type InputMessageReceiver = mpsc::UnboundedReceiver<InputMessage>;

#[derive(Debug)]
pub enum InputMessage {
    Fresh {
        recipient: Recipient,
        data: Vec<u8>,
        with_reply_surb: bool,
        // WIP(JON): use ConnectionId instead
        connection_id: u64,
    },
    Reply {
        reply_surb: ReplySurb,
        data: Vec<u8>,
    },
}

impl InputMessage {
    pub fn new_fresh(
        recipient: Recipient,
        data: Vec<u8>,
        with_reply_surb: bool,
        connection_id: u64,
    ) -> Self {
        InputMessage::Fresh {
            recipient,
            data,
            with_reply_surb,
            connection_id,
        }
    }

    pub fn new_reply(reply_surb: ReplySurb, data: Vec<u8>) -> Self {
        InputMessage::Reply { reply_surb, data }
    }
}
