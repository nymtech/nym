extern crate nym_outfox;

#[cfg(test)]
mod tests {

    use curve25519_dalek::constants::ED25519_BASEPOINT_TABLE;
    use curve25519_dalek::scalar::Scalar;
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

        let user_secret_bytes = randombytes(32);
        let mix_secret_bytes = randombytes(32);

        let user_secret = Scalar::from_bytes_mod_order(user_secret_bytes.try_into().unwrap());
        let mix_secret = Scalar::from_bytes_mod_order(mix_secret_bytes.try_into().unwrap());
        let mix_public_key = (&ED25519_BASEPOINT_TABLE * &mix_secret).to_montgomery();

        let routing = [0; 32];

        let buffer = randombytes(mix_params.incoming_packet_length());

        let mut new_buffer = buffer.clone();

        let _ = mix_params
            .encode_mix_layer(
                &mut new_buffer[..],
                &user_secret,
                &mix_public_key,
                &routing[..],
            )
            .unwrap();

        assert!(&new_buffer[mix_params.payload_range()] != &buffer[mix_params.payload_range()]);
        assert!(&new_buffer[mix_params.routing_data_range()] != &routing[..]);

        let _ = mix_params
            .decode_mix_layer(&mut new_buffer[..], &mix_secret)
            .unwrap();

        assert!(&new_buffer[mix_params.payload_range()] == &buffer[mix_params.payload_range()]);
        assert!(&new_buffer[mix_params.routing_data_range()] == &routing[..]);
    }

    #[test]
    fn test_lion() {
        let key = randombytes(32);
        let message = randombytes(1024);

        let mut message_clone = message.clone();
        lion_transform(&mut message_clone[..], &key, [1, 2, 3]).unwrap();
        assert!(&message_clone[..] != &message[..]);

        let mut message_clone_2 = message.clone();
        lion_transform_encrypt(&mut message_clone_2, &key).unwrap();
        assert_eq!(message_clone_2, message_clone);

        lion_transform(&mut message_clone[..], &key[..], [3, 2, 1]).unwrap();
        assert!(&message_clone[..] == &message[..]);
    }

    #[test]
    fn test_packet_params() {
        // Dummy keys -- we will use the same key for each layer
        let user_secret_bytes = randombytes(32);
        let mix_secret_bytes = randombytes(32);

        let user_secret = Scalar::from_bytes_mod_order(user_secret_bytes.try_into().unwrap());
        let mix_secret = Scalar::from_bytes_mod_order(mix_secret_bytes.try_into().unwrap());
        let mix_public_key = (&ED25519_BASEPOINT_TABLE * &mix_secret).to_montgomery();

        let routing = [0; 32];

        let mut params = MixCreationParameters::new(1025);
        params.add_outer_layer(32);
        params.add_outer_layer(32);
        params.add_outer_layer(32);

        let mut buf = vec![0; params.total_packet_length()];

        let (range0, layer_params0) = params.get_stage_params(0);
        let _ = layer_params0
            .encode_mix_layer(
                &mut buf[range0.clone()],
                &user_secret,
                &mix_public_key,
                &routing[..],
            )
            .unwrap();

        let (range1, layer_params1) = params.get_stage_params(1);
        let _ = layer_params1
            .encode_mix_layer(
                &mut buf[range1.clone()],
                &user_secret,
                &mix_public_key,
                &routing[..],
            )
            .unwrap();

        let (range2, layer_params2) = params.get_stage_params(2);
        let _ = layer_params2
            .encode_mix_layer(
                &mut buf[range2.clone()],
                &user_secret,
                &mix_public_key,
                &routing[..],
            )
            .unwrap();

        assert!(
            &buf[params.total_packet_length() - 1025..params.total_packet_length()] != [0; 1025]
        );

        let _ = layer_params2
            .decode_mix_layer(&mut buf[range2], &mix_secret)
            .unwrap();

        let _ = layer_params1
            .decode_mix_layer(&mut buf[range1], &mix_secret)
            .unwrap();

        let _ = layer_params0
            .decode_mix_layer(&mut buf[range0], &mix_secret)
            .unwrap();
    }
}
