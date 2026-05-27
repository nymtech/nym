// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::{Context, bail};
use nym_sphinx_params::PacketSize;
use nym_sphinx_types::{Payload, PayloadKey, SphinxHeader, SphinxPacket};
use time::OffsetDateTime;

/// A pre-built sphinx packet header that can be reused across multiple test packets.
///
/// When `config.reuse_header` is enabled the agent constructs one header for the entire
/// test run and stamps a fresh [`TestPacketContent`] (new ID + timestamp) into each
/// packet's payload. This lets the agent avoid performing expensive packet derivation
/// for each sent payload.
pub(crate) struct TestPacketHeader {
    /// The immutable sphinx routing header shared across all replayed packets.
    pub(crate) header: SphinxHeader,

    /// List of payload keys derived when the header was built
    pub(crate) payload_key: Vec<PayloadKey>,
}

impl Clone for TestPacketHeader {
    fn clone(&self) -> Self {
        TestPacketHeader {
            header: SphinxHeader {
                shared_secret: self.header.shared_secret,
                routing_info: self.header.routing_info.clone(),
            },
            payload_key: self.payload_key.clone(),
        }
    }
}

impl TestPacketHeader {
    /// Encapsulates `content` into a new [`SphinxPacket`] by reusing the pre-built header.
    pub(crate) fn create_test_packet(
        &self,
        content: TestPacketContent,
    ) -> anyhow::Result<SphinxPacket> {
        let payload = Payload::encapsulate_message(
            &content.to_bytes(),
            &self.payload_key,
            PacketSize::AckPacket.payload_size(),
        )?;
        Ok(SphinxPacket {
            header: SphinxHeader {
                shared_secret: self.header.shared_secret,
                routing_info: self.header.routing_info.clone(),
            },
            payload,
        })
    }

    /// Decrypts a received payload using the last payload key (the one belonging to this
    /// agent as the final hop) and deserialises it into a [`TestPacketContent`].
    pub(crate) fn recover_payload(&self, received: Payload) -> anyhow::Result<TestPacketContent> {
        let key = self
            .payload_key
            .last()
            .context("no payload keys generated")?;

        let payload = received.unwrap(key)?.recover_plaintext()?;
        TestPacketContent::from_bytes(&payload)
    }
}

/// The payload embedded in every test sphinx packet.
///
/// Serialises to exactly 16 bytes: 8 bytes for `id` (big-endian `u64`) followed by
/// 8 bytes for `sending_timestamp` (big-endian Unix timestamp in nanoseconds as `i64`).
/// Nanosecond precision is preserved for dates up to year 2262 (i64 max ≈ 9.2*10^18 ns).
/// The timestamp is used to compute the packet's round-trip time on receipt.
#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct TestPacketContent {
    /// Monotonically increasing ID assigned by the agent; used to detect duplicates and
    /// correlate sent packets with received ones.
    pub(crate) id: u64,

    /// UTC wall-clock time at which the packet was created, used to compute RTT.
    pub(crate) sending_timestamp: OffsetDateTime,
}

impl TestPacketContent {
    /// Creates a new content value with the given `id` and the current UTC time.
    pub(crate) fn new(id: u64) -> Self {
        Self {
            id,
            sending_timestamp: OffsetDateTime::now_utc(),
        }
    }

    /// Serialises the content to 16 bytes: `id` as big-endian u64, then
    /// `sending_timestamp` as a big-endian i64 Unix timestamp in nanoseconds.
    pub(crate) fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&self.id.to_be_bytes());
        // unix_timestamp_nanos() returns i128, but the value fits in i64 for dates up to year 2262.
        #[allow(clippy::cast_possible_truncation)]
        bytes.extend_from_slice(
            &(self.sending_timestamp.unix_timestamp_nanos() as i64).to_be_bytes(),
        );
        bytes
    }

    /// Deserialises content from a 16-byte slice produced by [`to_bytes`](Self::to_bytes).
    /// Returns an error if the slice is not exactly 16 bytes or the timestamp is out of range.
    pub(crate) fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 16 {
            bail!("malformed test packet received")
        }

        let id = u64::from_be_bytes(bytes[0..8].try_into()?);
        let nanos = i64::from_be_bytes(bytes[8..16].try_into()?);
        Ok(Self {
            id,
            sending_timestamp: OffsetDateTime::from_unix_timestamp_nanos(nanos as i128)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn content_with_timestamp(id: u64, ts: OffsetDateTime) -> TestPacketContent {
        TestPacketContent {
            id,
            sending_timestamp: ts,
        }
    }

    #[test]
    fn serialised_length_is_always_16_bytes() {
        let content = TestPacketContent::new(0);
        assert_eq!(content.to_bytes().len(), 16);
    }

    #[test]
    fn roundtrip_preserves_all_fields() {
        // Use a fixed timestamp to avoid sub-nanosecond clock jitter in the test.
        let original = content_with_timestamp(42, datetime!(2025-06-01 12:00:00 UTC));
        let bytes = original.to_bytes();
        let recovered = TestPacketContent::from_bytes(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn roundtrip_preserves_nanosecond_precision() {
        // Construct a timestamp with a sub-second component to verify nanos are not truncated.
        let ts = datetime!(2025-06-01 12:00:00.123456789 UTC);
        let original = content_with_timestamp(1, ts);
        let recovered = TestPacketContent::from_bytes(&original.to_bytes()).unwrap();
        assert_eq!(
            original.sending_timestamp.unix_timestamp_nanos(),
            recovered.sending_timestamp.unix_timestamp_nanos()
        );
    }

    #[test]
    fn id_zero_and_max_roundtrip() {
        for id in [0u64, u64::MAX] {
            let original = content_with_timestamp(id, datetime!(2025-01-01 00:00:00 UTC));
            let recovered = TestPacketContent::from_bytes(&original.to_bytes()).unwrap();
            assert_eq!(recovered.id, id);
        }
    }

    #[test]
    fn id_is_encoded_in_first_8_bytes_big_endian() {
        let content = content_with_timestamp(1, datetime!(2025-01-01 00:00:00 UTC));
        let bytes = content.to_bytes();
        let id_bytes: [u8; 8] = bytes[0..8].try_into().unwrap();
        assert_eq!(u64::from_be_bytes(id_bytes), 1u64);
    }

    #[test]
    fn timestamp_is_encoded_in_last_8_bytes_big_endian() {
        let ts = datetime!(2025-01-01 00:00:00 UTC);
        let content = content_with_timestamp(0, ts);
        let bytes = content.to_bytes();
        let ts_bytes: [u8; 8] = bytes[8..16].try_into().unwrap();
        assert_eq!(
            i64::from_be_bytes(ts_bytes),
            ts.unix_timestamp_nanos() as i64
        );
    }

    #[test]
    fn from_bytes_rejects_too_short() {
        assert!(TestPacketContent::from_bytes(&[0u8; 15]).is_err());
    }

    #[test]
    fn from_bytes_rejects_too_long() {
        assert!(TestPacketContent::from_bytes(&[0u8; 17]).is_err());
    }

    #[test]
    fn from_bytes_rejects_empty() {
        assert!(TestPacketContent::from_bytes(&[]).is_err());
    }
}
