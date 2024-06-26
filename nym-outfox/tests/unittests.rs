extern crate nym_outfox;

#[cfg(test)]
mod tests {

    use std::iter::repeat_with;

    pub fn randombytes(n: usize) -> Vec<u8> {
        repeat_with(|| fastrand::u8(..)).take(n).collect()
    }

    use curve25519_dalek::constants::ED25519_BASEPOINT_TABLE;
    use curve25519_dalek::scalar::Scalar;
    use nym_outfox::packet::OutfoxPacket;
    use sphinx_packet::constants::NODE_ADDRESS_LENGTH;
    use sphinx_packet::crypto::PublicKey;
    use sphinx_packet::route::Destination;
    use sphinx_packet::route::DestinationAddressBytes;
    use sphinx_packet::route::Node;
    use sphinx_packet::route::NodeAddressBytes;

    use nym_outfox::format::*;
    use nym_outfox::lion::*;

    #[test]
    fn test_encode_decode() {
        let mix_params = MixStageParameters {
            routing_information_length_bytes: 32,
            remaining_header_length_bytes: (32 + 16 + 32) * 4,
            payload_length_bytes: 1024, // 1kb
        };

        let user_secret = randombytes(32);
        let mix_secret = randombytes(32);
        let mix_secret_scalar =
            Scalar::from_bytes_mod_order(mix_secret.clone().try_into().unwrap());
        let mix_public_key = (ED25519_BASEPOINT_TABLE * &mix_secret_scalar).to_montgomery();

        let routing = [0; 32];
        let destination = [0; 32];

        let buffer = randombytes(mix_params.incoming_packet_length());

        let mut new_buffer = buffer.clone();

        let node_address_bytes = NodeAddressBytes::from_bytes(routing);
        let mix_public_key = PublicKey::from(*mix_public_key.as_bytes());

        let node = Node::new(node_address_bytes, mix_public_key);

        let _ = mix_params
            .encode_mix_layer(
                &mut new_buffer[..],
                &user_secret,
                node.pub_key.as_bytes(),
                &destination,
            )
            .unwrap();

        assert_ne!(
            new_buffer[mix_params.payload_range()],
            buffer[mix_params.payload_range()]
        );
        assert_ne!(new_buffer[mix_params.routing_data_range()], routing[..]);

        let _ = mix_params
            .decode_mix_layer(&mut new_buffer[..], &mix_secret)
            .unwrap();

        assert_eq!(
            new_buffer[mix_params.payload_range()],
            buffer[mix_params.payload_range()]
        );
        assert_eq!(new_buffer[mix_params.routing_data_range()], routing[..]);
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
        let (node1_pk, node1_pub) = sphinx_packet::crypto::keygen();
        let node1 = Node::new(
            NodeAddressBytes::from_bytes([0u8; NODE_ADDRESS_LENGTH]),
            node1_pub,
        );
        let (node2_pk, node2_pub) = sphinx_packet::crypto::keygen();
        let node2 = Node::new(
            NodeAddressBytes::from_bytes([1u8; NODE_ADDRESS_LENGTH]),
            node2_pub,
        );
        let (node3_pk, node3_pub) = sphinx_packet::crypto::keygen();
        let node3 = Node::new(
            NodeAddressBytes::from_bytes([2u8; NODE_ADDRESS_LENGTH]),
            node3_pub,
        );

        let (gateway_pk, gateway_pub) = sphinx_packet::crypto::keygen();
        let gateway = Node::new(
            NodeAddressBytes::from_bytes([3u8; NODE_ADDRESS_LENGTH]),
            gateway_pub,
        );

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([9u8; NODE_ADDRESS_LENGTH]),
            [0u8; 16],
        );

        let route = [node1, node2.clone(), node3.clone(), gateway.clone()];

        let payload = vec![0, 0, 1, 1, 1, 0, 0];

        let packet =
            OutfoxPacket::build(&payload, &route, &destination, Some(payload.len())).unwrap();
        let packet_bytes = packet.to_bytes().unwrap();
        println!(
            "packet bytes length, {}, declared {}",
            packet_bytes.len(),
            packet.len()
        );

        let mut packet = OutfoxPacket::try_from(packet_bytes.as_slice()).unwrap();

        let next_address = packet.decode_next_layer(&node1_pk).unwrap();
        assert_eq!(next_address, node2.address.as_bytes());
        let next_address = packet.decode_next_layer(&node2_pk).unwrap();
        assert_eq!(next_address, node3.address.as_bytes());
        let next_address = packet.decode_next_layer(&node3_pk).unwrap();
        assert_eq!(next_address, gateway.address.as_bytes());
        let destination_address = packet.decode_next_layer(&gateway_pk).unwrap();
        assert_eq!(destination_address, destination.address.as_bytes());

        assert_eq!(payload, packet.recover_plaintext().unwrap());
    }

    #[test]
    fn test_packet_params_long() {
        let (node1_pk, node1_pub) = sphinx_packet::crypto::keygen();
        let node1 = Node::new(
            NodeAddressBytes::from_bytes([0u8; NODE_ADDRESS_LENGTH]),
            node1_pub,
        );
        let (node2_pk, node2_pub) = sphinx_packet::crypto::keygen();
        let node2 = Node::new(
            NodeAddressBytes::from_bytes([1u8; NODE_ADDRESS_LENGTH]),
            node2_pub,
        );
        let (node3_pk, node3_pub) = sphinx_packet::crypto::keygen();
        let node3 = Node::new(
            NodeAddressBytes::from_bytes([2u8; NODE_ADDRESS_LENGTH]),
            node3_pub,
        );

        let (gateway_pk, gateway_pub) = sphinx_packet::crypto::keygen();
        let gateway = Node::new(
            NodeAddressBytes::from_bytes([3u8; NODE_ADDRESS_LENGTH]),
            gateway_pub,
        );

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([9u8; NODE_ADDRESS_LENGTH]),
            [0u8; 16],
        );

        let route = [node1, node2.clone(), node3.clone(), gateway.clone()];

        let payload = randombytes(2048);

        let packet =
            OutfoxPacket::build(&payload, &route, &destination, Some(payload.len())).unwrap();
        let packet_bytes = packet.to_bytes().unwrap();
        println!(
            "packet bytes length, {}, declared {}",
            packet_bytes.len(),
            packet.len()
        );

        let mut packet = OutfoxPacket::try_from(packet_bytes.as_slice()).unwrap();

        let next_address = packet.decode_next_layer(&node1_pk).unwrap();
        assert_eq!(next_address, node2.address.as_bytes());
        let next_address = packet.decode_next_layer(&node2_pk).unwrap();
        assert_eq!(next_address, node3.address.as_bytes());
        let next_address = packet.decode_next_layer(&node3_pk).unwrap();
        assert_eq!(next_address, gateway.address.as_bytes());
        let destination_address = packet.decode_next_layer(&gateway_pk).unwrap();
        assert_eq!(destination_address, destination.address.as_bytes());

        assert_eq!(payload, packet.recover_plaintext().unwrap());
    }
}
