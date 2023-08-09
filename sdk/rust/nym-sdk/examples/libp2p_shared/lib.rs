pub(crate) mod connection;
pub mod error;
pub(crate) mod message;
pub(crate) mod mixnet;
pub(crate) mod queue;
pub mod substream;
pub mod transport;

/// The deafult timeout secs for [`transport::Upgrade`] future.
const DEFAULT_HANDSHAKE_TIMEOUT_SECS: u64 = 5;
