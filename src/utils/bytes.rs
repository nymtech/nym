pub fn zero_pad_to_32(mut bytes: Vec<u8>) -> [u8; 32] {
    assert!(bytes.len() <= 32);
    if bytes.len() != 32 {
        bytes.resize(32, 0);
    }
    let mut padded_bytes = [0; 32];
    padded_bytes.copy_from_slice(&bytes[..]);
    assert!(padded_bytes.len() == 32);
    padded_bytes
}

#[cfg(test)]
mod zero_padding_to_32_bytes {
    use super::*;

    #[cfg(test)]
    mod with_empty_input {
        use super::*;

        #[test]
        fn it_returns_32_zeros() {
            let input = vec![];
            let result = zero_pad_to_32(input);
            assert_eq!([0u8; 32], result);
        }
    }

    #[cfg(test)]
    mod with_all_bytes_set_to_1 {
        use super::*;
        #[test]
        fn it_returns_32_ones() {
            let input = vec![1u8; 32];
            let result = zero_pad_to_32(input);
            assert_eq!([1u8; 32], result);
        }
    }

    #[cfg(test)]
    mod with_3_bytes_set {
        use super::*;
        #[test]
        fn it_returns_input_zero_padded_to_32_bytes() {
            let input = vec![1u8; 3];
            let result = zero_pad_to_32(input);
            let expected_content = vec![
                1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ];
            assert_eq!(expected_content, result.to_vec());
        }
    }

    #[cfg(test)]
    mod with_oversized_input {
        use super::*;
        #[test]
        #[should_panic]
        fn it_panics() {
            let input = vec![1u8; 33];
            zero_pad_to_32(input);
        }
    }
}
