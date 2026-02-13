use nym_crypto::{blake3, hmac::hmac::digest::ExtendableOutput};

use crate::error::{
    MaskedByteError,
    MaskedByteError::{Failure, InvalidLength},
};

pub const MASKED_BYTE_LEN: usize = 16;
pub const MASKED_BYTE_CONTEXT_STR: &[u8] = b"NYM_MASKED_BYTE_V1";

const U8_RANGE: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
    155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173,
    174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192,
    193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211,
    212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230,
    231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
    250, 251, 252, 253, 254, 255,
];

#[derive(Clone, Copy)]
pub struct MaskedByte([u8; MASKED_BYTE_LEN]);

impl MaskedByte {
    /// Mask a byte by hashing it with some mask.
    /// Outputs Blake3_Hash(MASKED_BYTE_CONTEXT_STR || mask || 0xFF || byte)
    pub fn new(byte: u8, mask: &[u8]) -> Self {
        let mut output: [u8; MASKED_BYTE_LEN] = [0u8; MASKED_BYTE_LEN];
        let mut hasher = blake3::Hasher::new();
        hasher.update(MASKED_BYTE_CONTEXT_STR);
        hasher.update(mask);
        // avoid zero update
        hasher.update(&[0xFF, byte]);
        hasher.finalize_xof_into(&mut output);

        Self(output)
    }
    /// Unmasks a byte by trial hashing.
    /// This function runs Blake3_Hash(MASKED_BYTE_CONTEXT_STR || mask || 0xFF).
    /// This Hasher state is then cloned updated with `i: u8` in (0..=u8::max).
    /// If we find an `i` which yields back the hash input, then we found the masked byte.
    /// Otherwise, the function returns an error.
    pub fn unmask(&self, mask: &[u8]) -> Result<u8, MaskedByteError> {
        self.unmask_check_version(mask, &U8_RANGE)
    }

    // This could be more efficient than unmask,
    // because we just could check against a smaller list of supported versions.
    pub fn unmask_check_version(
        &self,
        mask: &[u8],
        supported_versions: &[u8],
    ) -> Result<u8, MaskedByteError> {
        let mut buf: [u8; MASKED_BYTE_LEN] = [0u8; MASKED_BYTE_LEN];
        let mut hasher = blake3::Hasher::new();
        hasher.update(MASKED_BYTE_CONTEXT_STR);
        hasher.update(mask);
        // avoid zero update
        hasher.update(&[0xFF]);
        for i in supported_versions {
            let mut t_hasher = hasher.clone();
            t_hasher.update(&[*i]);
            t_hasher.finalize_xof_into(&mut buf);
            if buf == self.0 {
                return Ok(*i);
            }
        }
        Err(Failure)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl From<[u8; MASKED_BYTE_LEN]> for MaskedByte {
    fn from(value: [u8; MASKED_BYTE_LEN]) -> Self {
        MaskedByte(value)
    }
}

impl From<&[u8; MASKED_BYTE_LEN]> for MaskedByte {
    fn from(value: &[u8; MASKED_BYTE_LEN]) -> Self {
        MaskedByte(value.to_owned())
    }
}

impl TryFrom<&[u8]> for MaskedByte {
    type Error = MaskedByteError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != MASKED_BYTE_LEN {
            Err(InvalidLength {
                expected: MASKED_BYTE_LEN,
                actual: value.len(),
            })
        } else {
            Ok(Self::from(value.as_chunks::<MASKED_BYTE_LEN>().0[0]))
        }
    }
}

#[cfg(test)]
mod test {

    use crate::masked_byte::MASKED_BYTE_LEN;

    use super::MaskedByte;
    use rand09::{Rng, RngCore, rng};

    #[test]
    fn test_masking() {
        let mut mask: [u8; 256] = [0u8; 256];
        let mut wire_bytes: [u8; MASKED_BYTE_LEN];

        // why not
        for i in 0..=u8::MAX {
            // gen mask
            rng().fill_bytes(&mut mask);
            let masked_byte = MaskedByte::new(i, &mask);
            wire_bytes = masked_byte.to_bytes();

            let decoded_masked_byte = MaskedByte::from(wire_bytes);
            let output = decoded_masked_byte.unmask(&mask).unwrap();

            assert_eq!(i, output);

            // flip bit
            let mut with_flipped_bit = decoded_masked_byte.to_bytes();

            let byte_idx: usize = rng().random_range(0..MASKED_BYTE_LEN);
            let bit_idx = rng().random_range(0..8);
            with_flipped_bit[byte_idx] ^= 1 << bit_idx;

            let decoded_masked_byte = MaskedByte::from(with_flipped_bit);
            assert!(decoded_masked_byte.unmask(&mask).is_err());
        }
    }

    #[test]
    fn test_decoding() {
        let mut mask: [u8; 256] = [0u8; 256];

        // gen mask
        rng().fill_bytes(&mut mask);
        let byte = rng().random();
        let masked_byte = MaskedByte::new(byte, &mask);
        let wire_bytes: [u8; MASKED_BYTE_LEN] = masked_byte.to_bytes();

        // should succeed
        let decoded_masked_byte = MaskedByte::try_from(wire_bytes.as_slice()).unwrap();
        let output = decoded_masked_byte.unmask(&mask).unwrap();

        assert_eq!(byte, output);

        let empty_slice: &[u8] = &[];
        // should fail
        assert!(MaskedByte::try_from(empty_slice).is_err());

        let mut wire_bytes_messy = Vec::from(wire_bytes);

        // add more one more byte
        wire_bytes_messy.push(0x42);
        assert!(wire_bytes_messy.len() == MASKED_BYTE_LEN + 1);
        // should fail
        assert!(MaskedByte::try_from(wire_bytes_messy.as_slice()).is_err());

        // pop the added byte
        _ = wire_bytes_messy.pop();
        assert!(wire_bytes_messy.len() == MASKED_BYTE_LEN);
        // should succeed
        assert!(MaskedByte::try_from(wire_bytes_messy.as_slice()).is_ok());

        // pop one more byte
        _ = wire_bytes_messy.pop();
        assert!(wire_bytes_messy.len() == MASKED_BYTE_LEN - 1);
        // should fail
        assert!(MaskedByte::try_from(wire_bytes_messy.as_slice()).is_err());
    }
}
