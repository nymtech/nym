use libp2p::core::multiaddr;
use nym_sphinx::addressing::clients::RecipientFormattingError;

use crate::message::SubstreamId;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unimplemented")]
    Unimplemented,
    #[error("failed to format multiaddress from nym address")]
    FailedToFormatMultiaddr(#[from] multiaddr::Error),
    #[error("unexpected protocol in multiaddress")]
    InvalidProtocolForMultiaddr,
    #[error("nym message error")]
    NymMessageError(String),
    #[error("unexpected message received over mixnet")]
    UnexpectedNymMessage,
    #[error("unexpected response to get self address request")]
    UnexpectedSelfAddressResponse,
    #[error("unknown nym message")]
    UnknownNymMessage,
    #[error("failed to decode message")]
    InvalidMessageBytes,
    #[error("no connection found for ConnectionResponse")]
    NoConnectionForResponse,
    #[error("received ConnectionResponse but connection was already established")]
    ConnectionAlreadyEstablished,
    #[error("received None recipient in ConnectionRequest")]
    NoneRecipientInConnectionRequest,
    #[error("cannot handle connection request; already have connection with given ID")]
    ConnectionIDExists,
    #[error("no connection found for TransportMessage")]
    NoConnectionForTransportMessage,
    #[error("failed to decode ConnectionMessage; too short")]
    ConnectionMessageBytesTooShort,
    #[error("failed to decode ConnectionMessage; no recipient")]
    ConnectionMessageBytesNoRecipient,
    #[error("failed to decode ConnectionMessage; no peer ID")]
    ConnectionMessageBytesNoPeerId,
    #[error("invalid peer ID bytes")]
    InvalidPeerIdBytes(#[from] multihash::Error),
    #[error("invalid recipient bytes")]
    InvalidRecipientBytes(#[from] RecipientFormattingError),
    #[error("invalid recipient prefix byte")]
    InvalidRecipientPrefixByte,
    #[error("failed to decode TransportMessage; too short")]
    TransportMessageBytesTooShort,
    #[error("failed to decode TransportMessage; invalid nonce")]
    InvalidNonce,
    #[error("invalid substream ID")]
    InvalidSubstreamMessageBytes,
    #[error("invalid substream message type byte")]
    InvalidSubstreamMessageType,
    #[error("substrean with given ID already exists")]
    SubstreamIdExists(SubstreamId),
    #[error("no substream found for given ID")]
    SubstreamIdDoesNotExist(SubstreamId),
    #[error("recv error: channel closed")]
    OneshotRecvError(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("failed to send new substream; receiver dropped")]
    SubstreamSendError,
    #[error("recv error: channel closed")]
    RecvError,
    #[error("outbound send error")]
    OutboundSendError(String),
    #[error("inbound send error")]
    InboundSendError(String),
    #[error("failed to send new connection; receiver dropped")]
    ConnectionSendError,
    #[error("failed to send initial TransportEvent::NewAddress")]
    SendErrorTransportEvent,
    #[error("dial timed out")]
    DialTimeout(#[from] tokio::time::error::Elapsed),
}
