// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpError;
use crate::message::{
    ClientHelloData, EncryptedDataPayload, ForwardPacketData, HandshakeData, KKTRequestData,
    KKTResponseData, LpMessage, MessageType,
};
use crate::packet::{LpHeader, LpPacket, TRAILER_LEN};
use bytes::{BufMut, BytesMut};
use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    ChaCha20Poly1305, Key, Nonce, Tag,
};

/// Outer AEAD key for LP packet encryption.
///
/// Derived from PSK using Blake3 KDF with domain separation.
/// Used for opportunistic encryption: before PSK packets are cleartext,
/// after PSK packets have encrypted payload and authenticated header.
///
/// # Security: Nonce Reuse Prevention
///
/// ChaCha20-Poly1305 requires unique nonces per key. The counter starts at 0
/// for each session, which is safe because:
///
/// 1. **PSK is always fresh**: Each handshake uses PSQ 
///    with a client-generated random salt. This ensures a unique
///    PSK for every session, even between the same client-gateway pair.
///
/// 2. **Key derivation**: `outer_key = Blake3_KDF("lp-outer-aead", PSK)`.
///    Different PSK → different outer_key → nonce reuse impossible.
///
/// 3. **No PSK persistence**: PSK handles are not stored/reused across sessions.
///    Each connection performs fresh KKT+PSQ handshake.
///
#[derive(Clone)]
pub struct OuterAeadKey {
    key: [u8; 32],
}

impl OuterAeadKey {
    /// KDF context for outer AEAD key derivation (domain separation)
    const KDF_CONTEXT: &'static str = "lp-outer-aead";

    /// Derive outer AEAD key from PSK.
    ///
    /// Uses Blake3 KDF with domain separation to avoid key reuse
    /// between the outer AEAD layer and the inner Noise layer.
    pub fn from_psk(psk: &[u8; 32]) -> Self {
        let key = nym_crypto::kdf::derive_key_blake3(Self::KDF_CONTEXT, psk, &[]);
        Self { key }
    }

    /// Get reference to the raw key bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }
}

impl Drop for OuterAeadKey {
    fn drop(&mut self) {
        // Zeroize key material on drop
        self.key.iter_mut().for_each(|b| *b = 0);
    }
}

impl std::fmt::Debug for OuterAeadKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OuterAeadKey")
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Build 12-byte nonce from 8-byte counter (zero-padded).
///
/// Format: counter (8 bytes LE) || 0x00000000 (4 bytes)
fn build_nonce(counter: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&counter.to_le_bytes());
    // bytes 8..12 remain zero (zero-padding)
    nonce
}

/// Parse message from raw type and content bytes.
///
/// Used when decrypting outer-encrypted packets where the message type
/// was encrypted along with the content.
fn parse_message_from_type_and_content(
    msg_type_raw: u16,
    content: &[u8],
) -> Result<LpMessage, LpError> {
    let message_type = MessageType::from_u16(msg_type_raw)
        .ok_or_else(|| LpError::invalid_message_type(msg_type_raw))?;

    match message_type {
        MessageType::Busy => {
            if !content.is_empty() {
                return Err(LpError::InvalidPayloadSize {
                    expected: 0,
                    actual: content.len(),
                });
            }
            Ok(LpMessage::Busy)
        }
        MessageType::Handshake => Ok(LpMessage::Handshake(HandshakeData(content.to_vec()))),
        MessageType::EncryptedData => {
            Ok(LpMessage::EncryptedData(EncryptedDataPayload(content.to_vec())))
        }
        MessageType::ClientHello => {
            let data: ClientHelloData = bincode::deserialize(content)
                .map_err(|e| LpError::DeserializationError(e.to_string()))?;
            Ok(LpMessage::ClientHello(data))
        }
        MessageType::KKTRequest => Ok(LpMessage::KKTRequest(KKTRequestData(content.to_vec()))),
        MessageType::KKTResponse => Ok(LpMessage::KKTResponse(KKTResponseData(content.to_vec()))),
        MessageType::ForwardPacket => {
            let data: ForwardPacketData = bincode::deserialize(content)
                .map_err(|e| LpError::DeserializationError(e.to_string()))?;
            Ok(LpMessage::ForwardPacket(data))
        }
        MessageType::Collision => {
            if !content.is_empty() {
                return Err(LpError::InvalidPayloadSize {
                    expected: 0,
                    actual: content.len(),
                });
            }
            Ok(LpMessage::Collision)
        }
        MessageType::Ack => {
            if !content.is_empty() {
                return Err(LpError::InvalidPayloadSize {
                    expected: 0,
                    actual: content.len(),
                });
            }
            Ok(LpMessage::Ack)
        }
    }
}

