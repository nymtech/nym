use libp2p::core::PeerId;
use nym_sphinx::addressing::clients::Recipient;
use rand::rngs::OsRng;
use rand::RngCore;
use std::fmt::{Debug, Formatter};

use super::error::Error;

const RECIPIENT_LENGTH: usize = Recipient::LEN;
const CONNECTION_ID_LENGTH: usize = 32;
const SUBSTREAM_ID_LENGTH: usize = 32;

const NONCE_BYTES_LEN: usize = 8; // length of u64
const MIN_CONNECTION_MESSAGE_LEN: usize = CONNECTION_ID_LENGTH + NONCE_BYTES_LEN;

/// ConnectionId is a unique, randomly-generated per-connection ID that's used to
/// identify which connection a message belongs to.
#[derive(Clone, Default, Eq, Hash, PartialEq)]
pub(crate) struct ConnectionId([u8; 32]);

impl ConnectionId {
    pub(crate) fn generate() -> Self {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        ConnectionId(bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let mut id = [0u8; 32];
        id[..].copy_from_slice(&bytes[0..CONNECTION_ID_LENGTH]);
        ConnectionId(id)
    }
}

impl Debug for ConnectionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// SubstreamId is a unique, randomly-generated per-substream ID that's used to
/// identify which substream a message belongs to.
#[derive(Clone, Default, Eq, Hash, PartialEq)]
pub struct SubstreamId(pub(crate) [u8; 32]);

impl SubstreamId {
    pub(crate) fn generate() -> Self {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        SubstreamId(bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let mut id = [0u8; 32];
        id[..].copy_from_slice(&bytes[0..SUBSTREAM_ID_LENGTH]);
        SubstreamId(id)
    }
}

impl Debug for SubstreamId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Message {
    ConnectionRequest(ConnectionMessage),
    ConnectionResponse(ConnectionMessage),
    TransportMessage(TransportMessage),
}

/// ConnectionMessage is exchanged to open a new connection.
#[derive(Debug)]
pub(crate) struct ConnectionMessage {
    pub(crate) peer_id: PeerId,
    pub(crate) id: ConnectionId,
    /// recipient is the sender's Nym address.
    /// only required if this is a ConnectionRequest.
    pub(crate) recipient: Option<Recipient>,
}

/// TransportMessage is sent over a connection after establishment.
#[derive(Debug, Clone)]
pub(crate) struct TransportMessage {
    /// increments by 1 for every TransportMessage sent over a connection.
    /// required for ordering, since Nym does not guarantee ordering.
    /// ConnectionMessages do not need nonces, as we know that they will
    /// be the first messages sent over a connection.
    /// the first TransportMessage sent over a connection will have nonce 1.
    pub(crate) nonce: u64,
    pub(crate) message: SubstreamMessage,
    pub(crate) id: ConnectionId,
}

impl Message {
    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        if bytes.len() < 2 {
            return Err(Error::InvalidMessageBytes);
        }

        Ok(match bytes[0] {
            0 => Message::ConnectionRequest(ConnectionMessage::try_from_bytes(&bytes[1..])?),
            1 => Message::ConnectionResponse(ConnectionMessage::try_from_bytes(&bytes[1..])?),
            2 => Message::TransportMessage(TransportMessage::try_from_bytes(&bytes[1..])?),
            _ => return Err(Error::InvalidMessageBytes),
        })
    }
}

impl ConnectionMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.id.0.to_vec();
        match self.recipient {
            Some(recipient) => {
                bytes.push(1u8);
                bytes.append(&mut recipient.to_bytes().to_vec());
            }
            None => bytes.push(0u8),
        }
        bytes.append(&mut self.peer_id.to_bytes());
        bytes
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < CONNECTION_ID_LENGTH + 1 {
            return Err(Error::ConnectionMessageBytesTooShort);
        }

        let id = ConnectionId::from_bytes(&bytes[0..CONNECTION_ID_LENGTH]);
        let recipient = match bytes[CONNECTION_ID_LENGTH] {
            0u8 => None,
            1u8 => {
                if bytes.len() < CONNECTION_ID_LENGTH + 1 + RECIPIENT_LENGTH {
                    return Err(Error::ConnectionMessageBytesNoRecipient);
                }

                let mut recipient_bytes = [0u8; RECIPIENT_LENGTH];
                recipient_bytes[..].copy_from_slice(
                    &bytes[CONNECTION_ID_LENGTH + 1..CONNECTION_ID_LENGTH + 1 + RECIPIENT_LENGTH],
                );
                Some(
                    Recipient::try_from_bytes(recipient_bytes)
                        .map_err(Error::InvalidRecipientBytes)?,
                )
            }
            _ => {
                return Err(Error::InvalidRecipientPrefixByte);
            }
        };
        let peer_id = match recipient {
            Some(_) => {
                if bytes.len() < CONNECTION_ID_LENGTH + RECIPIENT_LENGTH + 2 {
                    return Err(Error::ConnectionMessageBytesNoPeerId);
                }
                PeerId::from_bytes(&bytes[CONNECTION_ID_LENGTH + 1 + RECIPIENT_LENGTH..])
                    .map_err(|_| Error::InvalidPeerIdBytes)?
            }
            None => {
                if bytes.len() < CONNECTION_ID_LENGTH + 2 {
                    return Err(Error::ConnectionMessageBytesNoPeerId);
                }
                PeerId::from_bytes(&bytes[CONNECTION_ID_LENGTH + 1..])
                    .map_err(|_| Error::InvalidPeerIdBytes)?
            }
        };
        Ok(ConnectionMessage {
            peer_id,
            recipient,
            id,
        })
    }
}

