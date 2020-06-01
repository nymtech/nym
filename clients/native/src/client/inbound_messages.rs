use futures::channel::mpsc;
use nymsphinx::addressing::clients::Recipient;

pub(crate) type InputMessageSender = mpsc::UnboundedSender<InputMessage>;
pub(crate) type InputMessageReceiver = mpsc::UnboundedReceiver<InputMessage>;

#[derive(Debug)]
pub(crate) struct InputMessage {
    recipient: Recipient,
    data: Vec<u8>,
}

impl InputMessage {
    pub(crate) fn new(recipient: Recipient, data: Vec<u8>) -> Self {
        InputMessage { recipient, data }
    }

    // I'm open to suggestions on how to rename this.
    pub(crate) fn destruct(self) -> (Recipient, Vec<u8>) {
        (self.recipient, self.data)
    }
}