/// Parse only the LP header from raw packet bytes.
///
/// Used for routing before session lookup when the header is always cleartext.
/// This allows the caller to determine the receiver_idx and look up the appropriate
/// session to get the outer AEAD key before calling `parse_lp_packet()`.
///
/// # Arguments
/// * `src` - Raw packet bytes (at least LpHeader::SIZE bytes)
///
/// # Errors
/// * `LpError::InsufficientBufferSize` - Packet too small for header
pub fn parse_lp_header_only(src: &[u8]) -> Result<LpHeader, LpError> {
    if src.len() < LpHeader::SIZE {
        return Err(LpError::InsufficientBufferSize);
    }
    LpHeader::parse(&src[..LpHeader::SIZE])
}

/// Parses a complete Lewes Protocol packet from a byte slice (e.g., a UDP datagram payload).
///
/// Assumes the input `src` contains exactly one complete packet. It does not handle
/// stream fragmentation or provide replay protection checks (these belong at the session level).
///
/// # Arguments
/// * `src` - Raw packet bytes
/// * `outer_key` - None for cleartext parsing, Some for AEAD decryption
///
/// # Errors
/// * `LpError::AeadTagMismatch` - Tag verification failed (when outer_key provided)
/// * `LpError::InsufficientBufferSize` - Packet too small
pub fn parse_lp_packet(
    src: &[u8],
    outer_key: Option<&OuterAeadKey>,
) -> Result<LpPacket, LpError> {
    // Minimum size check: LpHeader + Type + Trailer (for 0-payload message)
    let min_size = LpHeader::SIZE + 2 + TRAILER_LEN;
    if src.len() < min_size {
        return Err(LpError::InsufficientBufferSize);
    }

    // Parse LpHeader (always cleartext for routing)
    let header = LpHeader::parse(&src[..LpHeader::SIZE])?;

    // Extract trailer (potential AEAD tag)
    let trailer_start = src.len() - TRAILER_LEN;
    let mut trailer = [0u8; TRAILER_LEN];
    trailer.copy_from_slice(&src[trailer_start..]);

    // Payload is everything between header and trailer
    let payload_bytes = &src[LpHeader::SIZE..trailer_start];

    // Handle decryption if outer key provided
    let (message_type_raw, message_content) = match outer_key {
        None => {
            // Cleartext mode - parse directly
            if payload_bytes.len() < 2 {
                return Err(LpError::InsufficientBufferSize);
            }
            let msg_type = u16::from_le_bytes([payload_bytes[0], payload_bytes[1]]);
            (msg_type, &payload_bytes[2..])
        }
        Some(key) => {
            // AEAD decryption mode
            let nonce = build_nonce(header.counter);
            let aad = &src[..LpHeader::SIZE]; // Header as AAD

            // Copy payload for in-place decryption
            let mut decrypted = payload_bytes.to_vec();

            // Convert trailer to Tag
            let tag = Tag::from_slice(&trailer);

            // Decrypt and verify
            let cipher = ChaCha20Poly1305::new(Key::from_slice(key.as_bytes()));
            cipher
                .decrypt_in_place_detached(Nonce::from_slice(&nonce), aad, &mut decrypted, tag)
                .map_err(|_| LpError::AeadTagMismatch)?;

            // Extract message type from decrypted payload
            if decrypted.len() < 2 {
                return Err(LpError::InsufficientBufferSize);
            }
            let msg_type = u16::from_le_bytes([decrypted[0], decrypted[1]]);

            // Return decrypted content (owned, so we handle it differently)
            return parse_message_from_type_and_content(msg_type, &decrypted[2..]).map(|message| {
                LpPacket {
                    header,
                    message,
                    trailer,
                }
            });
        }
    };

    // Cleartext path: parse message from payload
    let message = parse_message_from_type_and_content(message_type_raw, message_content)?;

    Ok(LpPacket {
        header,
        message,
        trailer,
    })
}

