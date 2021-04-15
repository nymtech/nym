// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::packet::{FramedSphinxPacket, Header};
use bytes::{Buf, BufMut, BytesMut};
use nymsphinx_params::packet_modes::InvalidPacketMode;
use nymsphinx_params::packet_sizes::{InvalidPacketSize, PacketSize};
use nymsphinx_types::SphinxPacket;
use std::convert::TryFrom;
use std::io;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug)]
pub enum SphinxCodecError {
    InvalidPacketSize,
    InvalidPacketMode,
    MalformedSphinxPacket,
    IoError(io::Error),
}

impl From<io::Error> for SphinxCodecError {
    fn from(err: io::Error) -> Self {
        SphinxCodecError::IoError(err)
    }
}

impl From<SphinxCodecError> for io::Error {
    fn from(err: SphinxCodecError) -> Self {
        match err {
            SphinxCodecError::InvalidPacketSize => {
                io::Error::new(io::ErrorKind::InvalidInput, "invalid packet size")
            }
            SphinxCodecError::InvalidPacketMode => {
                io::Error::new(io::ErrorKind::InvalidInput, "invalid packet mode")
            }
            SphinxCodecError::MalformedSphinxPacket => {
                io::Error::new(io::ErrorKind::InvalidData, "malformed packet")
            }
            SphinxCodecError::IoError(err) => err,
        }
    }
}

impl From<InvalidPacketSize> for SphinxCodecError {
    fn from(_: InvalidPacketSize) -> Self {
        SphinxCodecError::InvalidPacketSize
    }
}

impl From<InvalidPacketMode> for SphinxCodecError {
    fn from(_: InvalidPacketMode) -> Self {
        SphinxCodecError::InvalidPacketMode
    }
}

// TODO: in the future it could be extended to have state containing symmetric encryption key
// so that all data could be encrypted easily (alternatively we could just slap TLS)
pub struct SphinxCodec;

impl Encoder<FramedSphinxPacket> for SphinxCodec {
    type Error = SphinxCodecError;

    fn encode(&mut self, item: FramedSphinxPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.header.encode(dst);
        dst.put(item.packet.to_bytes().as_ref());
        Ok(())
    }
}

impl Decoder for SphinxCodec {
    type Item = FramedSphinxPacket;
    type Error = SphinxCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            // can't do anything if we have no bytes, but let's reserve enough for the most
            // conservative case, i.e. receiving an ack packet
            src.reserve(Header::SIZE + PacketSize::AckPacket.size());
            return Ok(None);
        }

        // because header is so small and simple it makes no point in trying to cache
        // this result. It will be just simpler to re-decode it
        let header = match Header::decode(src)? {
            Some(header) => header,
            None => return Ok(None), // we have some data but not enough to get header back
        };

        let sphinx_packet_size = header.packet_size.size();
        let frame_len = Header::SIZE + sphinx_packet_size;

        if src.len() < frame_len {
            // we don't have enough bytes to read the rest of frame
            src.reserve(sphinx_packet_size);
            return Ok(None);
        }

        // advance buffer past the header - at this point we have enough bytes
        src.advance(Header::SIZE);
        let sphinx_packet_bytes = src.split_to(sphinx_packet_size);
        let sphinx_packet = match SphinxPacket::from_bytes(&sphinx_packet_bytes) {
            Ok(sphinx_packet) => sphinx_packet,
            // here it could be debatable whether stream is corrupt or not,
            // but let's go with the safer approach and assume it is.
            Err(_) => return Err(SphinxCodecError::MalformedSphinxPacket),
        };

        let nymsphinx_packet = FramedSphinxPacket {
            header,
            packet: sphinx_packet,
        };

        // As per docs:
        // Before returning from the function, implementations should ensure that the buffer
        // has appropriate capacity in anticipation of future calls to decode.
        // Failing to do so leads to inefficiency.

        // if we have at least one more byte available, we can reserve enough bytes for
        // the entire next frame, if not, we assume the next frame is an ack packet and
        // reserve for that.
        if !src.is_empty() {
            let next_packet_len = match PacketSize::try_from(src[0]) {
                Ok(next_packet_len) => next_packet_len,
                // the next frame will be malformed but let's leave handling the error to the next
                // call to 'decode', as presumably, the current sphinx packet is still valid
                Err(_) => return Ok(Some(nymsphinx_packet)),
            };
            let next_frame_len = next_packet_len.size() + Header::SIZE;
            src.reserve(next_frame_len - 1);
        } else {
            src.reserve(Header::SIZE + PacketSize::AckPacket.size());
        }

        Ok(Some(nymsphinx_packet))
    }
}

#[cfg(test)]
mod packet_encoding {
    use super::*;
    use nymsphinx_types::builder::SphinxPacketBuilder;
    use nymsphinx_types::{
        crypto, Delay as SphinxDelay, Destination, DestinationAddressBytes, Node, NodeAddressBytes,
        DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, NODE_ADDRESS_LENGTH,
    };

