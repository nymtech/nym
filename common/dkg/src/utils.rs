// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::CHUNK_SIZE;
use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve, HashToField};
use bls12_381::G1Projective;
use bls12_381::{G2Projective, Scalar};
use group::GroupEncoding;
use sha2::{Digest, Sha256};

#[macro_export]
macro_rules! ensure_len {
    ($a:expr, $b:expr) => {
        if $a.len() != $b {
            return false;
        }
    };
}

pub(crate) struct RandomOracleBuilder {
    inner_state: Sha256,
}

impl RandomOracleBuilder {
    pub(crate) fn new(domain: &[u8]) -> Self {
        let mut inner_state = Sha256::new();
        inner_state.update(domain);

        RandomOracleBuilder { inner_state }
    }

    pub(crate) fn update(&mut self, data: impl AsRef<[u8]>) {
        self.inner_state.update(data)
    }

    pub(crate) fn update_with_g1_elements<'a, I>(&mut self, items: I)
    where
        I: Iterator<Item = &'a G1Projective>,
    {
        items.for_each(|item| self.update(item.to_bytes()))
    }

    pub(crate) fn finalize(self) -> [u8; 32] {
        self.inner_state.finalize().into()
    }
}

// those will most likely need to somehow get re-combined with coconut (or maybe extracted to a completely different module)
pub(crate) fn hash_to_scalar<M: AsRef<[u8]>>(msg: M, domain: &[u8]) -> Scalar {
    // the unwrap here is fine as the result vector will have 1 element (as specified) and will not be empty
    hash_to_scalars(msg, domain, 1).pop().unwrap()
}

pub(crate) fn hash_to_scalars<M: AsRef<[u8]>>(msg: M, domain: &[u8], n: usize) -> Vec<Scalar> {
    let mut output = vec![Scalar::zero(); n];

    Scalar::hash_to_field::<ExpandMsgXmd<Sha256>, _>([msg], domain, &mut output);
    output
}

pub(crate) fn hash_g2<M: AsRef<[u8]>>(msg: M, domain: &[u8]) -> G2Projective {
    <G2Projective as HashToCurve<ExpandMsgXmd<Sha256>>>::hash_to_curve([msg], domain)
}

pub(crate) fn combine_scalar_chunks(chunks: &[Scalar]) -> Scalar {
    let chunk_size_scalar = Scalar::from(CHUNK_SIZE as u64);
    chunks.iter().rev().fold(Scalar::zero(), |mut acc, chunk| {
        acc *= chunk_size_scalar;
        acc += chunk;
        acc
    })
}

pub(crate) fn combine_g1_chunks(chunks: &[G1Projective]) -> G1Projective {
    let chunk_size_scalar = Scalar::from(CHUNK_SIZE as u64);
    chunks
        .iter()
        .rev()
        .fold(G1Projective::identity(), |mut acc, chunk| {
            acc *= chunk_size_scalar;
            acc += chunk;
            acc
        })
}

pub(crate) fn deserialize_scalar(b: &[u8]) -> Option<Scalar> {
    if b.len() != 32 {
        None
    } else {
        let mut repr: [u8; 32] = Default::default();
        repr.as_mut().copy_from_slice(b);
        Scalar::from_bytes(&repr).into()
    }
}

pub(crate) fn deserialize_g1(b: &[u8]) -> Option<G1Projective> {
    if b.len() != 48 {
        None
    } else {
        let mut encoding = <G1Projective as GroupEncoding>::Repr::default();
        encoding.as_mut().copy_from_slice(b);
        G1Projective::from_bytes(&encoding).into()
    }
}

