extern crate nym_outfox;

#[cfg(test)]
mod tests {

    use std::iter::repeat_with;

    pub fn randombytes(n: usize) -> Vec<u8> {
        repeat_with(|| fastrand::u8(..)).take(n).collect()
    }

    use libcrux_kem::key_gen;
    use nym_outfox::packet::OutfoxPacket;

    use nym_outfox::route::{
        Destination, DestinationAddressBytes, Node, NodeAddressBytes, NODE_ADDRESS_LENGTH,
    };

    use nym_outfox::format::*;
    use nym_outfox::lion::*;

    #[test]
    fn test_encode_decode() {
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

    #[test]
    fn test_lion() {
        let key = randombytes(32);
        let message = randombytes(1024);

        let mut message_clone = message.clone();
        lion_transform(&mut message_clone[..], &key, [1, 2, 3]).unwrap();
        assert_ne!(message_clone[..], message[..]);

        let mut message_clone_2 = message.clone();
        lion_transform_encrypt(&mut message_clone_2, &key).unwrap();
        assert_eq!(message_clone_2, message_clone);

        lion_transform(&mut message_clone[..], &key[..], [3, 2, 1]).unwrap();
        assert_eq!(message_clone[..], message[..]);
    }

    #[test]
    fn test_packet_params_short() {
        let mut rng = rand::rng();
        for kem in [
            libcrux_kem::Algorithm::X25519,
            libcrux_kem::Algorithm::XWingKemDraft06,
            libcrux_kem::Algorithm::MlKem768,
        ] {
            let (entry_pk, entry_pub) = key_gen(kem, &mut rng).unwrap();
            let entry = Node::new(
                kem,
                NodeAddressBytes::from_bytes([8u8; NODE_ADDRESS_LENGTH]),
                entry_pub,
            );
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

            let (exit_pk, exit_pub) = key_gen(kem, &mut rng).unwrap();
            let exit = Node::new(
                kem,
                NodeAddressBytes::from_bytes([3u8; NODE_ADDRESS_LENGTH]),
                exit_pub,
            );

            let destination = Destination::new(
                DestinationAddressBytes::from_bytes([9u8; NODE_ADDRESS_LENGTH]),
                [0u8; 16],
            );

            let route = [
                entry,
                node1.clone(),
                node2.clone(),
                node3.clone(),
                exit.clone(),
            ];

            let payload = vec![0, 0, 1, 1, 1, 0, 0];

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
            println!(
                "packet bytes length, {}, declared {}",
                packet_bytes.len(),
                packet.len()
            );

            let mut packet = OutfoxPacket::try_from((kem, packet_bytes.as_slice())).unwrap();

            let next_address = packet.decode_next_layer(&entry_pk).unwrap();
            assert_eq!(&next_address, node1.address.as_bytes());
            let next_address = packet.decode_next_layer(&node1_pk).unwrap();
            assert_eq!(&next_address, node2.address.as_bytes());
            let next_address = packet.decode_next_layer(&node2_pk).unwrap();
            assert_eq!(&next_address, node3.address.as_bytes());
            let next_address = packet.decode_next_layer(&node3_pk).unwrap();
            assert_eq!(&next_address, exit.address.as_bytes());
            let destination_address = packet.decode_next_layer(&exit_pk).unwrap();
            assert_eq!(destination_address, destination.address.as_bytes());

            assert_eq!(payload, packet.recover_plaintext().unwrap());
        }
    }

    #[test]
    fn test_packet_params_long() {
        let mut rng = rand::rng();
        for kem in [
            libcrux_kem::Algorithm::X25519,
            libcrux_kem::Algorithm::XWingKemDraft06,
            libcrux_kem::Algorithm::MlKem768,
        ] {
            let (entry_pk, entry_pub) = key_gen(kem, &mut rng).unwrap();
            let entry = Node::new(
                kem,
                NodeAddressBytes::from_bytes([8u8; NODE_ADDRESS_LENGTH]),
                entry_pub,
            );
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

            let (exit_pk, exit_pub) = key_gen(kem, &mut rng).unwrap();
            let exit = Node::new(
                kem,
                NodeAddressBytes::from_bytes([3u8; NODE_ADDRESS_LENGTH]),
                exit_pub,
            );

            let destination = Destination::new(
                DestinationAddressBytes::from_bytes([9u8; NODE_ADDRESS_LENGTH]),
                [0u8; 16],
            );

            let route = [
                entry,
                node1.clone(),
                node2.clone(),
                node3.clone(),
                exit.clone(),
            ];

            let payload = randombytes(2048);

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
            println!(
                "packet bytes length, {}, declared {}",
                packet_bytes.len(),
                packet.len()
            );

            let mut packet = OutfoxPacket::try_from((kem, packet_bytes.as_slice())).unwrap();

            let next_address = packet.decode_next_layer(&entry_pk).unwrap();
            assert_eq!(&next_address, node1.address.as_bytes());
            let next_address = packet.decode_next_layer(&node1_pk).unwrap();
            assert_eq!(&next_address, node2.address.as_bytes());
            let next_address = packet.decode_next_layer(&node2_pk).unwrap();
            assert_eq!(&next_address, node3.address.as_bytes());
            let next_address = packet.decode_next_layer(&node3_pk).unwrap();
            assert_eq!(&next_address, exit.address.as_bytes());
            let destination_address = packet.decode_next_layer(&exit_pk).unwrap();
            assert_eq!(destination_address, destination.address.as_bytes());

            assert_eq!(payload, packet.recover_plaintext().unwrap());
        }
    }
}
