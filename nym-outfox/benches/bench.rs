use criterion::{criterion_group, criterion_main, Criterion};
use libcrux_kem::key_gen;
use nym_outfox::{
    format::MixStageParameters,
    packet::OutfoxPacket,
    route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, NODE_ADDRESS_LENGTH},
};
use std::iter::repeat_with;

pub fn randombytes(n: usize) -> Vec<u8> {
    repeat_with(|| fastrand::u8(..)).take(n).collect()
}

fn test_encode_decode(c: &mut Criterion) {
    for kem in [
        libcrux_kem::Algorithm::X25519,
        libcrux_kem::Algorithm::XWingKemDraft06,
        libcrux_kem::Algorithm::MlKem768,
    ] {
        let mix_params = MixStageParameters {
            kem: kem,
            routing_information_length_bytes: 32,
            remaining_header_length_bytes: (32 + 16 + 32) * 4,
            payload_length_bytes: 1024, // 1kb
        };

        let mut rng = rand::rng();
        let (mix_decapsulation_key, mix_encapsulation_key) = key_gen(kem, &mut rng).unwrap();

        let routing = [0; 32];
        let destination = [0; 32];

        let buffer = randombytes(mix_params.incoming_packet_length());

        let mut new_buffer = buffer.clone();

        let node_address_bytes = NodeAddressBytes::from_bytes(routing);

        let node = Node::new(kem, node_address_bytes, mix_encapsulation_key);

        let _ = mix_params
            .encode_mix_layer(&mut rng, &mut new_buffer[..], &node.pub_key, &destination)
            .unwrap();

        assert_ne!(
            new_buffer[mix_params.payload_range()],
            buffer[mix_params.payload_range()]
        );
        assert_ne!(new_buffer[mix_params.routing_data_range()], routing[..]);

        let _ = mix_params
            .decode_mix_layer(&mut new_buffer[..], &mix_decapsulation_key)
            .unwrap();

        assert_eq!(
            new_buffer[mix_params.payload_range()],
            buffer[mix_params.payload_range()]
        );
        assert_eq!(new_buffer[mix_params.routing_data_range()], routing[..]);
    }
}

fn kem_str(kem: libcrux_kem::Algorithm) -> &'static str {
    match kem {
        libcrux_kem::Algorithm::X25519 => "KEM: x25519",
        libcrux_kem::Algorithm::XWingKemDraft06 => "KEM: XWing",
        libcrux_kem::Algorithm::MlKem768 => "KEM: MlKem768",
        _ => unreachable!(),
    }
}

