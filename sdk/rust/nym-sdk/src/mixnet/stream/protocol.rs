//! Wire protocol for stream multiplexing.
//!
//! Every message between streams carries a fixed header prepended to
//! the payload inside the mixnet message body:
//!
//! ```text
//! [Version: 1 byte][StreamId: 8 bytes][MessageType: 1 byte][payload: N bytes]
//! ```
//!
//! This header sits inside the sphinx packet payload.

use std::fmt;

/// Current stream protocol version.
pub const STREAM_PROTOCOL_VERSION: u8 = 1;

/// Length of a StreamId in bytes (u64, big-endian).
pub const STREAM_ID_LEN: usize = 8;

/// Total header length: Version (1) + StreamId (8) + MessageType (1).
pub const STREAM_HEADER_LEN: usize = 1 + STREAM_ID_LEN + 1;

/// Identifies a stream within a MixnetClient.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamId(u64);

impl StreamId {
    pub fn random() -> Self {
        Self(rand::random::<u64>())
    }

    pub fn to_bytes(self) -> [u8; STREAM_ID_LEN] {
        self.0.to_be_bytes()
    }

    pub fn from_bytes(bytes: [u8; STREAM_ID_LEN]) -> Self {
        Self(u64::from_be_bytes(bytes))
    }
}

impl fmt::Debug for StreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StreamId({:#018x})", self.0)
    }
}

impl fmt::Display for StreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Message types within the stream protocol.
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

/// The fixed-size header prepended to every stream message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixStreamHeader {
    pub version: u8,
    pub stream_id: StreamId,
    pub message_type: StreamMessageType,
}

/// A decoded stream frame: header + payload reference.
#[derive(Debug)]
pub struct MixStreamFrame<'a> {
    pub header: MixStreamHeader,
    pub data: &'a [u8],
}

/// Encode a stream message: `[version][stream_id][msg_type][payload]`.
pub fn encode_stream_message(
    id: &StreamId,
    msg_type: StreamMessageType,
    payload: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(STREAM_HEADER_LEN + payload.len());
    buf.push(STREAM_PROTOCOL_VERSION);
    buf.extend_from_slice(&id.to_bytes());
    buf.push(msg_type as u8);
    buf.extend_from_slice(payload);
    buf
}

/// Decode a stream message into a [`MixStreamFrame`].
///
/// Returns `None` if the buffer is too short, the version is unknown,
/// or the message type byte is invalid.
pub fn decode_stream_message(bytes: &[u8]) -> Option<MixStreamFrame<'_>> {
    if bytes.len() < STREAM_HEADER_LEN {
        return None;
    }

    let version = bytes[0];
    if version != STREAM_PROTOCOL_VERSION {
        return None;
    }

    let mut id_bytes = [0u8; STREAM_ID_LEN];
    id_bytes.copy_from_slice(&bytes[1..1 + STREAM_ID_LEN]);
    let stream_id = StreamId::from_bytes(id_bytes);

    let message_type = StreamMessageType::from_byte(bytes[1 + STREAM_ID_LEN])?;
    let data = &bytes[STREAM_HEADER_LEN..];

    Some(MixStreamFrame {
        header: MixStreamHeader {
            version,
            stream_id,
            message_type,
        },
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let id = StreamId::random();
        let payload = b"hello world";
        let encoded = encode_stream_message(&id, StreamMessageType::Data, payload);
        let frame = decode_stream_message(&encoded).unwrap();
        assert_eq!(frame.header.version, STREAM_PROTOCOL_VERSION);
        assert_eq!(frame.header.stream_id, id);
        assert_eq!(frame.header.message_type, StreamMessageType::Data);
        assert_eq!(frame.data, payload);
    }

    #[test]
    fn too_short() {
        assert!(decode_stream_message(&[0u8; 5]).is_none());
    }

    #[test]
    fn bad_version() {
        let id = StreamId::random();
        let mut encoded = encode_stream_message(&id, StreamMessageType::Data, b"x");
        encoded[0] = 0xFF;
        assert!(decode_stream_message(&encoded).is_none());
    }

    #[test]
    fn bad_message_type() {
        let mut buf = [0u8; STREAM_HEADER_LEN];
        buf[0] = STREAM_PROTOCOL_VERSION;
        buf[1 + STREAM_ID_LEN] = 0xFF;
        assert!(decode_stream_message(&buf).is_none());
    }

    #[test]
    fn empty_payload() {
        let id = StreamId::random();
        let encoded = encode_stream_message(&id, StreamMessageType::Open, &[]);
        let frame = decode_stream_message(&encoded).unwrap();
        assert_eq!(frame.header.message_type, StreamMessageType::Open);
        assert!(frame.data.is_empty());
    }

    #[test]
    fn header_wire_format() {
        let id = StreamId::from_bytes([0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]);
        let encoded = encode_stream_message(&id, StreamMessageType::Open, &[0xAA]);
        assert_eq!(encoded.len(), STREAM_HEADER_LEN + 1);
        assert_eq!(encoded[0], STREAM_PROTOCOL_VERSION);
        assert_eq!(
            &encoded[1..9],
            &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]
        );
        assert_eq!(encoded[9], StreamMessageType::Open as u8);
        assert_eq!(encoded[10], 0xAA);
    }
}
