// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{LpError, LpMessage};
use bytes::BytesMut;
use libcrux_psq::Channel;
use nym_lp_packet::{EncryptedLpPacket, InnerHeader, LpHeader, LpPacket, OuterHeader};
use nym_lp_transport::traits::LpTransportChannel;
use tracing::error;

pub(crate) const CIPHERTEXT_OVERHEAD: usize = 25;

/// Parse only the outer header from raw packet bytes.
///
/// Used for routing before session lookup. The outer header (receiver_idx + counter)
/// is always cleartext at bytes 0-12 in the unified packet format.
///
/// # Arguments
/// * `src` - Raw packet bytes (at least OuterHeader::SIZE bytes)
///
/// # Errors
/// * `LpError::InsufficientBufferSize` - Packet too small for outer header
pub fn parse_lp_header_only(src: &[u8]) -> Result<OuterHeader, LpError> {
    Ok(OuterHeader::parse(src)?)
}

// /// Parses a complete Lewes Protocol packet from a byte slice (e.g., a UDP datagram payload).
// ///
// /// # Arguments
// /// * `outer_header` - The parsed OuterHeader from the underlying stream
// /// * `plaintext` - the decrypted plaintext of the remainer of the packet
// ///
// /// # Errors
// /// * `LpError::InsufficientBufferSize` - Packet too small
// pub fn parse_lp_packet(outer_header: OuterHeader, plaintext: &[u8]) -> Result<LpPacket, LpError> {
//     todo!()
//     // if plaintext.len() < InnerHeader::SIZE {
//     //     return Err(LpError::InsufficientBufferSize);
//     // }
//     //
//     // let inner_header = InnerHeader::parse(plaintext)?;
//     // let payload = &plaintext[InnerHeader::SIZE..];
//     // let message = LpMessage::decode_content(payload, inner_header.message_type)?;
//     //
//     // Ok(LpPacket {
//     //     header: LpHeader {
//     //         outer: outer_header,
//     //         inner: inner_header,
//     //     },
//     //     message,
//     // })
// }

pub(crate) fn encrypt_data(
    plaintext: &[u8],
    transport: &mut libcrux_psq::session::Transport,
) -> Result<Vec<u8>, LpError> {
    let mut ciphertext = vec![0u8; plaintext.len() + CIPHERTEXT_OVERHEAD + 64];
    let n = transport.write_message(&*plaintext, &mut ciphertext)?;

    if plaintext.len() + CIPHERTEXT_OVERHEAD != n {
        // TODO: check consistency
        error!("FIXME: inconsistent ciphertext overhead")
    }
    ciphertext.truncate(n);

    Ok(ciphertext)
}

pub(crate) fn decrypt_data(
    ciphertext: &[u8],
    transport: &mut libcrux_psq::session::Transport,
) -> Result<Vec<u8>, LpError> {
    if ciphertext.len() < CIPHERTEXT_OVERHEAD {
        return Err(LpError::InsufficientBufferSize);
    }
    let mut plaintext = vec![0u8; ciphertext.len() - CIPHERTEXT_OVERHEAD];

    let (_, n) = transport.read_message(&ciphertext, &mut plaintext)?;
    if n != ciphertext.len() - CIPHERTEXT_OVERHEAD {
        // TODO: check consistency
        error!("FIXME: inconsistent ciphertext overhead")
    }
    plaintext.truncate(n);
    Ok(plaintext)
}

pub fn encrypt_lp_packet(
    packet: LpPacket,
    transport: &mut libcrux_psq::session::Transport,
) -> Result<EncryptedLpPacket, LpError> {
    let mut plaintext = BytesMut::with_capacity(InnerHeader::SIZE + packet.message().len());
    packet.header().inner.encode(&mut plaintext);
    packet.message().encode_content(&mut plaintext);

    let ciphertext = encrypt_data(plaintext.as_ref(), transport)?;

    Ok(EncryptedLpPacket::new(packet.header().outer, ciphertext))
}