impl TransportMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.nonce.to_be_bytes().to_vec();
        bytes.extend_from_slice(self.id.0.as_ref());
        bytes.extend_from_slice(&self.message.to_bytes());
        bytes
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < MIN_CONNECTION_MESSAGE_LEN + 1 {
            return Err(Error::TransportMessageBytesTooShort);
        }

        let nonce = u64::from_be_bytes(
            bytes[0..NONCE_BYTES_LEN]
                .to_vec()
                .try_into()
                .map_err(|_| Error::InvalidNonce)?,
        );
        let id = ConnectionId::from_bytes(&bytes[NONCE_BYTES_LEN..MIN_CONNECTION_MESSAGE_LEN]);
        let message = SubstreamMessage::try_from_bytes(&bytes[MIN_CONNECTION_MESSAGE_LEN..])?;
        Ok(TransportMessage { nonce, message, id })
    }
}

impl Ord for TransportMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.nonce.cmp(&other.nonce)
    }
}

impl std::cmp::PartialOrd for TransportMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Eq for TransportMessage {}

impl std::cmp::PartialEq for TransportMessage {
    fn eq(&self, other: &Self) -> bool {
        self.nonce == other.nonce
    }
}

#[derive(Debug, Clone)]
pub(crate) enum SubstreamMessageType {
    OpenRequest,
    OpenResponse,
    Close,
    Data(Vec<u8>),
}

impl SubstreamMessageType {
    fn to_u8(&self) -> u8 {
        match self {
            SubstreamMessageType::OpenRequest => 0,
            SubstreamMessageType::OpenResponse => 1,
            SubstreamMessageType::Close => 2,
            SubstreamMessageType::Data(_) => 3,
        }
    }
}

/// SubstreamMessage is a message sent over a substream.
#[derive(Debug, Clone)]
pub(crate) struct SubstreamMessage {
    pub(crate) substream_id: SubstreamId,
    pub(crate) message_type: SubstreamMessageType,
}

impl SubstreamMessage {
    pub(crate) fn new_with_data(substream_id: SubstreamId, message: Vec<u8>) -> Self {
        SubstreamMessage {
            substream_id,
            message_type: SubstreamMessageType::Data(message),
        }
    }

    pub(crate) fn new_close(substream_id: SubstreamId) -> Self {
        SubstreamMessage {
            substream_id,
            message_type: SubstreamMessageType::Close,
        }
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.substream_id.0.clone().to_vec();
        bytes.push(self.message_type.to_u8());
        if let SubstreamMessageType::Data(message) = &self.message_type {
            bytes.extend_from_slice(message);
        }
        bytes
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < SUBSTREAM_ID_LENGTH + 1 {
            return Err(Error::InvalidSubstreamMessageBytes);
        }

        let substream_id = SubstreamId::from_bytes(&bytes[0..SUBSTREAM_ID_LENGTH]);
        let message_type = match bytes[SUBSTREAM_ID_LENGTH] {
            0 => SubstreamMessageType::OpenRequest,
            1 => SubstreamMessageType::OpenResponse,
            2 => SubstreamMessageType::Close,
            3 => {
                if bytes.len() < SUBSTREAM_ID_LENGTH + 2 {
                    return Err(Error::InvalidSubstreamMessageBytes);
                }
                SubstreamMessageType::Data(bytes[SUBSTREAM_ID_LENGTH + 1..].to_vec())
            }
            _ => return Err(Error::InvalidSubstreamMessageType),
        };

        Ok(SubstreamMessage {
            substream_id,
            message_type,
        })
    }
}

impl Message {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        match self {
            Message::ConnectionRequest(msg) => {
                let mut bytes = 0_u8.to_be_bytes().to_vec();
                bytes.append(&mut msg.to_bytes());
                bytes
            }
            Message::ConnectionResponse(msg) => {
                let mut bytes = 1_u8.to_be_bytes().to_vec();
                bytes.append(&mut msg.to_bytes());
                bytes
            }
            Message::TransportMessage(msg) => {
                let mut bytes = 2_u8.to_be_bytes().to_vec();
                bytes.append(&mut msg.to_bytes());
                bytes
            }
        }
    }
}

/// InboundMessage represents an inbound mixnet message.
pub(crate) struct InboundMessage(pub(crate) Message);

/// OutboundMessage represents an outbound mixnet message.
#[derive(Debug)]
pub(crate) struct OutboundMessage {
    pub(crate) message: Message,
    pub(crate) recipient: Recipient,
}

pub(crate) fn parse_message_data(data: &[u8]) -> Result<InboundMessage, Error> {
    if data.len() < 2 {
        return Err(Error::InvalidMessageBytes);
    }
    let msg = Message::try_from_bytes(data.to_vec())?;
    Ok(InboundMessage(msg))
}