/// Serializes an LpPacket into the provided BytesMut buffer.
///
/// # Arguments
/// * `item` - Packet to serialize
/// * `dst` - Output buffer
/// * `outer_key` - None for cleartext (uses packet's trailer), Some for AEAD encryption
///
/// When `outer_key` is provided:
/// - Header is written in cleartext (used as AAD)
/// - Message type + content is encrypted
/// - Trailer is set to the AEAD tag
pub fn serialize_lp_packet(
    item: &LpPacket,
    dst: &mut BytesMut,
    outer_key: Option<&OuterAeadKey>,
) -> Result<(), LpError> {
    match outer_key {
        None => {
            // Cleartext mode - use existing encode method
            dst.reserve(LpHeader::SIZE + 2 + item.message.len() + TRAILER_LEN);
            item.encode(dst);
            Ok(())
        }
        Some(key) => {
            // AEAD encryption mode
            dst.reserve(LpHeader::SIZE + 2 + item.message.len() + TRAILER_LEN);

            // 1. Encode header (AAD - not encrypted)
            let header_start = dst.len();
            item.header.encode(dst);
            let header_end = dst.len();

            // 2. Build plaintext: message_type (2B) + content
            let mut plaintext = BytesMut::new();
            plaintext.put_slice(&(item.message.typ() as u16).to_le_bytes());
            item.message.encode_content(&mut plaintext);

            // 3. Copy plaintext to dst for in-place encryption
            let payload_start = dst.len();
            dst.put_slice(&plaintext);

            // 4. Build nonce and get AAD
            let nonce = build_nonce(item.header.counter);
            let aad = &dst[header_start..header_end].to_vec(); // Copy AAD since we mutate dst

            // 5. Encrypt payload in-place
            let cipher = ChaCha20Poly1305::new(Key::from_slice(key.as_bytes()));
            let tag = cipher
                .encrypt_in_place_detached(
                    Nonce::from_slice(&nonce),
                    aad,
                    &mut dst[payload_start..],
                )
                .map_err(|_| LpError::Internal("AEAD encryption failed".to_string()))?;

            // 6. Append tag as trailer
            dst.put_slice(&tag);

            Ok(())
        }
    }
}

// Add a new error variant for invalid message types (Moved from previous impl LpError block)
impl LpError {
    pub fn invalid_message_type(message_type: u16) -> Self {
        LpError::InvalidMessageType(message_type)
    }
}

#[cfg(test)]
mod tests {
    // Import standalone functions
    use super::{parse_lp_packet, serialize_lp_packet, OuterAeadKey};
    // Keep necessary imports
    use crate::LpError;
    use crate::message::{EncryptedDataPayload, HandshakeData, LpMessage, MessageType};
    use crate::packet::{LpHeader, LpPacket, TRAILER_LEN};
    use bytes::BytesMut;

    // Header length: version(1) + reserved(3) + receiver_index(4) + counter(8) = 16 bytes
    const HEADER_LEN: usize = 16;

    // === Cleartext Encode/Decode Tests ===