pub fn decrypt_lp_packet(
    packet: EncryptedLpPacket,
    transport: &mut libcrux_psq::session::Transport,
) -> Result<LpPacket, LpError> {
    if packet.ciphertext().len() < InnerHeader::SIZE + CIPHERTEXT_OVERHEAD {
        return Err(LpError::InsufficientBufferSize);
    }

    let plaintext = decrypt_data(packet.ciphertext(), transport)?;

    let inner_header = InnerHeader::parse(&plaintext)?;
    let payload = &plaintext[InnerHeader::SIZE..];
    let message = LpMessage::decode_content(payload, inner_header.message_type)?;

    Ok(LpPacket::new(
        LpHeader {
            outer: packet.outer_header(),
            inner: inner_header,
        },
        message,
    ))
}

/// Serializes an LpPacket into the provided BytesMut buffer.
///
/// ## Unified Packet Format
///
/// Both cleartext and encrypted packets have the same structure:
/// - Outer header (12B): receiver_idx(4) + counter(8) - always cleartext
/// - Inner payload: proto(1) + reserved(3) + msg_type(4) + content - encrypted
///
/// # Arguments
/// * `item` - Packet to serialize
/// * `dst` - Output buffer
/// * `transport` - AEAD encryption channel
pub fn serialize_lp_packet(
    item: LpPacket,
    dst: &mut BytesMut,
    transport: &mut libcrux_psq::session::Transport,
) -> Result<(), LpError> {
    // 1. encrypt the inner header and payload
    let encrypted_packet = encrypt_lp_packet(item, transport)?;

    // 2. Write outer header (always cleartext) followed by the ciphertext
    encrypted_packet.encode(dst);

    Ok(())
}

#[cfg(test)]
mod tests {

    // === Cleartext Encode/Decode Tests ===

