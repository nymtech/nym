use client_connections::TransmissionLane;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::ReplySurb;

pub type InputMessageSender = tokio::sync::mpsc::Sender<InputMessage>;
pub type InputMessageReceiver = tokio::sync::mpsc::Receiver<InputMessage>;

#[derive(Debug)]
pub enum InputMessage {
    Fresh {
        recipient: Recipient,
        data: Vec<u8>,
        with_reply_surb: bool,
        lane: TransmissionLane,
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
        lane: TransmissionLane,
    ) -> Self {
        InputMessage::Fresh {
            recipient,
            data,
            with_reply_surb,
            lane,
        }
    }

    pub fn new_reply(reply_surb: ReplySurb, data: Vec<u8>) -> Self {
        InputMessage::Reply { reply_surb, data }
    }
}