fn test_packet(c: &mut Criterion) {
    let mut rng = rand::rng();
    for kem in [
        libcrux_kem::Algorithm::X25519,
        libcrux_kem::Algorithm::XWingKemDraft06,
        libcrux_kem::Algorithm::MlKem768,
    ] {
        let (node1_pk, node1_pub) = key_gen(kem, &mut rng).unwrap();
        let node1 = Node::new(
            kem,
            NodeAddressBytes::from_bytes([0u8; NODE_ADDRESS_LENGTH]),
            node1_pub,
        );
        let (node2_pk, node2_pub) = key_gen(kem, &mut rng).unwrap();
        let node2 = Node::new(
            kem,
            NodeAddressBytes::from_bytes([1u8; NODE_ADDRESS_LENGTH]),
            node2_pub,
        );
        let (node3_pk, node3_pub) = key_gen(kem, &mut rng).unwrap();
        let node3 = Node::new(
            kem,
            NodeAddressBytes::from_bytes([2u8; NODE_ADDRESS_LENGTH]),
            node3_pub,
        );

        let (gateway_pk, gateway_pub) = key_gen(kem, &mut rng).unwrap();
        let gateway = Node::new(
            kem,
            NodeAddressBytes::from_bytes([3u8; NODE_ADDRESS_LENGTH]),
            gateway_pub,
        );

        c.bench_function(&format!("{} | Key Generation", kem_str(kem)), |b| {
            b.iter(|| key_gen(kem, &mut rng).unwrap())
        });

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([9u8; NODE_ADDRESS_LENGTH]),
            [0u8; 16],
        );

        let route = [node1, node2.clone(), node3.clone(), gateway.clone()];

        for payload_size in [512, 1024, 2048, 4096] {
            c.bench_function(
                &format!(
                    "{} | Packet Construction | Payload: {} bytes",
                    kem_str(kem),
                    payload_size
                ),
                |b| {
                    b.iter_batched(
                        || (rand::rng(), randombytes(payload_size)),
                        |(mut rng, payload)| {
                            OutfoxPacket::build(
                                &mut rng,
                                kem,
                                &payload,
                                &route,
                                &destination,
                                Some(payload.len()),
                            )
                        },
                        criterion::BatchSize::PerIteration,
                    )
                },
            );
            let payload = randombytes(payload_size);

            let packet = OutfoxPacket::build(
                &mut rng,
                kem,
                &payload,
                &route,
                &destination,
                Some(payload.len()),
            )
            .unwrap();
            let packet_bytes = packet.to_bytes().unwrap();

            let mut packet = OutfoxPacket::try_from((kem, packet_bytes.as_slice())).unwrap();

            c.bench_function(
                &format!(
                    "{} | Packet Decoding (Layer 1) | Payload: {} bytes",
                    kem_str(kem),
                    payload_size
                ),
                |b| {
                    b.iter_batched(
                        || OutfoxPacket::try_from((kem, packet_bytes.as_slice())).unwrap(),
                        |mut packet| packet.decode_next_layer(&node1_pk).unwrap(),
                        criterion::BatchSize::PerIteration,
                    )
                },
            );

            c.bench_function(
                &format!(
                    "{} | Packet Decoding (Layer 2) | Payload: {} bytes",
                    kem_str(kem),
                    payload_size
                ),
                |b| {
                    b.iter_batched(
                        || {
                            let mut packet =
                                OutfoxPacket::try_from((kem, packet_bytes.as_slice())).unwrap();
                            packet.decode_next_layer(&node1_pk).unwrap();
                            packet
                        },
                        |mut packet| packet.decode_next_layer(&node2_pk).unwrap(),
                        criterion::BatchSize::PerIteration,
                    )
                },
            );
            c.bench_function(
                &format!(
                    "{} | Packet Decoding (Layer 3) | Payload: {} bytes",
                    kem_str(kem),
                    payload_size
                ),
                |b| {
                    b.iter_batched(
                        || {
                            let mut packet =
                                OutfoxPacket::try_from((kem, packet_bytes.as_slice())).unwrap();
                            packet.decode_next_layer(&node1_pk).unwrap();
                            packet.decode_next_layer(&node2_pk).unwrap();
                            packet
                        },
                        |mut packet| packet.decode_next_layer(&node3_pk).unwrap(),
                        criterion::BatchSize::PerIteration,
                    )
                },
            );
            c.bench_function(
                &format!(
                    "{} | Packet Decoding + Plaintext Recovery (Gateway) | Payload: {} bytes",
                    kem_str(kem),
                    payload_size
                ),
                |b| {
                    b.iter_batched(
                        || {
                            let mut packet =
                                OutfoxPacket::try_from((kem, packet_bytes.as_slice())).unwrap();
                            packet.decode_next_layer(&node1_pk).unwrap();
                            packet.decode_next_layer(&node2_pk).unwrap();
                            packet.decode_next_layer(&node3_pk).unwrap();
                            packet
                        },
                        |mut packet| {
                            packet.decode_next_layer(&gateway_pk).unwrap();
                            packet.recover_plaintext()
                        },
                        criterion::BatchSize::PerIteration,
                    )
                },
            );

            let next_address = packet.decode_next_layer(&node1_pk).unwrap();
            assert_eq!(&next_address, node2.address.as_bytes());
            let next_address = packet.decode_next_layer(&node2_pk).unwrap();
            assert_eq!(&next_address, node3.address.as_bytes());
            let next_address = packet.decode_next_layer(&node3_pk).unwrap();
            assert_eq!(&next_address, gateway.address.as_bytes());
            let destination_address = packet.decode_next_layer(&gateway_pk).unwrap();
            assert_eq!(destination_address, destination.address.as_bytes());

            assert_eq!(payload, packet.recover_plaintext().unwrap());
        }
    }
}

criterion_group!(benches, test_packet);
criterion_main!(benches);
