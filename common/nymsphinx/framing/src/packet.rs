// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::NymCodecError;
use bytes::{BufMut, BytesMut};
use nym_sphinx_params::packet_sizes::PacketSize;
use nym_sphinx_params::packet_version::{PacketVersion, CURRENT_PACKET_VERSION};
use nym_sphinx_params::PacketType;
use nym_sphinx_types::NymPacket;

#[derive(Debug)]
pub struct FramedNymPacket {
    /// Contains any metadata helping receiver to handle the underlying packet.
    pub(crate) header: Header,

    /// The actual SphinxPacket being sent.
    pub(crate) packet: NymPacket,
}

impl FramedNymPacket {
    pub fn new(packet: NymPacket, packet_type: PacketType) -> Self {
        // If this fails somebody is using the library in a super incorrect way, because they
        // already managed to somehow create a sphinx packet
        let packet_size = PacketSize::get_type(packet.len()).unwrap();

        let header = Header {
            packet_version: PacketVersion::new(),
            packet_size,
            packet_type,
        };

        FramedNymPacket { header, packet }
    }

    pub fn header(&self) -> Header {
        self.header
    }

    pub fn packet_size(&self) -> PacketSize {
        self.header.packet_size
    }

    pub fn packet_type(&self) -> PacketType {
        self.header.packet_type
    }

    pub fn into_inner(self) -> NymPacket {
        self.packet
    }

    pub fn packet(&self) -> &NymPacket {
        &self.packet
    }

    pub fn is_sphinx(&self) -> bool {
        self.packet.is_sphinx()
    }
}

// Contains any metadata that might be useful for sending between mix nodes.
// TODO: in theory all those data could be put in a single `u8` by setting appropriate bits,
// but would that really be worth it?
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
pub struct Header {
    /// Represents the wire format version used to construct this packet.
    pub(crate) packet_version: PacketVersion,

    /// Represents type and consequently size of the included SphinxPacket.
    pub(crate) packet_size: PacketSize,

    /// Represents whether this packet is sent in a `vpn_mode` meaning it should not get delayed
    /// and shared keys might get reused. Mixnodes are capable of inferring this mode from the
    /// delay values inside the packet header (i.e. being set to 0), however, gateway, being final
    /// hop, would be unable to do so.
    ///
    /// TODO: ask @AP whether this can be sent like this - could it introduce some anonymity issues?
    /// (note: this will be behind some encryption, either something implemented by us or some SSL action)
    // Note: currently packet_type is deprecated but is still left as a concept behind to not break
    // compatibility with existing network
    pub(crate) packet_type: PacketType,
}

impl Header {
    pub(crate) const SIZE: usize = 3;

    pub fn outfox() -> Header {
        Header {
            packet_version: PacketVersion::default(),
            packet_size: PacketSize::OutfoxRegularPacket,
            packet_type: PacketType::Outfox,
        }
    }

    pub(crate) fn encode(&self, dst: &mut BytesMut) {
        dst.reserve(Self::SIZE);

        dst.put_u8(self.packet_version.as_u8());
        dst.put_u8(self.packet_size as u8);
        dst.put_u8(self.packet_type as u8);

        // reserve bytes for the actual packet
        dst.reserve(self.packet_size.size());
    }

    pub(crate) fn decode(src: &mut BytesMut) -> Result<Option<Self>, NymCodecError> {
        if src.len() < Self::SIZE {
            // can't do anything if we don't have enough bytes - but reserve enough for the next call
            src.reserve(Self::SIZE);
            return Ok(None);
        }

        let packet_version = PacketVersion::try_from(src[0])?;
        if packet_version > CURRENT_PACKET_VERSION {
            // received an unsupported packet version - we don't know how it's meant to look like!
            // (this is in preparation for the dual support of breaking sphinx changes)
            return Err(NymCodecError::UnsupportedPacketVersion {
                received: packet_version,
                max_supported: CURRENT_PACKET_VERSION,
            });
        }

        Ok(Some(Header {
            packet_version,
            packet_size: PacketSize::try_from(src[1])?,
            packet_type: PacketType::try_from(src[2])?,
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

        // unfortunately this will only work for the 'versioned' variant
        // due to the hack used to get legacy mode compatibility
        let mut bytes = BytesMut::from(
            [
                PacketVersion::new().as_u8(),
                unknown_packet_size,
                PacketType::default() as u8,
            ]
            .as_ref(),
        );
        assert!(Header::decode(&mut bytes).is_err())
    }

    #[test]
    fn decoding_will_fail_for_unknown_packet_type() {
        let unknown_packet_type: u8 = 255;
        // make sure this is still 'unknown' for if we make changes in the future
        assert!(PacketType::try_from(unknown_packet_type).is_err());

        let mut bytes = BytesMut::from(
            [
                PacketVersion::new().as_u8(),
                PacketSize::default() as u8,
                unknown_packet_type,
            ]
            .as_ref(),
        );
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
    fn header_encoding_reserves_enough_bytes_for_full_sphinx_packet_() {
        let packet_sizes = vec![
            PacketSize::AckPacket,
            PacketSize::RegularPacket,
            PacketSize::ExtendedPacket8,
            PacketSize::ExtendedPacket16,
            PacketSize::ExtendedPacket32,
        ];
        for packet_size in packet_sizes {
            let header = Header {
                packet_version: PacketVersion::new(),
                packet_size,
                ..Default::default()
            };
            let mut bytes = BytesMut::new();
            header.encode(&mut bytes);
            assert_eq!(bytes.capacity(), bytes.len() + packet_size.size())
        }
    }

    #[test]
    fn header_decoding_will_reject_future_versions() {
        let future_version = PacketVersion::try_from(123).unwrap();

        let unchecked_header = Header {
            packet_version: future_version,
            packet_size: PacketSize::RegularPacket,
            packet_type: PacketType::Mix,
        };
        let mut bytes = BytesMut::new();
        unchecked_header.encode(&mut bytes);

        match Header::decode(&mut bytes).unwrap_err() {
            NymCodecError::UnsupportedPacketVersion {
                received,
                max_supported,
            } => {
                assert_eq!(received, future_version);
                assert_eq!(max_supported, CURRENT_PACKET_VERSION);
            }
            _ => panic!("unexpected error variant"),
        }
    }
}
