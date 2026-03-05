//! Wire protocol for stream multiplexing.
//!
//! Every message between streams carries a fixed 17-byte header prepended
//! to the payload inside the mixnet message body:
//!
//! ```text
//! [StreamId: 16 bytes][MessageType: 1 byte][payload: N bytes]
//! ```
//!
//! This header sits inside the sphinx packet payload.

use rand::RngCore;
use std::fmt;

/// Length of a StreamId in bytes.
pub const STREAM_ID_LEN: usize = 16;

/// Total header length: StreamId (16) + MessageType (1).
pub const STREAM_HEADER_LEN: usize = STREAM_ID_LEN + 1;

/// Identifies a stream within a MixnetClient.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamId([u8; STREAM_ID_LEN]);

impl StreamId {
    pub fn random() -> Self {
        let mut bytes = [0u8; STREAM_ID_LEN];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; STREAM_ID_LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; STREAM_ID_LEN] {
        &self.0
    }
}

impl fmt::Debug for StreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StreamId({:02x}{:02x}..{:02x}{:02x})",
            self.0[0], self.0[1], self.0[14], self.0[15]
        )
    }
}

impl fmt::Display for StreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Message types within the stream protocol.
///
/// Encoded as a single byte after the StreamId in every message payload:
/// `[StreamId (16 bytes)][MessageType (1 byte)][payload ...]`
///
/// Note: there is no Close variant. Without message sequencing, a close
/// message races ahead of in-flight data and arrives before the data is
/// reconstructed. Streams clean up locally via Drop. If ordered close/EOF
/// is needed in future, add sequencing + reorder buffering (see the
/// tcp_proxy's `MessageBuffer` for a working example).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamMessageType {
    /// Open a new stream. Payload is optional initial data.
    Open = 0,
    /// Data on an existing stream.
    Data = 1,
}

impl StreamMessageType {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Open),
            1 => Some(Self::Data),
            _ => None,
        }
    }
}

/// Encode a stream message: `[StreamId][MessageType][payload]`.
pub fn encode_stream_message(
    id: &StreamId,
    msg_type: StreamMessageType,
    payload: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(STREAM_HEADER_LEN + payload.len());
    buf.extend_from_slice(id.as_bytes());
    buf.push(msg_type as u8);
    buf.extend_from_slice(payload);
    buf
}

/// Decode a stream message, returning `(StreamId, MessageType, payload)`.
///
/// Returns `None` if the buffer is too short or the message type byte is unknown.
pub fn decode_stream_message(bytes: &[u8]) -> Option<(StreamId, StreamMessageType, &[u8])> {
    if bytes.len() < STREAM_HEADER_LEN {
        return None;
    }
    let mut id_bytes = [0u8; STREAM_ID_LEN];
    id_bytes.copy_from_slice(&bytes[..STREAM_ID_LEN]);
    let msg_type = StreamMessageType::from_byte(bytes[STREAM_ID_LEN])?;
    let payload = &bytes[STREAM_HEADER_LEN..];
    Some((StreamId::from_bytes(id_bytes), msg_type, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let id = StreamId::random();
        let payload = b"hello world";
        let encoded = encode_stream_message(&id, StreamMessageType::Data, payload);
        let (dec_id, dec_type, dec_payload) = decode_stream_message(&encoded).unwrap();
        assert_eq!(dec_id, id);
        assert_eq!(dec_type, StreamMessageType::Data);
        assert_eq!(dec_payload, payload);
    }

    #[test]
    fn too_short() {
        assert!(decode_stream_message(&[0u8; 10]).is_none());
    }

    #[test]
    fn bad_message_type() {
        let mut buf = [0u8; STREAM_HEADER_LEN];
        buf[STREAM_ID_LEN] = 0xFF;
        assert!(decode_stream_message(&buf).is_none());
    }

    #[test]
    fn empty_payload() {
        let id = StreamId::random();
        let encoded = encode_stream_message(&id, StreamMessageType::Open, &[]);
        let (_, msg_type, payload) = decode_stream_message(&encoded).unwrap();
        assert_eq!(msg_type, StreamMessageType::Open);
        assert!(payload.is_empty());
    }
}
