//! Wire protocol for stream multiplexing.
//!
//! Every message between streams carries an LP frame header prepended to
//! the payload inside the mixnet message body:
//!
//! ```text
//! [LpFrameKind: 2 bytes LE][SphinxStreamFrameAttributes: 14 bytes][payload: N bytes]
//! ```
//!
//! The `SphinxStreamFrameAttributes` encode stream_id, message type, and sequence
//! number inside the LP header's `frame_attributes` field. This is the same
//! LP frame format used across the system (IPR detection, gateway dispatch).

use std::fmt;

use bytes::BytesMut;
use nym_lp::packet::frame::{
    LpFrame, LpFrameHeader, LpFrameKind, SphinxStreamFrameAttributes, SphinxStreamMsgType,
};

/// Identifies a stream within a MixnetClient.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamId(u64);

impl StreamId {
    pub fn random() -> Self {
        Self(rand::random::<u64>())
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }

    pub fn from_u64(v: u64) -> Self {
        Self(v)
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

/// A decoded stream frame: LP header fields + payload reference.
#[derive(Debug)]
pub struct StreamFrame<'a> {
    pub stream_id: StreamId,
    pub msg_type: SphinxStreamMsgType,
    pub sequence_num: u32,
    pub data: &'a [u8],
}

/// Encode a stream message as an LP frame: `[LpFrameHeader][payload]`.
pub fn encode_stream_message(
    id: &StreamId,
    msg_type: SphinxStreamMsgType,
    sequence_num: u32,
    payload: &[u8],
) -> Vec<u8> {
    let attrs = SphinxStreamFrameAttributes {
        stream_id: id.as_u64(),
        msg_type,
        sequence_num,
    };
    let frame = LpFrame::new_stream(attrs, payload.to_vec());
    let mut buf = BytesMut::with_capacity(LpFrameHeader::SIZE + payload.len());
    frame.encode(&mut buf);
    buf.to_vec()
}

/// Decode a stream message from LP frame bytes.
///
/// Returns `None` if the buffer is too short, the frame kind is not `Stream`,
/// or the stream attributes are invalid.
pub fn decode_stream_message(bytes: &[u8]) -> Option<StreamFrame<'_>> {
    if bytes.len() < LpFrameHeader::SIZE {
        return None;
    }

    let header = LpFrameHeader::parse(bytes).ok()?;
    if header.kind != LpFrameKind::SphinxStream {
        return None;
    }

    let attrs = SphinxStreamFrameAttributes::parse(&header.frame_attributes).ok()?;
    let data = &bytes[LpFrameHeader::SIZE..];

    Some(StreamFrame {
        stream_id: StreamId::from_u64(attrs.stream_id),
        msg_type: attrs.msg_type,
        sequence_num: attrs.sequence_num,
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
        let encoded = encode_stream_message(&id, SphinxStreamMsgType::Data, 42, payload);
        let frame = decode_stream_message(&encoded).unwrap();
        assert_eq!(frame.stream_id, id);
        assert_eq!(frame.msg_type, SphinxStreamMsgType::Data);
        assert_eq!(frame.sequence_num, 42);
        assert_eq!(frame.data, payload);
    }

    #[test]
    fn too_short() {
        assert!(decode_stream_message(&[0u8; 5]).is_none());
    }

    #[test]
    fn wrong_frame_kind() {
        // Opaque frame kind (0x00, 0x00) should not parse as stream
        let mut buf = vec![0u8; LpFrameHeader::SIZE + 1];
        buf[LpFrameHeader::SIZE] = 0xAA;
        assert!(decode_stream_message(&buf).is_none());
    }

    #[test]
    fn bad_msg_type() {
        let id = StreamId::random();
        let mut encoded = encode_stream_message(&id, SphinxStreamMsgType::Data, 0, b"x");
        // msg_type is at byte offset 2 + 8 = 10 (inside frame_attributes)
        encoded[10] = 0xFF;
        assert!(decode_stream_message(&encoded).is_none());
    }

    #[test]
    fn empty_payload() {
        let id = StreamId::random();
        let encoded = encode_stream_message(&id, SphinxStreamMsgType::Open, 0, &[]);
        let frame = decode_stream_message(&encoded).unwrap();
        assert_eq!(frame.msg_type, SphinxStreamMsgType::Open);
        assert_eq!(frame.sequence_num, 0);
        assert!(frame.data.is_empty());
    }

    #[test]
    fn header_wire_format() {
        let id = StreamId::from_u64(0x0011223344556677);
        let encoded = encode_stream_message(&id, SphinxStreamMsgType::Open, 1, &[0xAA]);

        // LpFrameHeader::SIZE (16) + 1 byte payload
        assert_eq!(encoded.len(), LpFrameHeader::SIZE + 1);

        // First 2 bytes: LpFrameKind::SphinxStream = 3, LE
        assert_eq!(encoded[0], 0x03);
        assert_eq!(encoded[1], 0x00);

        // Bytes 2..10: stream_id BE
        assert_eq!(
            &encoded[2..10],
            &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]
        );

        // Byte 10: msg_type = Open = 0
        assert_eq!(encoded[10], SphinxStreamMsgType::Open as u8);

        // Bytes 11..15: sequence_num = 1, BE
        assert_eq!(&encoded[11..15], &[0x00, 0x00, 0x00, 0x01]);

        // Byte 15: reserved = 0
        assert_eq!(encoded[15], 0x00);

        // Byte 16: payload
        assert_eq!(encoded[16], 0xAA);
    }

    #[test]
    fn sequence_num_roundtrip() {
        let id = StreamId::random();
        for seq in [0, 1, 255, 65535, u32::MAX] {
            let encoded = encode_stream_message(&id, SphinxStreamMsgType::Data, seq, b"test");
            let frame = decode_stream_message(&encoded).unwrap();
            assert_eq!(frame.sequence_num, seq);
        }
    }
}
