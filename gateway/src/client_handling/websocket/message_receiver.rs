use futures::channel::mpsc;

pub(crate) type MixMessageSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub(crate) type MixMessageReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;
