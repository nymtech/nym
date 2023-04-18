extern crate nym_outfox;

#[cfg(test)]
mod tests {

    use curve25519_dalek::constants::ED25519_BASEPOINT_TABLE;
    use curve25519_dalek::scalar::Scalar;
    use nym_outfox::packet::OutfoxPacket;
    use sphinx_packet::constants::NODE_ADDRESS_LENGTH;
    use sphinx_packet::crypto::PublicKey;
    use sphinx_packet::packet::builder::DEFAULT_PAYLOAD_SIZE;
    use sphinx_packet::route::Node;
    use sphinx_packet::route::NodeAddressBytes;
    use std::convert::TryInto;

    use nym_outfox::format::*;
    use nym_outfox::lion::*;

    use std::iter::repeat_with;

    pub fn randombytes(n: usize) -> Vec<u8> {
        repeat_with(|| fastrand::u8(..)).take(n).collect()
    }

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
        let mix_public_key = (&ED25519_BASEPOINT_TABLE * &mix_secret_scalar).to_montgomery();

        let routing = [0; 32];

        let buffer = randombytes(mix_params.incoming_packet_length());

        let mut new_buffer = buffer.clone();

        let node_address_bytes = NodeAddressBytes::from_bytes(routing);
        let mix_public_key = PublicKey::from(*mix_public_key.as_bytes());

        let node = Node::new(node_address_bytes, mix_public_key);

        let _ = mix_params
            .encode_mix_layer(&mut new_buffer[..], &user_secret, &node)
            .unwrap();

        assert!(new_buffer[mix_params.payload_range()] != buffer[mix_params.payload_range()]);
        assert!(new_buffer[mix_params.routing_data_range()] != routing[..]);

        let _ = mix_params
            .decode_mix_layer(&mut new_buffer[..], &mix_secret)
            .unwrap();

        assert!(new_buffer[mix_params.payload_range()] == buffer[mix_params.payload_range()]);
        assert!(new_buffer[mix_params.routing_data_range()] == routing[..]);
    }

    #[test]
    fn test_lion() {
        let key = randombytes(32);
        let message = randombytes(1024);

        let mut message_clone = message.clone();
        lion_transform(&mut message_clone[..], &key, [1, 2, 3]).unwrap();
        assert!(message_clone[..] != message[..]);

        let mut message_clone_2 = message.clone();
        lion_transform_encrypt(&mut message_clone_2, &key).unwrap();
        assert_eq!(message_clone_2, message_clone);

        lion_transform(&mut message_clone[..], &key[..], [3, 2, 1]).unwrap();
        assert!(message_clone[..] == message[..]);
    }

    #[test]
    fn test_packet_params() {
        let user_secret = randombytes(32);

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

        let route = [node1, node2, node3];

        let payload = randombytes(DEFAULT_PAYLOAD_SIZE);

        let mut packet = OutfoxPacket::build(&payload, &route, &user_secret).unwrap();

        packet.decode_mix_layer(2, &node1_pk.to_bytes()).unwrap();
        packet.decode_mix_layer(1, &node2_pk.to_bytes()).unwrap();
        packet.decode_mix_layer(0, &node3_pk.to_bytes()).unwrap();

        assert_eq!(payload, &packet.payload()[packet.payload_range()]);
    }
}
