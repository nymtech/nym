// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::packet::{FramedNymPacket, Header};
use bytes::{Buf, BufMut, BytesMut};
use nym_sphinx_params::packet_sizes::{InvalidPacketSize, PacketSize};
use nym_sphinx_params::packet_types::InvalidPacketType;
use nym_sphinx_params::PacketType;
use nym_sphinx_types::{NymPacket, NymPacketError};
use std::io;
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Error, Debug)]
pub enum NymCodecError {
    #[error("the packet size information was malformed - {0}")]
    InvalidPacketSize(#[from] InvalidPacketSize),

    #[error("the packet mode information was malformed - {0}")]
    InvalidPacketType(#[from] InvalidPacketType),

    #[error("encountered an IO error - {0}")]
    IoError(#[from] io::Error),

    #[error("encountered a packet error - {0}")]
    NymPacket(#[from] NymPacketError),

    #[error("could not convert to bytes")]
    ToBytes,

    #[error("could not convert to bytes")]
    FromBytes,
}

// TODO: in the future it could be extended to have state containing symmetric encryption key
// so that all data could be encrypted easily (alternatively we could just slap TLS)
pub struct NymCodec;

impl Encoder<FramedNymPacket> for NymCodec {
    type Error = NymCodecError;

    fn encode(&mut self, item: FramedNymPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.header.encode(dst);
        let packet_bytes = item.packet.to_bytes()?;
        let encoded = packet_bytes.as_slice();
        dst.put(encoded);
        Ok(())
    }
}

impl Decoder for NymCodec {
    type Item = FramedNymPacket;
    type Error = NymCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            // can't do anything if we have no bytes, but let's reserve enough for the most
            // conservative case, i.e. receiving an ack packet
            src.reserve(Header::LEGACY_SIZE + PacketSize::AckPacket.size());
            return Ok(None);
        }

        // because header is so small and simple it makes no point in trying to cache
        // this result. It will be just simpler to re-decode it
        let header = match Header::decode(src)? {
            Some(header) => header,
            None => return Ok(None), // we have some data but not enough to get header back
        };

        let packet_size = header.packet_size.size();
        let frame_len = header.size() + packet_size;

        if src.len() < frame_len {
            // we don't have enough bytes to read the rest of frame
            src.reserve(packet_size);
            return Ok(None);
        }

        // advance buffer past the header - at this point we have enough bytes
        src.advance(header.size());
        let packet_bytes = src.split_to(packet_size);
        let packet = if let Some(slice) = packet_bytes.get(..) {
            // here it could be debatable whether stream is corrupt or not,
            // but let's go with the safer approach and assume it is.
            match header.packet_type {
                PacketType::Outfox => NymPacket::outfox_from_bytes(slice)?,
                PacketType::Mix => NymPacket::sphinx_from_bytes(slice)?,
                #[allow(deprecated)]
                PacketType::Vpn => NymPacket::sphinx_from_bytes(slice)?,
            }
        } else {
            return Ok(None);
        };

        // let packet = SphinxPacket::from_bytes(&sphinx_packet_bytes)?;
        let nymsphinx_packet = FramedNymPacket { header, packet };

        // As per docs:
        // Before returning from the function, implementations should ensure that the buffer
        // has appropriate capacity in anticipation of future calls to decode.
        // Failing to do so leads to inefficiency.

        // if we have enough bytes to decode the header of the next packet, we can reserve enough bytes for
        // the entire next frame, if not, we assume the next frame is an ack packet and
        // reserve for that.
        // we also assume the next packet coming from the same client will use exactly the same versioning
        // as the current packet

        let mut allocate_for_next_packet = header.size() + PacketSize::AckPacket.size();
        if !src.is_empty() {
            match Header::decode(src) {
                Ok(Some(next_header)) => {
                    allocate_for_next_packet = next_header.size() + next_header.packet_size.size();
                }
                Ok(None) => {
                    // we don't have enough information to know how much to reserve, fallback to the ack case
                }

                // the next frame will be malformed but let's leave handling the error to the next
                // call to 'decode', as presumably, the current sphinx packet is still valid
                Err(_) => return Ok(Some(nymsphinx_packet)),
            };
        }
        src.reserve(allocate_for_next_packet);
        Ok(Some(nymsphinx_packet))
    }
}

#[cfg(test)]
mod packet_encoding {
    use super::*;
    use nym_sphinx_types::{
        test_utils, Delay as SphinxDelay, Destination, DestinationAddressBytes, Node,
        NodeAddressBytes, DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, NODE_ADDRESS_LENGTH,
    };

    fn make_valid_outfox_packet(size: PacketSize) -> NymPacket {
        let (_, node1_pk) = test_utils::fixtures::keygen();
        let node1 = Node::new(
            NodeAddressBytes::from_bytes([5u8; NODE_ADDRESS_LENGTH]),
            node1_pk,
        );
        let (_, node2_pk) = test_utils::fixtures::keygen();
        let node2 = Node::new(
            NodeAddressBytes::from_bytes([4u8; NODE_ADDRESS_LENGTH]),
            node2_pk,
        );
        let (_, node3_pk) = test_utils::fixtures::keygen();
        let node3 = Node::new(
            NodeAddressBytes::from_bytes([2u8; NODE_ADDRESS_LENGTH]),
            node3_pk,
        );

        let (_, node4_pk) = test_utils::fixtures::keygen();
        let node4 = Node::new(
            NodeAddressBytes::from_bytes([2u8; NODE_ADDRESS_LENGTH]),
            node4_pk,
        );

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
            [4u8; IDENTIFIER_LENGTH],
        );

        let route = &[node1, node2, node3, node4];

        let payload = vec![1; 48];

        NymPacket::outfox_build(payload, route, &destination, Some(size.plaintext_size())).unwrap()
    }

    fn make_valid_sphinx_packet(size: PacketSize) -> NymPacket {
        let (_, node1_pk) = test_utils::fixtures::keygen();
        let node1 = Node::new(
            NodeAddressBytes::from_bytes([5u8; NODE_ADDRESS_LENGTH]),
            node1_pk,
        );
        let (_, node2_pk) = test_utils::fixtures::keygen();
        let node2 = Node::new(
            NodeAddressBytes::from_bytes([4u8; NODE_ADDRESS_LENGTH]),
            node2_pk,
        );
        let (_, node3_pk) = test_utils::fixtures::keygen();
        let node3 = Node::new(
            NodeAddressBytes::from_bytes([2u8; NODE_ADDRESS_LENGTH]),
            node3_pk,
        );

        let route = [node1, node2, node3];
        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
            [4u8; IDENTIFIER_LENGTH],
        );
        let delays = vec![
            SphinxDelay::new_from_nanos(42),
            SphinxDelay::new_from_nanos(42),
            SphinxDelay::new_from_nanos(42),
        ];
        NymPacket::sphinx_build(size.payload_size(), b"foomp", &route, &destination, &delays)
            .unwrap()
    }