    #[test]
    fn restore_below_tests() {
        todo!()
    }
    //
    // #[test]
    // fn test_serialize_parse_busy() {
    //     let mut dst = BytesMut::new();
    //
    //     // Create a Busy packet
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 42,
    //             counter: 123,
    //         },
    //         message: LpMessage::Busy,
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     // Serialize the packet (cleartext)
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //
    //     // Parse the packet (cleartext)
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     // Verify the packet fields
    //     assert_eq!(decoded.header.protocol_version, 1);
    //     assert_eq!(decoded.header.receiver_idx, 42);
    //     assert_eq!(decoded.header.counter, 123);
    //     assert!(matches!(decoded.message, LpMessage::Busy));
    //     assert_eq!(decoded.trailer, [0; TRAILER_LEN]);
    // }
    //
    // #[test]
    // fn test_serialize_parse_handshake() {
    //     let mut dst = BytesMut::new();
    //
    //     // Create a Handshake message packet
    //     let payload = vec![42u8; 80]; // Example payload size
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 42,
    //             counter: 123,
    //         },
    //         message: LpMessage::PSQRequest(PSQRequestData(payload.clone())),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     // Serialize the packet (cleartext)
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //
    //     // Parse the packet (cleartext)
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     // Verify the packet fields
    //     assert_eq!(decoded.header.protocol_version, 1);
    //     assert_eq!(decoded.header.receiver_idx, 42);
    //     assert_eq!(decoded.header.counter, 123);
    //
    //     // Verify message type and data
    //     match decoded.message {
    //         LpMessage::PSQRequest(decoded_payload) => {
    //             assert_eq!(decoded_payload, PSQRequestData(payload));
    //         }
    //         _ => panic!("Expected Handshake message"),
    //     }
    //     assert_eq!(decoded.trailer, [0; TRAILER_LEN]);
    // }
    //
    // #[test]
    // fn test_serialize_parse_encrypted_data() {
    //     let mut dst = BytesMut::new();
    //
    //     // Create an EncryptedData message packet
    //     let payload = vec![43u8; 124]; // Example payload size
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 42,
    //             counter: 123,
    //         },
    //         message: LpMessage::EncryptedData(EncryptedDataPayload(payload.clone())),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     // Serialize the packet (cleartext)
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //
    //     // Parse the packet (cleartext)
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     // Verify the packet fields
    //     assert_eq!(decoded.header.protocol_version, 1);
    //     assert_eq!(decoded.header.receiver_idx, 42);
    //     assert_eq!(decoded.header.counter, 123);
    //
    //     // Verify message type and data
    //     match decoded.message {
    //         LpMessage::EncryptedData(decoded_payload) => {
    //             assert_eq!(decoded_payload, EncryptedDataPayload(payload));
    //         }
    //         _ => panic!("Expected EncryptedData message"),
    //     }
    //     assert_eq!(decoded.trailer, [0; TRAILER_LEN]);
    // }
    //
    // // === Incomplete Data Tests ===
    //
    // #[test]
    // fn test_parse_incomplete_header() {
    //     // Create a buffer with incomplete header
    //     let mut buf = BytesMut::new();
    //     buf.extend_from_slice(&[1, 0, 0, 0]); // Only 4 bytes, not enough for LpHeader::SIZE
    //
    //     // Attempt to parse - expect error
    //     let result = parse_lp_packet(&buf, None);
    //     assert!(result.is_err());
    //     assert!(matches!(
    //         result.unwrap_err(),
    //         LpError::InsufficientBufferSize
    //     ));
    // }
    //
    // #[test]
    // fn test_parse_incomplete_message_type() {
    //     // Create a buffer with complete header but incomplete message type
    //     let mut buf = BytesMut::new();
    //     buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
    //     buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
    //     buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
    //     buf.extend_from_slice(&[0]); // Only 1 byte of message type (need 2)
    //
    //     // Buffer length = 16 + 1 = 17. Min size = 16 + 2 + 16 = 34.
    //     let result = parse_lp_packet(&buf, None);
    //     assert!(result.is_err());
    //     assert!(matches!(
    //         result.unwrap_err(),
    //         LpError::InsufficientBufferSize
    //     ));
    // }
    //
    // #[test]
    // fn test_parse_incomplete_message_data() {
    //     // Create a buffer simulating Handshake but missing trailer and maybe partial payload
    //     let mut buf = BytesMut::new();
    //     buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
    //     buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
    //     buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
    //     buf.extend_from_slice(&MessageType::Handshake.to_u32().to_le_bytes()); // Handshake type
    //     buf.extend_from_slice(&[42; 40]); // 40 bytes of payload data
    //
    //     // Buffer length = 16 + 2 + 40 = 58. Min size = 16 + 2 + 16 = 34.
    //     // Payload size calculated as 58 - 34 = 24.
    //     // Trailer expected at index 16 + 2 + 24 = 42.
    //     // Trailer read attempts src[42..58].
    //     // This *should* parse successfully based on the logic, but the trailer is garbage.
    //     // Let's rethink: parse_lp_packet assumes the *entire slice* is the packet.
    //     // If the slice doesn't end exactly where the trailer should, it's an error.
    //     // In this case, total length is 58. OuterHdr(12) + InnerPrefix(4) + Type(2) + Trailer(16) = 34. Payload = 58-34=24.
    //     // Trailer starts at 16+2+24 = 42. Ends at 42+16=58. It fits exactly.
    //     // This test *still* doesn't test incompleteness correctly for the datagram parser.
    //
    //     // Let's test a buffer that's *too short* even for header+type+trailer+min_payload
    //     // Note: Buffer order doesn't matter for this test since we fail on minimum size check
    //     let mut buf_too_short = BytesMut::new();
    //     buf_too_short.extend_from_slice(&42u32.to_le_bytes()); // receiver_idx (outer header)
    //     buf_too_short.extend_from_slice(&123u64.to_le_bytes()); // counter (outer header)
    //     buf_too_short.extend_from_slice(&[1, 0, 0, 0]); // version + reserved (inner prefix)
    //     buf_too_short.extend_from_slice(&MessageType::Handshake.to_u32().to_le_bytes()); // msg type
    //     // No payload, no trailer. Length = 12+4+2=18. Min size = 34.
    //     let result_too_short = parse_lp_packet(&buf_too_short, None);
    //     assert!(result_too_short.is_err());
    //     assert!(matches!(
    //         result_too_short.unwrap_err(),
    //         LpError::InsufficientBufferSize
    //     ));
    //
    //     // Test a buffer missing PART of the trailer
    //     let mut buf_partial_trailer = BytesMut::new();
    //     buf_partial_trailer.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
    //     buf_partial_trailer.extend_from_slice(&42u32.to_le_bytes()); // Sender index
    //     buf_partial_trailer.extend_from_slice(&123u64.to_le_bytes()); // Counter
    //     buf_partial_trailer.extend_from_slice(&MessageType::Handshake.to_u32().to_le_bytes()); // Handshake type
    //     let payload = vec![42u8; 20]; // Assume 20 byte payload
    //     buf_partial_trailer.extend_from_slice(&payload);
    //     buf_partial_trailer.extend_from_slice(&[0; TRAILER_LEN - 1]); // Missing last byte of trailer
    //
    //     // Total length = 16 + 2 + 20 + 15 = 53. Min size = 34. This passes.
    //     // Payload size = 53 - 34 = 19. <--- THIS IS WRONG. The parser assumes the length dictates payload.
    //     // Let's fix the parser logic slightly.
    //
    //     // The point is, parse_lp_packet expects a COMPLETE datagram. Providing less bytes
    //     // than LpHeader + Type + Trailer should fail. Providing *more* is also an issue unless
    //     // the length calculation works out perfectly. The most direct test is just < min_size.
    //     // Renaming test to reflect this.
    // }
    //
    // #[test]
    // fn test_parse_buffer_smaller_than_minimum() {
    //     // Test a buffer that's smaller than the smallest possible packet (LpHeader+Type+Trailer)
    //     let mut buf_too_short = BytesMut::new();
    //     buf_too_short.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
    //     buf_too_short.extend_from_slice(&42u32.to_le_bytes()); // Sender index
    //     buf_too_short.extend_from_slice(&123u64.to_le_bytes()); // Counter
    //     buf_too_short.extend_from_slice(&MessageType::Busy.to_u32().to_le_bytes()); // Type
    //     buf_too_short.extend_from_slice(&[0; TRAILER_LEN - 1]); // Missing last byte of trailer
    //     // Length = 16 + 2 + 15 = 33. Min Size = 34.
    //     let result_too_short = parse_lp_packet(&buf_too_short, None);
    //     assert!(
    //         result_too_short.is_err(),
    //         "Expected error for buffer size 33, min 34"
    //     );
    //     assert!(matches!(
    //         result_too_short.unwrap_err(),
    //         LpError::InsufficientBufferSize
    //     ));
    // }
    //
    // #[test]
    // fn test_parse_invalid_message_type() {
    //     // Create a buffer with invalid message type
    //     let mut buf = BytesMut::new();
    //     buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
    //     buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
    //     buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
    //     buf.extend_from_slice(&231u16.to_le_bytes()); // Invalid message type
    //     // Need payload and trailer to meet min_size requirement
    //     let payload_size = 10; // Arbitrary
    //     buf.extend_from_slice(&vec![0u8; payload_size]); // Some data
    //     buf.extend_from_slice(&[0; TRAILER_LEN]); // Trailer
    //
    //     // Attempt to parse
    //     let result = parse_lp_packet(&buf, None);
    //     assert!(result.is_err());
    //     match result {
    //         Err(LpError::InvalidMessageType(231)) => {} // Expected error
    //         Err(e) => panic!("Expected InvalidMessageType error, got {:?}", e),
    //         Ok(_) => panic!("Expected error, but got Ok"),
    //     }
    // }
    //
    // #[test]
    // fn test_parse_incorrect_payload_size_for_busy() {
    //     // Create a Busy packet but *with* a payload (which is invalid)
    //     let mut buf = BytesMut::new();
    //     buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
    //     buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
    //     buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
    //     buf.extend_from_slice(&MessageType::Busy.to_u32().to_le_bytes()); // Busy type
    //     buf.extend_from_slice(&[42; 1]); // <<< Invalid 1-byte payload for Busy
    //     buf.extend_from_slice(&[0; TRAILER_LEN]); // Trailer
    //
    //     // Total size = 16 + 2 + 1 + 16 = 35. Min size = 34.
    //     // Calculated payload size = 35 - 34 = 1.
    //     let result = parse_lp_packet(&buf, None);
    //     assert!(result.is_err());
    //     assert!(matches!(
    //         result.unwrap_err(),
    //         LpError::InvalidPayloadSize {
    //             expected: 0,
    //             actual: 1
    //         }
    //     ));
    // }
    //
    // // Test multiple packets simulation isn't relevant for datagram parsing
    // // #[test]
    // // fn test_multiple_packets_in_buffer() { ... }
    //
    //
    //
    // #[test]
    // fn test_forward_packet_encode_decode_roundtrip_v4() {
    //     let mut dst = BytesMut::new();
    //
    //     let forward_data = crate::message::ForwardPacketData {
    //         target_gateway_identity: [77u8; 32],
    //         target_lp_address: "1.2.3.4:41264".parse().unwrap(),
    //         inner_packet_bytes: vec![0xa, 0xb, 0xc, 0xd],
    //     };
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 999,
    //             counter: 555,
    //         },
    //         message: LpMessage::ForwardPacket(forward_data),
    //         trailer: [0xff; TRAILER_LEN],
    //     };
    //
    //     // Serialize
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //
    //     // Parse back
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     // Verify LP protocol handling works correctly
    //     assert_eq!(decoded.header.receiver_idx, 999);
    //     assert!(matches!(decoded.message.typ(), MessageType::ForwardPacket));
    //
    //     if let LpMessage::ForwardPacket(data) = decoded.message {
    //         assert_eq!(data.target_gateway_identity, [77u8; 32]);
    //         assert_eq!(data.target_lp_address, "1.2.3.4:41264".parse().unwrap());
    //         assert_eq!(data.inner_packet_bytes, vec![0xa, 0xb, 0xc, 0xd]);
    //     } else {
    //         panic!("Expected ForwardPacket message");
    //     }
    // }
    //
    // #[test]
    // fn test_forward_packet_encode_decode_roundtrip_v6() {
    //     let mut dst = BytesMut::new();
    //
    //     let forward_data = crate::message::ForwardPacketData {
    //         target_gateway_identity: [77u8; 32],
    //         target_lp_address: "[dead:beef:4242:c0ff:ee00::1111]:41264".parse().unwrap(),
    //         inner_packet_bytes: vec![0xa, 0xb, 0xc, 0xd],
    //     };
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 999,
    //             counter: 555,
    //         },
    //         message: LpMessage::ForwardPacket(forward_data),
    //         trailer: [0xff; TRAILER_LEN],
    //     };
    //
    //     // Serialize
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //
    //     // Parse back
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     // Verify LP protocol handling works correctly
    //     assert_eq!(decoded.header.receiver_idx, 999);
    //     assert!(matches!(decoded.message.typ(), MessageType::ForwardPacket));
    //
    //     if let LpMessage::ForwardPacket(data) = decoded.message {
    //         assert_eq!(data.target_gateway_identity, [77u8; 32]);
    //         assert_eq!(
    //             data.target_lp_address,
    //             "[dead:beef:4242:c0ff:ee00::1111]:41264".parse().unwrap()
    //         );
    //         assert_eq!(data.inner_packet_bytes, vec![0xa, 0xb, 0xc, 0xd]);
    //     } else {
    //         panic!("Expected ForwardPacket message");
    //     }
    // }
    //
    // // === Outer AEAD Tests ===
    //
    // #[test]
    // fn test_aead_roundtrip_with_key() {
    //     // Test that encrypt/decrypt roundtrip works with an AEAD key
    //     let psk = [42u8; 32];
    //     let outer_key = OuterAeadKey::from_psk(&psk);
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 12345,
    //             counter: 999,
    //         },
    //         message: LpMessage::Busy,
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let mut encrypted = BytesMut::new();
    //     serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();
    //
    //     // Parse back with the same key
    //     let decoded = parse_lp_packet(&encrypted, Some(&outer_key)).unwrap();
    //
    //     assert_eq!(decoded.header.protocol_version, 1);
    //     assert_eq!(decoded.header.receiver_idx, 12345);
    //     assert_eq!(decoded.header.counter, 999);
    //     assert!(matches!(decoded.message, LpMessage::Busy));
    // }
    //
    // #[test]
    // fn test_aead_ciphertext_differs_from_plaintext() {
    //     // Verify that encrypted payload differs from plaintext
    //     let psk = [42u8; 32];
    //     let outer_key = OuterAeadKey::from_psk(&psk);
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 12345,
    //             counter: 999,
    //         },
    //         message: LpMessage::EncryptedData(crate::message::EncryptedDataPayload(vec![
    //             0xAA, 0xBB, 0xCC, 0xDD,
    //         ])),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let mut cleartext = BytesMut::new();
    //     serialize_lp_packet(&packet, &mut cleartext, None).unwrap();
    //
    //     let mut encrypted = BytesMut::new();
    //     serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();
    //
    //     // Outer header (receiver_idx + counter) should be the same - always cleartext
    //     assert_eq!(&cleartext[..OUTER_HDR], &encrypted[..OUTER_HDR]);
    //
    //     // Inner payload (proto + reserved + msg_type + content) should differ (encrypted)
    //     let payload_start = OUTER_HDR;
    //     let payload_end_cleartext = cleartext.len() - TRAILER_LEN;
    //     let payload_end_encrypted = encrypted.len() - TRAILER_LEN;
    //
    //     assert_ne!(
    //         &cleartext[payload_start..payload_end_cleartext],
    //         &encrypted[payload_start..payload_end_encrypted],
    //         "Encrypted payload should differ from plaintext"
    //     );
    //
    //     // Trailer should differ (zeros vs AEAD tag)
    //     assert_ne!(
    //         &cleartext[payload_end_cleartext..],
    //         &encrypted[payload_end_encrypted..],
    //         "Encrypted trailer should be a tag, not zeros"
    //     );
    // }
    //
    // #[test]
    // fn test_aead_tampered_tag_fails() {
    //     // Verify that tampering with the tag causes decryption failure
    //     let psk = [42u8; 32];
    //     let outer_key = OuterAeadKey::from_psk(&psk);
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 12345,
    //             counter: 999,
    //         },
    //         message: LpMessage::Busy,
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let mut encrypted = BytesMut::new();
    //     serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();
    //
    //     // Tamper with the tag (last byte)
    //     let last_idx = encrypted.len() - 1;
    //     encrypted[last_idx] ^= 0xFF;
    //
    //     // Parsing should fail with AeadTagMismatch
    //     let result = parse_lp_packet(&encrypted, Some(&outer_key));
    //     assert!(matches!(result, Err(LpError::AeadTagMismatch)));
    // }
    //
    // #[test]
    // fn test_aead_tampered_header_fails() {
    //     // Verify that tampering with the header (AAD) causes decryption failure
    //     let psk = [42u8; 32];
    //     let outer_key = OuterAeadKey::from_psk(&psk);
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 12345,
    //             counter: 999,
    //         },
    //         message: LpMessage::Busy,
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let mut encrypted = BytesMut::new();
    //     serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();
    //
    //     // Tamper with the outer header AAD (flip a bit in counter at byte 4)
    //     // New format: [receiver_idx(0-3), counter(4-11)], so byte 4 is counter's LSB
    //     encrypted[4] ^= 0x01;
    //
    //     // Parsing should fail with AeadTagMismatch
    //     let result = parse_lp_packet(&encrypted, Some(&outer_key));
    //     assert!(matches!(result, Err(LpError::AeadTagMismatch)));
    // }
    //
    // #[test]
    // fn test_aead_different_counters_produce_different_ciphertext() {
    //     // Verify that different counters (nonces) produce different ciphertexts
    //     let psk = [42u8; 32];
    //     let outer_key = OuterAeadKey::from_psk(&psk);
    //
    //     let packet1 = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 12345,
    //             counter: 1,
    //         },
    //         message: LpMessage::Busy,
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let packet2 = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 12345,
    //             counter: 2, // Different counter
    //         },
    //         message: LpMessage::Busy,
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let mut encrypted1 = BytesMut::new();
    //     serialize_lp_packet(&packet1, &mut encrypted1, Some(&outer_key)).unwrap();
    //
    //     let mut encrypted2 = BytesMut::new();
    //     serialize_lp_packet(&packet2, &mut encrypted2, Some(&outer_key)).unwrap();
    //
    //     // The encrypted inner payloads should differ even though the message is the same
    //     // (because nonce is different). Inner payload starts after outer header.
    //     let payload_start = OUTER_HDR;
    //     assert_ne!(
    //         &encrypted1[payload_start..],
    //         &encrypted2[payload_start..],
    //         "Different counters should produce different ciphertexts"
    //     );
    // }
    //
    // #[test]
    // fn test_aead_wrong_key_fails() {
    //     // Verify that decryption with wrong key fails
    //     let psk1 = [42u8; 32];
    //     let psk2 = [43u8; 32]; // Different PSK
    //     let outer_key1 = OuterAeadKey::from_psk(&psk1);
    //     let outer_key2 = OuterAeadKey::from_psk(&psk2);
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 12345,
    //             counter: 999,
    //         },
    //         message: LpMessage::Busy,
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let mut encrypted = BytesMut::new();
    //     serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key1)).unwrap();
    //
    //     // Parsing with wrong key should fail
    //     let result = parse_lp_packet(&encrypted, Some(&outer_key2));
    //     assert!(matches!(result, Err(LpError::AeadTagMismatch)));
    // }
    //
    // #[test]
    // fn test_aead_encrypted_data_message_roundtrip() {
    //     // Test AEAD with EncryptedData message type (larger payload)
    //     let psk = [42u8; 32];
    //     let outer_key = OuterAeadKey::from_psk(&psk);
    //
    //     let payload_data = vec![0xDE; 100];
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 54321,
    //             counter: 12345678,
    //         },
    //         message: LpMessage::EncryptedData(crate::message::EncryptedDataPayload(
    //             payload_data.clone(),
    //         )),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let mut encrypted = BytesMut::new();
    //     serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();
    //
    //     let decoded = parse_lp_packet(&encrypted, Some(&outer_key)).unwrap();
    //
    //     match decoded.message {
    //         LpMessage::EncryptedData(data) => {
    //             assert_eq!(data.0, payload_data);
    //         }
    //         _ => panic!("Expected EncryptedData message"),
    //     }
    // }
    //
    // #[test]
    // fn test_aead_handshake_message_roundtrip() {
    //     // Test AEAD with Handshake message type
    //     let psk = [42u8; 32];
    //     let outer_key = OuterAeadKey::from_psk(&psk);
    //
    //     let handshake_data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 99999,
    //             counter: 2,
    //         },
    //         message: LpMessage::PSQRequest(PSQRequestData(handshake_data.clone())),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let mut encrypted = BytesMut::new();
    //     serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();
    //
    //     let decoded = parse_lp_packet(&encrypted, Some(&outer_key)).unwrap();
    //
    //     match decoded.message {
    //         LpMessage::PSQResponse(data) => {
    //             assert_eq!(data.0, handshake_data);
    //         }
    //         _ => panic!("Expected Handshake message"),
    //     }
    // }
    //
    // // === Subsession Message Tests ===
    //
    // #[test]
    // fn test_serialize_parse_subsession_request() {
    //     let mut dst = BytesMut::new();
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 42,
    //             counter: 100,
    //         },
    //         message: LpMessage::SubsessionRequest,
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     assert_eq!(decoded.header.receiver_idx, 42);
    //     assert_eq!(decoded.header.counter, 100);
    //     assert!(matches!(decoded.message, LpMessage::SubsessionRequest));
    // }
    //
    // #[test]
    // fn test_serialize_parse_subsession_kk1() {
    //     use crate::message::SubsessionKK1Data;
    //
    //     let mut dst = BytesMut::new();
    //
    //     let kk1_data = SubsessionKK1Data {
    //         payload: vec![0xAA; 50], // 50 bytes KK payload
    //     };
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 123,
    //             counter: 456,
    //         },
    //         message: LpMessage::SubsessionKK1(kk1_data.clone()),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     assert_eq!(decoded.header.receiver_idx, 123);
    //     match decoded.message {
    //         LpMessage::SubsessionKK1(data) => {
    //             assert_eq!(data.payload, kk1_data.payload);
    //         }
    //         _ => panic!("Expected SubsessionKK1 message"),
    //     }
    // }
    //
    // #[test]
    // fn test_serialize_parse_subsession_kk2() {
    //     use crate::message::SubsessionKK2Data;
    //
    //     let mut dst = BytesMut::new();
    //
    //     let kk2_data = SubsessionKK2Data {
    //         payload: vec![0x11; 60], // 60 bytes KK response payload
    //     };
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 789,
    //             counter: 1000,
    //         },
    //         message: LpMessage::SubsessionKK2(kk2_data.clone()),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     assert_eq!(decoded.header.receiver_idx, 789);
    //     match decoded.message {
    //         LpMessage::SubsessionKK2(data) => {
    //             assert_eq!(data.payload, kk2_data.payload);
    //         }
    //         _ => panic!("Expected SubsessionKK2 message"),
    //     }
    // }
    //
    // #[test]
    // fn test_serialize_parse_subsession_ready() {
    //     use crate::message::SubsessionReadyData;
    //
    //     let mut dst = BytesMut::new();
    //
    //     let ready_data = SubsessionReadyData {
    //         receiver_index: 99999,
    //     };
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 42,
    //             counter: 200,
    //         },
    //         message: LpMessage::SubsessionReady(ready_data.clone()),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     assert_eq!(decoded.header.receiver_idx, 42);
    //     match decoded.message {
    //         LpMessage::SubsessionReady(data) => {
    //             assert_eq!(data.receiver_index, 99999);
    //         }
    //         _ => panic!("Expected SubsessionReady message"),
    //     }
    // }
    //
    // #[test]
    // fn test_subsession_request_with_payload_fails() {
    //     // SubsessionRequest should have no payload
    //     let mut buf = BytesMut::new();
    //     buf.extend_from_slice(&42u32.to_le_bytes()); // receiver_idx
    //     buf.extend_from_slice(&123u64.to_le_bytes()); // counter
    //     buf.extend_from_slice(&[1, 0, 0, 0]); // version + reserved
    //     buf.extend_from_slice(&MessageType::SubsessionRequest.to_u32().to_le_bytes());
    //     buf.extend_from_slice(&[0xFF]); // Invalid payload for SubsessionRequest
    //     buf.extend_from_slice(&[0; TRAILER_LEN]);
    //
    //     let result = parse_lp_packet(&buf, None);
    //     assert!(matches!(
    //         result,
    //         Err(LpError::InvalidPayloadSize {
    //             expected: 0,
    //             actual: 1
    //         })
    //     ));
    // }
    //
    // #[test]
    // fn test_aead_subsession_roundtrip() {
    //     use crate::message::SubsessionKK1Data;
    //
    //     let psk = [42u8; 32];
    //     let outer_key = OuterAeadKey::from_psk(&psk);
    //
    //     let kk1_data = SubsessionKK1Data {
    //         payload: vec![0xDE; 48], // 48 bytes KK payload
    //     };
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 54321,
    //             counter: 999,
    //         },
    //         message: LpMessage::SubsessionKK1(kk1_data.clone()),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     let mut encrypted = BytesMut::new();
    //     serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();
    //
    //     let decoded = parse_lp_packet(&encrypted, Some(&outer_key)).unwrap();
    //
    //     match decoded.message {
    //         LpMessage::SubsessionKK1(data) => {
    //             assert_eq!(data.payload, kk1_data.payload);
    //         }
    //         _ => panic!("Expected SubsessionKK1 message"),
    //     }
    // }
    //
    // #[test]
    // fn test_serialize_parse_error() {
    //     use crate::message::ErrorPacketData;
    //
    //     let mut dst = BytesMut::new();
    //
    //     let error_data = ErrorPacketData {
    //         message: "this is an error".to_string(),
    //     };
    //
    //     let packet = LpPacket {
    //         header: LpHeader {
    //             protocol_version: 1,
    //             reserved: [0u8; 3],
    //             receiver_idx: 42,
    //             counter: 200,
    //         },
    //         message: LpMessage::Error(error_data.clone()),
    //         trailer: [0; TRAILER_LEN],
    //     };
    //
    //     serialize_lp_packet(&packet, &mut dst, None).unwrap();
    //     let decoded = parse_lp_packet(&dst, None).unwrap();
    //
    //     assert_eq!(decoded.header.receiver_idx, 42);
    //     match decoded.message {
    //         LpMessage::Error(data) => {
    //             assert_eq!(data.message, "this is an error");
    //         }
    //         _ => panic!("Expected Error message"),
    //     }
    // }
}