    #[test]
    fn test_serialize_parse_busy() {
        let mut dst = BytesMut::new();

        // Create a Busy packet
        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 42,
                counter: 123,
            },
            message: LpMessage::Busy,
            trailer: [0; TRAILER_LEN],
        };

        // Serialize the packet (cleartext)
        serialize_lp_packet(&packet, &mut dst, None).unwrap();

        // Parse the packet (cleartext)
        let decoded = parse_lp_packet(&dst, None).unwrap();

        // Verify the packet fields
        assert_eq!(decoded.header.protocol_version, 1);
        assert_eq!(decoded.header.receiver_idx, 42);
        assert_eq!(decoded.header.counter, 123);
        assert!(matches!(decoded.message, LpMessage::Busy));
        assert_eq!(decoded.trailer, [0; TRAILER_LEN]);
    }

    #[test]
    fn test_serialize_parse_handshake() {
        let mut dst = BytesMut::new();

        // Create a Handshake message packet
        let payload = vec![42u8; 80]; // Example payload size
        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 42,
                counter: 123,
            },
            message: LpMessage::Handshake(HandshakeData(payload.clone())),
            trailer: [0; TRAILER_LEN],
        };

        // Serialize the packet (cleartext)
        serialize_lp_packet(&packet, &mut dst, None).unwrap();

        // Parse the packet (cleartext)
        let decoded = parse_lp_packet(&dst, None).unwrap();

        // Verify the packet fields
        assert_eq!(decoded.header.protocol_version, 1);
        assert_eq!(decoded.header.receiver_idx, 42);
        assert_eq!(decoded.header.counter, 123);

        // Verify message type and data
        match decoded.message {
            LpMessage::Handshake(decoded_payload) => {
                assert_eq!(decoded_payload, HandshakeData(payload));
            }
            _ => panic!("Expected Handshake message"),
        }
        assert_eq!(decoded.trailer, [0; TRAILER_LEN]);
    }

    #[test]
    fn test_serialize_parse_encrypted_data() {
        let mut dst = BytesMut::new();

        // Create an EncryptedData message packet
        let payload = vec![43u8; 124]; // Example payload size
        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 42,
                counter: 123,
            },
            message: LpMessage::EncryptedData(EncryptedDataPayload(payload.clone())),
            trailer: [0; TRAILER_LEN],
        };

        // Serialize the packet (cleartext)
        serialize_lp_packet(&packet, &mut dst, None).unwrap();

        // Parse the packet (cleartext)
        let decoded = parse_lp_packet(&dst, None).unwrap();

        // Verify the packet fields
        assert_eq!(decoded.header.protocol_version, 1);
        assert_eq!(decoded.header.receiver_idx, 42);
        assert_eq!(decoded.header.counter, 123);

        // Verify message type and data
        match decoded.message {
            LpMessage::EncryptedData(decoded_payload) => {
                assert_eq!(decoded_payload, EncryptedDataPayload(payload));
            }
            _ => panic!("Expected EncryptedData message"),
        }
        assert_eq!(decoded.trailer, [0; TRAILER_LEN]);
    }

    // === Incomplete Data Tests ===

    #[test]
    fn test_parse_incomplete_header() {
        // Create a buffer with incomplete header
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 0, 0, 0]); // Only 4 bytes, not enough for LpHeader::SIZE

        // Attempt to parse - expect error
        let result = parse_lp_packet(&buf, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LpError::InsufficientBufferSize
        ));
    }

    #[test]
    fn test_parse_incomplete_message_type() {
        // Create a buffer with complete header but incomplete message type
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
        buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
        buf.extend_from_slice(&[0]); // Only 1 byte of message type (need 2)

        // Buffer length = 16 + 1 = 17. Min size = 16 + 2 + 16 = 34.
        let result = parse_lp_packet(&buf, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LpError::InsufficientBufferSize
        ));
    }

    #[test]
    fn test_parse_incomplete_message_data() {
        // Create a buffer simulating Handshake but missing trailer and maybe partial payload
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
        buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
        buf.extend_from_slice(&MessageType::Handshake.to_u16().to_le_bytes()); // Handshake type
        buf.extend_from_slice(&[42; 40]); // 40 bytes of payload data

        // Buffer length = 16 + 2 + 40 = 58. Min size = 16 + 2 + 16 = 34.
        // Payload size calculated as 58 - 34 = 24.
        // Trailer expected at index 16 + 2 + 24 = 42.
        // Trailer read attempts src[42..58].
        // This *should* parse successfully based on the logic, but the trailer is garbage.
        // Let's rethink: parse_lp_packet assumes the *entire slice* is the packet.
        // If the slice doesn't end exactly where the trailer should, it's an error.
        // In this case, total length is 58. LpHeader(16) + Type(2) + Trailer(16) = 34. Payload = 58-34=24.
        // Trailer starts at 16+2+24 = 42. Ends at 42+16=58. It fits exactly.
        // This test *still* doesn't test incompleteness correctly for the datagram parser.

        // Let's test a buffer that's *too short* even for header+type+trailer+min_payload
        let mut buf_too_short = BytesMut::new();
        buf_too_short.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf_too_short.extend_from_slice(&42u32.to_le_bytes()); // Sender index
        buf_too_short.extend_from_slice(&123u64.to_le_bytes()); // Counter
        buf_too_short.extend_from_slice(&MessageType::Handshake.to_u16().to_le_bytes()); // Handshake type
        // No payload, no trailer. Length = 16+2=18. Min size = 34.
        let result_too_short = parse_lp_packet(&buf_too_short, None);
        assert!(result_too_short.is_err());
        assert!(matches!(
            result_too_short.unwrap_err(),
            LpError::InsufficientBufferSize
        ));

        // Test a buffer missing PART of the trailer
        let mut buf_partial_trailer = BytesMut::new();
        buf_partial_trailer.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf_partial_trailer.extend_from_slice(&42u32.to_le_bytes()); // Sender index
        buf_partial_trailer.extend_from_slice(&123u64.to_le_bytes()); // Counter
        buf_partial_trailer.extend_from_slice(&MessageType::Handshake.to_u16().to_le_bytes()); // Handshake type
        let payload = vec![42u8; 20]; // Assume 20 byte payload
        buf_partial_trailer.extend_from_slice(&payload);
        buf_partial_trailer.extend_from_slice(&[0; TRAILER_LEN - 1]); // Missing last byte of trailer

        // Total length = 16 + 2 + 20 + 15 = 53. Min size = 34. This passes.
        // Payload size = 53 - 34 = 19. <--- THIS IS WRONG. The parser assumes the length dictates payload.
        // Let's fix the parser logic slightly.

        // The point is, parse_lp_packet expects a COMPLETE datagram. Providing less bytes
        // than LpHeader + Type + Trailer should fail. Providing *more* is also an issue unless
        // the length calculation works out perfectly. The most direct test is just < min_size.
        // Renaming test to reflect this.
    }

    #[test]
    fn test_parse_buffer_smaller_than_minimum() {
        // Test a buffer that's smaller than the smallest possible packet (LpHeader+Type+Trailer)
        let mut buf_too_short = BytesMut::new();
        buf_too_short.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf_too_short.extend_from_slice(&42u32.to_le_bytes()); // Sender index
        buf_too_short.extend_from_slice(&123u64.to_le_bytes()); // Counter
        buf_too_short.extend_from_slice(&MessageType::Busy.to_u16().to_le_bytes()); // Type
        buf_too_short.extend_from_slice(&[0; TRAILER_LEN - 1]); // Missing last byte of trailer
        // Length = 16 + 2 + 15 = 33. Min Size = 34.
        let result_too_short = parse_lp_packet(&buf_too_short, None);
        assert!(
            result_too_short.is_err(),
            "Expected error for buffer size 33, min 34"
        );
        assert!(matches!(
            result_too_short.unwrap_err(),
            LpError::InsufficientBufferSize
        ));
    }

    #[test]
    fn test_parse_invalid_message_type() {
        // Create a buffer with invalid message type
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
        buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
        buf.extend_from_slice(&255u16.to_le_bytes()); // Invalid message type
        // Need payload and trailer to meet min_size requirement
        let payload_size = 10; // Arbitrary
        buf.extend_from_slice(&vec![0u8; payload_size]); // Some data
        buf.extend_from_slice(&[0; TRAILER_LEN]); // Trailer

        // Attempt to parse
        let result = parse_lp_packet(&buf, None);
        assert!(result.is_err());
        match result {
            Err(LpError::InvalidMessageType(255)) => {} // Expected error
            Err(e) => panic!("Expected InvalidMessageType error, got {:?}", e),
            Ok(_) => panic!("Expected error, but got Ok"),
        }
    }

    #[test]
    fn test_parse_incorrect_payload_size_for_busy() {
        // Create a Busy packet but *with* a payload (which is invalid)
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
        buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
        buf.extend_from_slice(&MessageType::Busy.to_u16().to_le_bytes()); // Busy type
        buf.extend_from_slice(&[42; 1]); // <<< Invalid 1-byte payload for Busy
        buf.extend_from_slice(&[0; TRAILER_LEN]); // Trailer

        // Total size = 16 + 2 + 1 + 16 = 35. Min size = 34.
        // Calculated payload size = 35 - 34 = 1.
        let result = parse_lp_packet(&buf, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LpError::InvalidPayloadSize {
                expected: 0,
                actual: 1
            }
        ));
    }

    // Test multiple packets simulation isn't relevant for datagram parsing
    // #[test]
    // fn test_multiple_packets_in_buffer() { ... }

    // === ClientHello Serialization Tests ===

    #[test]
    fn test_serialize_parse_client_hello() {
        use crate::message::ClientHelloData;

        let mut dst = BytesMut::new();

        // Create ClientHelloData
        let client_key = [42u8; 32];
        let client_ed25519_key = [43u8; 32];
        let salt = [99u8; 32];
        let hello_data = ClientHelloData {
            receiver_index: 12345,
            client_lp_public_key: client_key,
            client_ed25519_public_key: client_ed25519_key,
            salt,
        };

        // Create a ClientHello message packet
        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 42,
                counter: 123,
            },
            message: LpMessage::ClientHello(hello_data.clone()),
            trailer: [0; TRAILER_LEN],
        };

        // Serialize the packet
        serialize_lp_packet(&packet, &mut dst, None).unwrap();

        // Parse the packet
        let decoded = parse_lp_packet(&dst, None).unwrap();

        // Verify the packet fields
        assert_eq!(decoded.header.protocol_version, 1);
        assert_eq!(decoded.header.receiver_idx, 42);
        assert_eq!(decoded.header.counter, 123);

        // Verify message type and data
        match decoded.message {
            LpMessage::ClientHello(decoded_data) => {
                assert_eq!(decoded_data.client_lp_public_key, client_key);
                assert_eq!(decoded_data.salt, salt);
            }
            _ => panic!("Expected ClientHello message"),
        }
        assert_eq!(decoded.trailer, [0; TRAILER_LEN]);
    }

    #[test]
    fn test_serialize_parse_client_hello_with_fresh_salt() {
        use crate::message::ClientHelloData;

        let mut dst = BytesMut::new();

        // Create ClientHelloData with fresh salt
        let client_key = [7u8; 32];
        let client_ed25519_key = [8u8; 32];
        let hello_data = ClientHelloData::new_with_fresh_salt(client_key, client_ed25519_key);

        // Create a ClientHello message packet
        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 100,
                counter: 200,
            },
            message: LpMessage::ClientHello(hello_data.clone()),
            trailer: [55; TRAILER_LEN],
        };

        // Serialize the packet
        serialize_lp_packet(&packet, &mut dst, None).unwrap();

        // Parse the packet
        let decoded = parse_lp_packet(&dst, None).unwrap();

        // Verify message type and data
        match decoded.message {
            LpMessage::ClientHello(decoded_data) => {
                assert_eq!(decoded_data.client_lp_public_key, client_key);
                assert_eq!(decoded_data.salt, hello_data.salt);

                // Verify timestamp can be extracted
                let timestamp = decoded_data.extract_timestamp();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                // Timestamp should be within 2 seconds of now
                assert!((timestamp as i64 - now as i64).abs() <= 2);
            }
            _ => panic!("Expected ClientHello message"),
        }
    }

    #[test]
    fn test_parse_client_hello_malformed_bincode() {
        // Create a buffer with ClientHello message type but invalid bincode data
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
        buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
        buf.extend_from_slice(&MessageType::ClientHello.to_u16().to_le_bytes()); // ClientHello type

        // Add malformed bincode data (random bytes that won't deserialize to ClientHelloData)
        buf.extend_from_slice(&[0xFF; 50]); // Invalid bincode data
        buf.extend_from_slice(&[0; TRAILER_LEN]); // Trailer

        // Attempt to parse
        let result = parse_lp_packet(&buf, None);
        assert!(result.is_err());
        match result {
            Err(LpError::DeserializationError(_)) => {} // Expected error
            Err(e) => panic!("Expected DeserializationError, got {:?}", e),
            Ok(_) => panic!("Expected error, but got Ok"),
        }
    }

    #[test]
    fn test_parse_client_hello_incomplete_bincode() {
        // Create a buffer with ClientHello but truncated bincode data
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf.extend_from_slice(&42u32.to_le_bytes()); // Sender index
        buf.extend_from_slice(&123u64.to_le_bytes()); // Counter
        buf.extend_from_slice(&MessageType::ClientHello.to_u16().to_le_bytes()); // ClientHello type

        // Add incomplete bincode data (only partial ClientHelloData)
        buf.extend_from_slice(&[0; 20]); // Too few bytes for full ClientHelloData
        buf.extend_from_slice(&[0; TRAILER_LEN]); // Trailer

        // Attempt to parse
        let result = parse_lp_packet(&buf, None);
        assert!(result.is_err());
        match result {
            Err(LpError::DeserializationError(_)) => {} // Expected error
            Err(e) => panic!("Expected DeserializationError, got {:?}", e),
            Ok(_) => panic!("Expected error, but got Ok"),
        }
    }

    #[test]
    fn test_client_hello_different_protocol_versions() {
        use crate::message::ClientHelloData;

        for version in [0u8, 1, 2, 255] {
            let mut dst = BytesMut::new();

            let hello_data = ClientHelloData {
                receiver_index: version as u32,
                client_lp_public_key: [version; 32],
                client_ed25519_public_key: [version.wrapping_add(2); 32],
                salt: [version.wrapping_add(1); 32],
            };

            let packet = LpPacket {
                header: LpHeader {
                    protocol_version: 1,
                    reserved: 0,
                    receiver_idx: version as u32,
                    counter: version as u64,
                },
                message: LpMessage::ClientHello(hello_data.clone()),
                trailer: [version; TRAILER_LEN],
            };

            serialize_lp_packet(&packet, &mut dst, None).unwrap();
            let decoded = parse_lp_packet(&dst, None).unwrap();

            match decoded.message {
                LpMessage::ClientHello(decoded_data) => {
                    assert_eq!(decoded_data.client_lp_public_key, [version; 32]);
                }
                _ => panic!("Expected ClientHello message for version {}", version),
            }
        }
    }

    #[test]
    fn test_forward_packet_encode_decode_roundtrip() {
        let mut dst = BytesMut::new();

        let forward_data = crate::message::ForwardPacketData {
            target_gateway_identity: [77u8; 32],
            target_lp_address: "1.2.3.4:41264".to_string(),
            inner_packet_bytes: vec![0xa, 0xb, 0xc, 0xd],
        };

        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 999,
                counter: 555,
            },
            message: LpMessage::ForwardPacket(forward_data),
            trailer: [0xff; TRAILER_LEN],
        };

        // Serialize
        serialize_lp_packet(&packet, &mut dst, None).unwrap();

        // Parse back
        let decoded = parse_lp_packet(&dst, None).unwrap();

        // Verify LP protocol handling works correctly
        assert_eq!(decoded.header.receiver_idx, 999);
        assert!(matches!(decoded.message.typ(), MessageType::ForwardPacket));

        if let LpMessage::ForwardPacket(data) = decoded.message {
            assert_eq!(data.target_gateway_identity, [77u8; 32]);
            assert_eq!(data.target_lp_address, "1.2.3.4:41264");
            assert_eq!(data.inner_packet_bytes, vec![0xa, 0xb, 0xc, 0xd]);
        } else {
            panic!("Expected ForwardPacket message");
        }
    }

    // === Outer AEAD Tests ===

    #[test]
    fn test_aead_roundtrip_with_key() {
        // Test that encrypt/decrypt roundtrip works with an AEAD key
        let psk = [42u8; 32];
        let outer_key = OuterAeadKey::from_psk(&psk);

        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 12345,
                counter: 999,
            },
            message: LpMessage::Busy,
            trailer: [0; TRAILER_LEN],
        };

        let mut encrypted = BytesMut::new();
        serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();

        // Parse back with the same key
        let decoded = parse_lp_packet(&encrypted, Some(&outer_key)).unwrap();

        assert_eq!(decoded.header.protocol_version, 1);
        assert_eq!(decoded.header.receiver_idx, 12345);
        assert_eq!(decoded.header.counter, 999);
        assert!(matches!(decoded.message, LpMessage::Busy));
    }

    #[test]
    fn test_aead_ciphertext_differs_from_plaintext() {
        // Verify that encrypted payload differs from plaintext
        let psk = [42u8; 32];
        let outer_key = OuterAeadKey::from_psk(&psk);

        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 12345,
                counter: 999,
            },
            message: LpMessage::EncryptedData(crate::message::EncryptedDataPayload(vec![
                0xAA, 0xBB, 0xCC, 0xDD,
            ])),
            trailer: [0; TRAILER_LEN],
        };

        let mut cleartext = BytesMut::new();
        serialize_lp_packet(&packet, &mut cleartext, None).unwrap();

        let mut encrypted = BytesMut::new();
        serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();

        // Header should be the same (it's authenticated but not encrypted)
        assert_eq!(&cleartext[..HEADER_LEN], &encrypted[..HEADER_LEN]);

        // Payload should differ (it's encrypted)
        let payload_start = HEADER_LEN;
        let payload_end_cleartext = cleartext.len() - TRAILER_LEN;
        let payload_end_encrypted = encrypted.len() - TRAILER_LEN;

        assert_ne!(
            &cleartext[payload_start..payload_end_cleartext],
            &encrypted[payload_start..payload_end_encrypted],
            "Encrypted payload should differ from plaintext"
        );

        // Trailer should differ (zeros vs AEAD tag)
        assert_ne!(
            &cleartext[payload_end_cleartext..],
            &encrypted[payload_end_encrypted..],
            "Encrypted trailer should be a tag, not zeros"
        );
    }

    #[test]
    fn test_aead_tampered_tag_fails() {
        // Verify that tampering with the tag causes decryption failure
        let psk = [42u8; 32];
        let outer_key = OuterAeadKey::from_psk(&psk);

        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 12345,
                counter: 999,
            },
            message: LpMessage::Busy,
            trailer: [0; TRAILER_LEN],
        };

        let mut encrypted = BytesMut::new();
        serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();

        // Tamper with the tag (last byte)
        let last_idx = encrypted.len() - 1;
        encrypted[last_idx] ^= 0xFF;

        // Parsing should fail with AeadTagMismatch
        let result = parse_lp_packet(&encrypted, Some(&outer_key));
        assert!(matches!(result, Err(LpError::AeadTagMismatch)));
    }

    #[test]
    fn test_aead_tampered_header_fails() {
        // Verify that tampering with the header (AAD) causes decryption failure
        let psk = [42u8; 32];
        let outer_key = OuterAeadKey::from_psk(&psk);

        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 12345,
                counter: 999,
            },
            message: LpMessage::Busy,
            trailer: [0; TRAILER_LEN],
        };

        let mut encrypted = BytesMut::new();
        serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();

        // Tamper with the header (flip a bit in receiver_idx)
        encrypted[4] ^= 0x01;

        // Parsing should fail with AeadTagMismatch
        let result = parse_lp_packet(&encrypted, Some(&outer_key));
        assert!(matches!(result, Err(LpError::AeadTagMismatch)));
    }

    #[test]
    fn test_aead_different_counters_produce_different_ciphertext() {
        // Verify that different counters (nonces) produce different ciphertexts
        let psk = [42u8; 32];
        let outer_key = OuterAeadKey::from_psk(&psk);

        let packet1 = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 12345,
                counter: 1,
            },
            message: LpMessage::Busy,
            trailer: [0; TRAILER_LEN],
        };

        let packet2 = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 12345,
                counter: 2, // Different counter
            },
            message: LpMessage::Busy,
            trailer: [0; TRAILER_LEN],
        };

        let mut encrypted1 = BytesMut::new();
        serialize_lp_packet(&packet1, &mut encrypted1, Some(&outer_key)).unwrap();

        let mut encrypted2 = BytesMut::new();
        serialize_lp_packet(&packet2, &mut encrypted2, Some(&outer_key)).unwrap();

        // The encrypted payloads should differ even though the message is the same
        // (because nonce is different)
        let payload_start = HEADER_LEN;
        assert_ne!(
            &encrypted1[payload_start..],
            &encrypted2[payload_start..],
            "Different counters should produce different ciphertexts"
        );
    }

    #[test]
    fn test_aead_wrong_key_fails() {
        // Verify that decryption with wrong key fails
        let psk1 = [42u8; 32];
        let psk2 = [43u8; 32]; // Different PSK
        let outer_key1 = OuterAeadKey::from_psk(&psk1);
        let outer_key2 = OuterAeadKey::from_psk(&psk2);

        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 12345,
                counter: 999,
            },
            message: LpMessage::Busy,
            trailer: [0; TRAILER_LEN],
        };

        let mut encrypted = BytesMut::new();
        serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key1)).unwrap();

        // Parsing with wrong key should fail
        let result = parse_lp_packet(&encrypted, Some(&outer_key2));
        assert!(matches!(result, Err(LpError::AeadTagMismatch)));
    }

    #[test]
    fn test_aead_encrypted_data_message_roundtrip() {
        // Test AEAD with EncryptedData message type (larger payload)
        let psk = [42u8; 32];
        let outer_key = OuterAeadKey::from_psk(&psk);

        let payload_data = vec![0xDE; 100];
        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 54321,
                counter: 12345678,
            },
            message: LpMessage::EncryptedData(crate::message::EncryptedDataPayload(
                payload_data.clone(),
            )),
            trailer: [0; TRAILER_LEN],
        };

        let mut encrypted = BytesMut::new();
        serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();

        let decoded = parse_lp_packet(&encrypted, Some(&outer_key)).unwrap();

        match decoded.message {
            LpMessage::EncryptedData(data) => {
                assert_eq!(data.0, payload_data);
            }
            _ => panic!("Expected EncryptedData message"),
        }
    }

    #[test]
    fn test_aead_handshake_message_roundtrip() {
        // Test AEAD with Handshake message type
        let psk = [42u8; 32];
        let outer_key = OuterAeadKey::from_psk(&psk);

        let handshake_data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let packet = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                receiver_idx: 99999,
                counter: 2,
            },
            message: LpMessage::Handshake(HandshakeData(handshake_data.clone())),
            trailer: [0; TRAILER_LEN],
        };

        let mut encrypted = BytesMut::new();
        serialize_lp_packet(&packet, &mut encrypted, Some(&outer_key)).unwrap();

        let decoded = parse_lp_packet(&encrypted, Some(&outer_key)).unwrap();

        match decoded.message {
            LpMessage::Handshake(data) => {
                assert_eq!(data.0, handshake_data);
            }
            _ => panic!("Expected Handshake message"),
        }
    }
}