    #[test]
    fn whole_packet_can_be_decoded_from_a_valid_encoded_instance() {
        let header = Default::default();
        let sphinx_packet = make_valid_sphinx_packet(Default::default());
        let sphinx_bytes = sphinx_packet.to_bytes().unwrap();

        let packet = FramedNymPacket {
            header,
            packet: sphinx_packet,
        };

        let mut bytes = BytesMut::new();
        NymCodec.encode(packet, &mut bytes).unwrap();
        let decoded = NymCodec.decode(&mut bytes).unwrap().unwrap();

        assert_eq!(decoded.header, header);
        assert_eq!(decoded.packet.to_bytes().unwrap(), sphinx_bytes)
    }

    #[test]
    fn whole_outfox_can_be_decoded_from_a_valid_encoded_instance() {
        let header = Header::outfox();
        let packet = make_valid_outfox_packet(PacketSize::OutfoxRegularPacket);
        let packet_bytes = packet.to_bytes().unwrap();

        NymPacket::outfox_from_bytes(packet_bytes.as_slice()).unwrap();

        let packet = FramedNymPacket { header, packet };

        let mut bytes = BytesMut::new();
        NymCodec.encode(packet, &mut bytes).unwrap();
        let decoded = NymCodec.decode(&mut bytes).unwrap().unwrap();

        assert_eq!(decoded.header, header);
        assert_eq!(decoded.packet.to_bytes().unwrap(), packet_bytes)
    }