pub(crate) fn deserialize_g2(b: &[u8]) -> Option<G2Projective> {
    if b.len() != 96 {
        None
    } else {
        let mut encoding = <G2Projective as GroupEncoding>::Repr::default();
        encoding.as_mut().copy_from_slice(b);
        G2Projective::from_bytes(&encoding).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bls12_381::G2Affine;

    #[test]
    fn test_hash_to_scalar() {
        let msg1 = "foo";
        let expected1 = Scalar::from_bytes(&[
            253, 57, 224, 227, 175, 195, 226, 82, 46, 175, 33, 126, 171, 239, 255, 92, 108, 168, 6,
            79, 90, 11, 235, 236, 221, 10, 85, 133, 42, 81, 95, 30,
        ])
        .unwrap();

        let msg2 = "bar";
        let expected2 = Scalar::from_bytes(&[
            48, 83, 69, 52, 42, 18, 135, 244, 211, 190, 160, 196, 118, 154, 24, 126, 0, 125, 72,
            201, 170, 225, 123, 201, 52, 120, 171, 132, 235, 182, 20, 26,
        ])
        .unwrap();

        let msg3 = [
            33, 135, 76, 234, 71, 35, 247, 216, 39, 242, 42, 88, 152, 29, 74, 135, 9, 29, 216, 123,
            250, 87, 108, 29, 245, 126, 109, 102, 84, 71, 158, 224, 145, 243, 49, 121, 244, 27,
            115, 121, 25, 66, 216, 67, 97, 101, 140, 160, 77, 239, 114, 215, 152, 48, 15, 231, 101,
            60, 42, 92, 128, 131, 161, 43,
        ];
        let expected3 = Scalar::from_bytes(&[
            128, 189, 8, 43, 186, 55, 52, 61, 171, 196, 159, 177, 162, 100, 27, 143, 85, 83, 218,
            171, 91, 220, 155, 25, 7, 38, 2, 36, 4, 93, 136, 4,
        ])
        .unwrap();

        assert_eq!(
            hash_to_scalar(msg1, b"NYMECASH-V01-CS02-with-expander-SHA256"),
            expected1
        );
        assert_eq!(
            hash_to_scalar(msg2, b"NYMECASH-V01-CS02-with-expander-SHA256"),
            expected2
        );
        assert_eq!(
            hash_to_scalar(msg3, b"NYMECASH-V01-CS02-with-expander-SHA256"),
            expected3
        );
    }

    #[test]
    fn test_hash_g2() {
        let msg1 = "foo";
        let expected1 = G2Affine::from_compressed(&[
            175, 187, 62, 7, 29, 17, 42, 93, 28, 93, 234, 253, 101, 166, 158, 187, 153, 82, 93, 18,
            11, 233, 36, 107, 51, 117, 30, 127, 32, 254, 210, 77, 133, 12, 253, 255, 84, 128, 36,
            214, 234, 103, 50, 21, 26, 78, 112, 49, 20, 69, 19, 109, 7, 78, 33, 227, 196, 180, 168,
            219, 73, 251, 192, 221, 41, 138, 160, 131, 191, 186, 156, 117, 179, 179, 191, 235, 171,
            26, 219, 148, 170, 179, 11, 38, 137, 14, 95, 115, 171, 186, 163, 82, 158, 6, 239, 88,
        ])
        .unwrap()
        .into();

        let msg2 = "bar";
        let expected2 = G2Affine::from_compressed(&[
            183, 25, 90, 187, 34, 184, 30, 182, 215, 242, 158, 83, 116, 34, 210, 96, 188, 79, 83,
            255, 100, 122, 90, 188, 196, 93, 164, 253, 20, 106, 205, 33, 48, 140, 60, 149, 66, 246,
            121, 244, 146, 66, 170, 60, 113, 95, 102, 237, 25, 231, 8, 42, 121, 124, 180, 140, 34,
            104, 173, 251, 89, 189, 28, 196, 49, 66, 101, 38, 68, 44, 40, 235, 21, 35, 204, 123,
            218, 238, 216, 92, 134, 217, 212, 246, 176, 77, 187, 0, 245, 134, 132, 73, 31, 44, 137,
            197,
        ])
        .unwrap()
        .into();
        let msg3 = [
            33, 135, 76, 234, 71, 35, 247, 216, 39, 242, 42, 88, 152, 29, 74, 135, 9, 29, 216, 123,
            250, 87, 108, 29, 245, 126, 109, 102, 84, 71, 158, 224, 145, 243, 49, 121, 244, 27,
            115, 121, 25, 66, 216, 67, 97, 101, 140, 160, 77, 239, 114, 215, 152, 48, 15, 231, 101,
            60, 42, 92, 128, 131, 161, 43,
        ];
        let expected3 = G2Affine::from_compressed(&[
            151, 185, 8, 123, 223, 150, 192, 192, 115, 10, 3, 129, 49, 179, 31, 108, 0, 17, 46,
            231, 184, 164, 247, 228, 22, 142, 87, 70, 120, 111, 154, 15, 245, 110, 32, 84, 53, 117,
            239, 93, 89, 119, 32, 17, 39, 250, 198, 137, 6, 95, 137, 202, 54, 244, 238, 190, 11,
            217, 237, 95, 72, 59, 140, 56, 3, 42, 61, 195, 192, 101, 46, 204, 207, 75, 70, 176,
            207, 48, 24, 195, 248, 234, 178, 168, 54, 109, 19, 189, 51, 52, 120, 69, 248, 226, 102,
            91,
        ])
        .unwrap()
        .into();

        assert_eq!(hash_g2(msg1, b"DUMMY_TEST_DOMAIN"), expected1);
        assert_eq!(hash_g2(msg2, b"DUMMY_TEST_DOMAIN"), expected2);
        assert_eq!(hash_g2(msg3, b"DUMMY_TEST_DOMAIN"), expected3);
    }
}