    fn make_valid_sphinx_packet(size: PacketSize) -> SphinxPacket {
        let (_, node1_pk) = crypto::keygen();
        let node1 = Node::new(
            NodeAddressBytes::from_bytes([5u8; NODE_ADDRESS_LENGTH]),
            node1_pk,
        );
        let (_, node2_pk) = crypto::keygen();
        let node2 = Node::new(
            NodeAddressBytes::from_bytes([4u8; NODE_ADDRESS_LENGTH]),
            node2_pk,
        );
        let (_, node3_pk) = crypto::keygen();
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
        SphinxPacketBuilder::new()
            .with_payload_size(size.payload_size())
            .build_packet(b"foomp".to_vec(), &route, &destination, &delays)
            .unwrap()
    }

    #[test]
    fn whole_packet_can_be_decoded_from_a_valid_encoded_instance() {
        let header = Default::default();
        let sphinx_packet = make_valid_sphinx_packet(Default::default());
        let sphinx_bytes = sphinx_packet.to_bytes();

        let packet = FramedSphinxPacket {
            header,
            packet: sphinx_packet,
        };

        let mut bytes = BytesMut::new();
        SphinxCodec.encode(packet, &mut bytes).unwrap();
        let decoded = SphinxCodec.decode(&mut bytes).unwrap().unwrap();

        assert_eq!(decoded.header, header);
        assert_eq!(decoded.packet.to_bytes(), sphinx_bytes)
    }

    #[cfg(test)]
    mod decode_will_allocate_enough_bytes_for_next_call {
        use super::*;

        #[test]
        fn for_empty_bytes() {
            // empty bytes should allocate for header + ack packet
            let mut empty_bytes = BytesMut::new();
            assert!(SphinxCodec.decode(&mut empty_bytes).unwrap().is_none());
            assert_eq!(
                empty_bytes.capacity(),
                Header::SIZE + PacketSize::AckPacket.size()
            );
        }

        #[test]
        fn for_bytes_with_header() {
            // if header gets decoded there should be enough bytes for the entire frame
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
                assert!(SphinxCodec.decode(&mut bytes).unwrap().is_none());

                assert_eq!(bytes.capacity(), Header::SIZE + packet_size.size())
            }
        }

        #[test]
        fn for_full_frame() {
            // if full frame is used exactly, there should be enough space for header + ack packet
            let packet = FramedSphinxPacket {
                header: Header::default(),
                packet: make_valid_sphinx_packet(Default::default()),
            };

            let mut bytes = BytesMut::new();
            SphinxCodec.encode(packet, &mut bytes).unwrap();
            assert!(SphinxCodec.decode(&mut bytes).unwrap().is_some());
            assert_eq!(
                bytes.capacity(),
                Header::SIZE + PacketSize::AckPacket.size()
            );
        }

        #[test]
        fn for_full_frame_with_extra_byte() {
            // if there was at least 1 byte left, there should be enough space for entire next frame
            let packet_sizes = vec![
                PacketSize::AckPacket,
                PacketSize::RegularPacket,
                PacketSize::ExtendedPacket,
            ];

            for packet_size in packet_sizes {
                let first_packet = FramedSphinxPacket {
                    header: Header::default(),
                    packet: make_valid_sphinx_packet(Default::default()),
                };

                let mut bytes = BytesMut::new();
                SphinxCodec.encode(first_packet, &mut bytes).unwrap();
                bytes.put_u8(packet_size as u8);
                assert!(SphinxCodec.decode(&mut bytes).unwrap().is_some());

                assert!(bytes.capacity() >= Header::SIZE + packet_size.size())
            }
        }
    }

    #[test]
    fn can_decode_two_packets_immediately() {
        let packet1 = FramedSphinxPacket {
            header: Header::default(),
            packet: make_valid_sphinx_packet(Default::default()),
        };

        let packet2 = FramedSphinxPacket {
            header: Header::default(),
            packet: make_valid_sphinx_packet(Default::default()),
        };

        let mut bytes = BytesMut::new();

        SphinxCodec.encode(packet1, &mut bytes).unwrap();
        SphinxCodec.encode(packet2, &mut bytes).unwrap();

        assert!(SphinxCodec.decode(&mut bytes).unwrap().is_some());
        assert!(SphinxCodec.decode(&mut bytes).unwrap().is_some());
        assert!(SphinxCodec.decode(&mut bytes).unwrap().is_none());
    }

    #[test]
    fn can_decode_two_packets_in_separate_calls() {
        let packet1 = FramedSphinxPacket {
            header: Header::default(),
            packet: make_valid_sphinx_packet(Default::default()),
        };

        let packet2 = FramedSphinxPacket {
            header: Header::default(),
            packet: make_valid_sphinx_packet(Default::default()),
        };

        let mut bytes = BytesMut::new();
        let mut bytes_tmp = BytesMut::new();

        SphinxCodec.encode(packet1, &mut bytes).unwrap();
        SphinxCodec.encode(packet2, &mut bytes_tmp).unwrap();

        let tmp = bytes_tmp.split_off(100);
        bytes.put(bytes_tmp);

        assert!(SphinxCodec.decode(&mut bytes).unwrap().is_some());
        assert!(SphinxCodec.decode(&mut bytes).unwrap().is_none());

        bytes.put(tmp);

        assert!(SphinxCodec.decode(&mut bytes).unwrap().is_some());
        assert!(SphinxCodec.decode(&mut bytes).unwrap().is_none());
    }
}
