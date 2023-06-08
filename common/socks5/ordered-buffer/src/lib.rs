mod buffer;
#[deprecated]
mod message;
#[deprecated]
mod sender;

pub use buffer::{OrderedMessageBuffer, ReadContiguousData};
pub use message::MessageError;
pub use message::OrderedMessage;
pub use sender::OrderedMessageSender;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Socks5DataStream {
    inner: Arc<Socks5DataStreamInner>,
}

#[derive(Debug)]
pub struct Socks5DataStreamInner {
    next_seq: AtomicU64,
}
