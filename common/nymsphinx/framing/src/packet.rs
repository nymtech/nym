// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::SphinxCodecError;
use bytes::{BufMut, BytesMut};
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_params::PacketMode;
use nymsphinx_types::SphinxPacket;
use std::convert::TryFrom;

pub struct FramedSphinxPacket {
    /// Contains any metadata helping receiver to handle the underlying packet.
    pub(crate) header: Header,

    /// The actual SphinxPacket being sent.
    pub(crate) packet: SphinxPacket,
}

impl FramedSphinxPacket {
    pub fn new(packet: SphinxPacket, packet_mode: PacketMode) -> Self {
        // If this fails somebody is using the library in a super incorrect way, because they
        // already managed to somehow create a sphinx packet
        let packet_size = PacketSize::get_type(packet.len()).unwrap();
        FramedSphinxPacket {
            header: Header {
                packet_size,
                packet_mode,
            },
            packet,
        }
    }

    pub fn packet_size(&self) -> PacketSize {
        self.header.packet_size
    }

    pub fn packet_mode(&self) -> PacketMode {
        self.header.packet_mode
    }

    pub fn into_inner(self) -> SphinxPacket {
        self.packet
    }
}

// Contains any metadata that might be useful for sending between mix nodes.
// TODO: in theory all those data could be put in a single `u8` by setting appropriate bits,
// but would that really be worth it?
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
pub struct Header {
    /// Represents type and consequently size of the included SphinxPacket.
    pub(crate) packet_size: PacketSize,

    /// Represents whether this packet is sent in a `vpn_mode` meaning it should not get delayed
    /// and shared keys might get reused. Mixnodes are capable of inferring this mode from the
    /// delay values inside the packet header (i.e. being set to 0), however, gateway, being final
    /// hop, would be unable to do so.
    ///
    /// TODO: ask @AP whether this can be sent like this - could it introduce some anonymity issues?
    /// (note: this will be behind some encryption, either something implemented by us or some SSL action)
    // Note: currently packet_mode is deprecated but is still left as a concept behind to not break
    // compatibility with existing network
    pub(crate) packet_mode: PacketMode,
}

impl Header {
    pub(crate) const SIZE: usize = 2;

    pub(crate) fn encode(&self, dst: &mut BytesMut) {
        // we reserve one byte for `packet_size` and the other for `mode`
        dst.reserve(Self::SIZE);
        dst.put_u8(self.packet_size as u8);
        dst.put_u8(self.packet_mode as u8);
        // reserve bytes for the actual packet
        dst.reserve(self.packet_size.size());
    }

    pub(crate) fn decode(src: &mut BytesMut) -> Result<Option<Self>, SphinxCodecError> {
        if src.len() < Self::SIZE {
            // can't do anything if we don't have enough bytes - but reserve enough for the next call
            src.reserve(Self::SIZE);
            return Ok(None);
        }

        Ok(Some(Header {
            packet_size: PacketSize::try_from(src[0])?,
            packet_mode: PacketMode::try_from(src[1])?,
        }))
    }
}

#[cfg(test)]
mod header_encoding {
    use super::*;

    #[test]
    fn header_can_be_decoded_from_a_valid_encoded_instance() {
        let header = Header::default();
        let mut bytes = BytesMut::new();
        header.encode(&mut bytes);
        let decoded = Header::decode(&mut bytes).unwrap().unwrap();
        assert_eq!(decoded, header);
    }

    #[test]
    fn decoding_will_fail_for_unknown_packet_size() {
        let unknown_packet_size: u8 = 255;
        // make sure this is still 'unknown' for if we make changes in the future
        assert!(PacketSize::try_from(unknown_packet_size).is_err());

        let mut bytes = BytesMut::from([unknown_packet_size, PacketMode::default() as u8].as_ref());
        assert!(Header::decode(&mut bytes).is_err())
    }

    #[test]
    fn decoding_will_fail_for_unknown_packet_mode() {
        let unknown_packet_mode: u8 = 255;
        // make sure this is still 'unknown' for if we make changes in the future
        assert!(PacketMode::try_from(unknown_packet_mode).is_err());

        let mut bytes = BytesMut::from([PacketSize::default() as u8, unknown_packet_mode].as_ref());
        assert!(Header::decode(&mut bytes).is_err())
    }

    #[test]
    fn decode_will_allocate_enough_bytes_for_next_call() {
        let mut empty_bytes = BytesMut::new();
        let decode_attempt_1 = Header::decode(&mut empty_bytes).unwrap();
        assert!(decode_attempt_1.is_none());
        assert!(empty_bytes.capacity() > Header::SIZE);

        let mut empty_bytes = BytesMut::with_capacity(1);
        let decode_attempt_2 = Header::decode(&mut empty_bytes).unwrap();
        assert!(decode_attempt_2.is_none());
        assert!(empty_bytes.capacity() > Header::SIZE);
    }

    #[test]
    fn header_encoding_reserves_enough_bytes_for_full_sphinx_packet() {
        let packet_sizes = vec![
            PacketSize::AckPacket,
            PacketSize::RegularPacket,
            PacketSize::ExtendedPacket,
        ];
        for packet_size in packet_sizes {
            let header = Header {
                packet_size,
                packet_mode: Default::default(),
            };
            let mut bytes = BytesMut::new();
            header.encode(&mut bytes);
            assert_eq!(bytes.capacity(), bytes.len() + packet_size.size())
        }
    }
}
