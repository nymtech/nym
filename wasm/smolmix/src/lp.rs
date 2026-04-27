// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! Lightweight LP frame encode/decode for SphinxStream multiplexing.
//!
//! Reimplements the wire format from `nym-lp` without depending on it (nym-lp
//! pulls in tokio + libcrux, neither of which compiles on wasm32).
//!
//! Wire format (16-byte header + payload):
//! ```text
//! [0..2  ) LpFrameKind::SphinxStream = 0x03, 0x00 (LE u16)
//! [2..10 ) stream_id  (BE u64)
//! [10    ) msg_type   (0 = Open, 1 = Data)
//! [11..15) seq_num    (BE u32)
//! [15    ) reserved   (0x00)
//! [16..  ) payload
//! ```
//!
//! Reference: `sdk/rust/nym-sdk/src/mixnet/stream/protocol.rs:155-183`

/// Total size of the LP frame header in bytes.
pub const HEADER_SIZE: usize = 16;

/// LpFrameKind::SphinxStream encoded as LE u16.
const FRAME_KIND_SPHINX_STREAM: [u8; 2] = [0x03, 0x00];

/// Message type within an LP SphinxStream frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    Open = 0,
    Data = 1,
}

impl MsgType {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Open),
            1 => Some(Self::Data),
            _ => None,
        }
    }
}

/// A decoded LP frame.
#[derive(Debug)]
pub struct LpFrame {
    pub stream_id: u64,
    pub msg_type: MsgType,
    pub seq: u32,
    pub payload: Vec<u8>,
}

/// Encode payload into an LP SphinxStream frame.
pub fn encode(stream_id: u64, msg_type: MsgType, seq: u32, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(HEADER_SIZE + payload.len());

    // [0..2) frame kind
    buf.extend_from_slice(&FRAME_KIND_SPHINX_STREAM);
    // [2..10) stream_id BE
    buf.extend_from_slice(&stream_id.to_be_bytes());
    // [10] msg_type
    buf.push(msg_type as u8);
    // [11..15) sequence number BE
    buf.extend_from_slice(&seq.to_be_bytes());
    // [15] reserved
    buf.push(0x00);
    // [16..) payload
    buf.extend_from_slice(payload);

    buf
}

/// Decode an LP SphinxStream frame from raw bytes.
///
/// Returns `None` if the buffer is too short, the frame kind doesn't match
/// `SphinxStream`, or the message type byte is invalid.
pub fn decode(bytes: &[u8]) -> Option<LpFrame> {
    if bytes.len() < HEADER_SIZE {
        return None;
    }

    // Check frame kind
    if bytes[0] != FRAME_KIND_SPHINX_STREAM[0] || bytes[1] != FRAME_KIND_SPHINX_STREAM[1] {
        return None;
    }

    let stream_id = u64::from_be_bytes(bytes[2..10].try_into().ok()?);
    let msg_type = MsgType::from_u8(bytes[10])?;
    let seq = u32::from_be_bytes(bytes[11..15].try_into().ok()?);

    // bytes[15] is reserved, ignore its value on decode
    let payload = bytes[HEADER_SIZE..].to_vec();

    Some(LpFrame {
        stream_id,
        msg_type,
        seq,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let payload = b"hello world";
        let encoded = encode(0xDEADBEEF, MsgType::Data, 42, payload);
        let frame = decode(&encoded).unwrap();
        assert_eq!(frame.stream_id, 0xDEADBEEF);
        assert_eq!(frame.msg_type, MsgType::Data);
        assert_eq!(frame.seq, 42);
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn too_short() {
        assert!(decode(&[0u8; 5]).is_none());
    }

    #[test]
    fn wrong_frame_kind() {
        let mut buf = vec![0u8; HEADER_SIZE + 1];
        buf[HEADER_SIZE] = 0xAA;
        assert!(decode(&buf).is_none());
    }

    #[test]
    fn bad_msg_type() {
        let mut encoded = encode(1, MsgType::Data, 0, b"x");
        encoded[10] = 0xFF;
        assert!(decode(&encoded).is_none());
    }

    #[test]
    fn empty_payload() {
        let encoded = encode(99, MsgType::Open, 0, &[]);
        let frame = decode(&encoded).unwrap();
        assert_eq!(frame.msg_type, MsgType::Open);
        assert_eq!(frame.seq, 0);
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn header_wire_format() {
        let encoded = encode(0x0011223344556677, MsgType::Open, 1, &[0xAA]);

        assert_eq!(encoded.len(), HEADER_SIZE + 1);

        // Frame kind: SphinxStream = 3, LE
        assert_eq!(encoded[0], 0x03);
        assert_eq!(encoded[1], 0x00);

        // stream_id BE
        assert_eq!(
            &encoded[2..10],
            &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]
        );

        // msg_type = Open = 0
        assert_eq!(encoded[10], 0x00);

        // sequence_num = 1, BE
        assert_eq!(&encoded[11..15], &[0x00, 0x00, 0x00, 0x01]);

        // reserved = 0
        assert_eq!(encoded[15], 0x00);

        // payload
        assert_eq!(encoded[16], 0xAA);
    }

    #[test]
    fn sequence_num_limits() {
        for seq in [0, 1, 255, 65535, u32::MAX] {
            let encoded = encode(1, MsgType::Data, seq, b"test");
            let frame = decode(&encoded).unwrap();
            assert_eq!(frame.seq, seq);
        }
    }
}