    #[cfg(test)]
    mod decode_will_allocate_enough_bytes_for_next_call {
        use super::*;
        use nym_sphinx_params::packet_version::PacketVersion;

        #[test]
        fn for_empty_bytes() {
            // empty bytes should allocate for header + ack packet
            let mut empty_bytes = BytesMut::new();
            assert!(NymCodec.decode(&mut empty_bytes).unwrap().is_none());
            assert_eq!(
                empty_bytes.capacity(),
                Header::LEGACY_SIZE + PacketSize::AckPacket.size()
            );
        }

        #[test]
        fn for_bytes_with_legacy_header() {
            // if header gets decoded there should be enough bytes for the entire frame
            let packet_sizes = vec![
                PacketSize::AckPacket,
                PacketSize::RegularPacket,
                PacketSize::ExtendedPacket8,
                PacketSize::ExtendedPacket16,
                PacketSize::ExtendedPacket32,
            ];
            for packet_size in packet_sizes {
                let header = Header {
                    packet_version: PacketVersion::Legacy,
                    packet_size,
                    ..Default::default()
                };
                let mut bytes = BytesMut::new();
                header.encode(&mut bytes);
                assert!(NymCodec.decode(&mut bytes).unwrap().is_none());

                assert_eq!(bytes.capacity(), Header::LEGACY_SIZE + packet_size.size())
            }
        }

        #[test]
        fn for_bytes_with_versioned_header() {
            // if header gets decoded there should be enough bytes for the entire frame
            let packet_sizes = vec![
                PacketSize::AckPacket,
                PacketSize::RegularPacket,
                PacketSize::ExtendedPacket8,
                PacketSize::ExtendedPacket16,
                PacketSize::ExtendedPacket32,
            ];
            for packet_size in packet_sizes {
                let header = Header {
                    packet_version: PacketVersion::Versioned(123),
                    packet_size,
                    ..Default::default()
                };
                let mut bytes = BytesMut::new();
                header.encode(&mut bytes);
                assert!(NymCodec.decode(&mut bytes).unwrap().is_none());

                assert_eq!(
                    bytes.capacity(),
                    Header::VERSIONED_SIZE + packet_size.size()
                )
            }
        }

        #[test]
        fn for_full_frame_with_legacy_header() {
            // if full frame is used exactly, there should be enough space for header + ack packet
            let packet = FramedNymPacket {
                header: Header {
                    packet_version: PacketVersion::Legacy,
                    ..Default::default()
                },
                packet: make_valid_sphinx_packet(Default::default()),
            };

            let mut bytes = BytesMut::new();
            NymCodec.encode(packet, &mut bytes).unwrap();
            assert!(NymCodec.decode(&mut bytes).unwrap().is_some());
            assert_eq!(
                bytes.capacity(),
                Header::LEGACY_SIZE + PacketSize::AckPacket.size()
            );
        }

        #[test]
        fn for_full_frame_with_versioned_header() {
            // if full frame is used exactly, there should be enough space for header + ack packet
            let packet = FramedNymPacket {
                header: Header::default(),
                packet: make_valid_sphinx_packet(Default::default()),
            };

            let mut bytes = BytesMut::new();
            NymCodec.encode(packet, &mut bytes).unwrap();
            assert!(NymCodec.decode(&mut bytes).unwrap().is_some());
            assert_eq!(
                bytes.capacity(),
                Header::VERSIONED_SIZE + PacketSize::AckPacket.size()
            );
        }

