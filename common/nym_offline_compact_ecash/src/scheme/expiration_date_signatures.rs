use crate::scheme::setup::Parameters;
use crate::scheme::keygen::{SecretKeyAuth, VerificationKeyAuth};
use crate::constants;
use crate::utils::hash_g1;
use bls12_381::{Scalar, G1Projective};

#[derive(Debug, PartialEq, Clone)]
pub struct ExpirationDateSignature{
    pub(crate) h : G1Projective,
    pub(crate) s : G1Projective,
}

pub type PartialExpirationDateSignature = ExpirationDateSignature;

// This function is execute by a single issuing authority
pub fn sign_expiration_date(params: &Parameters, sk_auth: SecretKeyAuth, expiration_date: u64) -> Vec<PartialExpirationDateSignature>{

    // Initialize a vector to collect exp_sign values
    let mut exp_signs: Vec<PartialExpirationDateSignature> = Vec::new();

    for l in 0..constants::VALIDITY_PERIOD {
        let m0: u64 = expiration_date;
        let m1: u64 = expiration_date - constants::VALIDITY_PERIOD + l;
        let m2 = constants::TYPE_EXP;

        // Convert u64 values to bytes
        let m0_bytes: [u8; 8] = m0.to_le_bytes();
        let m1_bytes: [u8; 8] = m1.to_le_bytes();
        let mut concatenated_attributes = Vec::new();

        concatenated_attributes.extend_from_slice(&m0_bytes);
        concatenated_attributes.extend_from_slice(&m1_bytes);
        concatenated_attributes.extend_from_slice(&m2);

        // Compute the hash
        let h = hash_g1(concatenated_attributes);

        // Compute s = x + y[0]*m0 + y[1]*m1 + y[2]*m2
        let mut s_exponent = sk_auth.x;

        // Perform scalar-point multiplications and accumulate the result
        s_exponent += &sk_auth.ys[0] * Scalar::from(m0);
        s_exponent += &sk_auth.ys[1] * Scalar::from(m1);
        s_exponent += &sk_auth.ys[2] * Scalar::from_bytes(&m2).unwrap();

        // Create the signature on the expiration date
        let exp_sign = PartialExpirationDateSignature {
            h,
            s: h * s_exponent,
        };

        // Collect the exp_sign value into the vector
        exp_signs.push(exp_sign);
    }

    exp_signs

}

// pub fn aggregate_expiration_signatures(params: &Parameters, vk_auth: &VerificationKeyAuth, expiration_date: u64, indices: Option<&[u64]>, )