        #[test]
        fn for_full_frame_with_extra_bytes_with_legacy_header() {
            // if there was at least 2 byte left, there should be enough space for entire next frame
            let packet_sizes = vec![
                PacketSize::AckPacket,
                PacketSize::RegularPacket,
                PacketSize::ExtendedPacket8,
                PacketSize::ExtendedPacket16,
                PacketSize::ExtendedPacket32,
            ];

            for packet_size in packet_sizes {
                let first_packet = FramedNymPacket {
                    header: Header {
                        packet_version: PacketVersion::Legacy,
                        ..Default::default()
                    },
                    packet: make_valid_sphinx_packet(Default::default()),
                };

                let mut bytes = BytesMut::new();
                NymCodec.encode(first_packet, &mut bytes).unwrap();
                bytes.put_u8(packet_size as u8);
                bytes.put_u8(PacketType::default() as u8);
                assert!(NymCodec.decode(&mut bytes).unwrap().is_some());

                assert!(bytes.capacity() >= Header::LEGACY_SIZE + packet_size.size())
            }
        }

        #[test]
        fn for_full_frame_with_extra_bytes_with_versioned_header() {
            // if there was at least 3 byte left, there should be enough space for entire next frame
            let packet_sizes = vec![
                PacketSize::AckPacket,
                PacketSize::RegularPacket,
                PacketSize::ExtendedPacket8,
                PacketSize::ExtendedPacket16,
                PacketSize::ExtendedPacket32,
            ];

            for packet_size in packet_sizes {
                let first_packet = FramedNymPacket {
                    header: Header::default(),
                    packet: make_valid_sphinx_packet(Default::default()),
                };

                let mut bytes = BytesMut::new();
                NymCodec.encode(first_packet, &mut bytes).unwrap();
                bytes.put_u8(PacketVersion::new_versioned(123).as_u8().unwrap());
                bytes.put_u8(packet_size as u8);
                bytes.put_u8(PacketType::default() as u8);
                assert!(NymCodec.decode(&mut bytes).unwrap().is_some());

                // assert!(bytes.capacity() >= Header::VERSIONED_SIZE + packet_size.size())
            }
        }
    }

    #[test]
    fn can_decode_two_packets_immediately() {
        let packet1 = FramedNymPacket {
            header: Header::default(),
            packet: make_valid_sphinx_packet(Default::default()),
        };

        let packet2 = FramedNymPacket {
            header: Header::default(),
            packet: make_valid_sphinx_packet(Default::default()),
        };

        let mut bytes = BytesMut::new();

        NymCodec.encode(packet1, &mut bytes).unwrap();
        NymCodec.encode(packet2, &mut bytes).unwrap();

        assert!(NymCodec.decode(&mut bytes).unwrap().is_some());
        assert!(NymCodec.decode(&mut bytes).unwrap().is_some());
        assert!(NymCodec.decode(&mut bytes).unwrap().is_none());
    }

    #[test]
    fn can_decode_two_packets_in_separate_calls() {
        let packet1 = FramedNymPacket {
            header: Header::default(),
            packet: make_valid_sphinx_packet(Default::default()),
        };

        let packet2 = FramedNymPacket {
            header: Header::default(),
            packet: make_valid_sphinx_packet(Default::default()),
        };

        let mut bytes = BytesMut::new();
        let mut bytes_tmp = BytesMut::new();

        NymCodec.encode(packet1, &mut bytes).unwrap();
        NymCodec.encode(packet2, &mut bytes_tmp).unwrap();

        let tmp = bytes_tmp.split_off(100);
        bytes.put(bytes_tmp);

        assert!(NymCodec.decode(&mut bytes).unwrap().is_some());
        assert!(NymCodec.decode(&mut bytes).unwrap().is_none());

        bytes.put(tmp);
        assert!(NymCodec.decode(&mut bytes).unwrap().is_some());
        assert!(NymCodec.decode(&mut bytes).unwrap().is_none());
    }
}
