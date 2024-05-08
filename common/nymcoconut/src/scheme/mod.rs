// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: implement https://crates.io/crates/signature traits?

use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::Curve;

pub use keygen::{SecretKey, VerificationKey};

use crate::error::{CoconutError, Result};
use crate::scheme::setup::Parameters;
use crate::scheme::verification::check_bilinear_pairing;
use crate::traits::{Base58, Bytable};
use crate::utils::try_deserialize_g1_projective;
use crate::Attribute;

pub mod aggregation;
pub mod double_use;
pub mod issuance;
pub mod keygen;
pub mod setup;
pub mod verification;

pub type SignerIndex = u64;

// (h, s)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub(crate) G1Projective, pub(crate) G1Projective);

pub type PartialSignature = Signature;

impl TryFrom<&[u8]> for Signature {
    type Error = CoconutError;

    fn try_from(bytes: &[u8]) -> Result<Signature> {
        if bytes.len() != 96 {
            return Err(CoconutError::Deserialization(format!(
                "Signature must be exactly 96 bytes, got {}",
                bytes.len()
            )));
        }

        // safety: we just checked for the length so the unwraps are fine
        #[allow(clippy::expect_used)]
        let sig1_bytes: &[u8; 48] = &bytes[..48].try_into().expect("Slice size != 48");
        #[allow(clippy::expect_used)]
        let sig2_bytes: &[u8; 48] = &bytes[48..].try_into().expect("Slice size != 48");

        let sig1 = try_deserialize_g1_projective(
            sig1_bytes,
            CoconutError::Deserialization("Failed to deserialize compressed sig1".to_string()),
        )?;

        let sig2 = try_deserialize_g1_projective(
            sig2_bytes,
            CoconutError::Deserialization("Failed to deserialize compressed sig2".to_string()),
        )?;

        Ok(Signature(sig1, sig2))
    }
}

impl Signature {
    pub(crate) fn sig1(&self) -> &G1Projective {
        &self.0
    }

    pub(crate) fn sig2(&self) -> &G1Projective {
        &self.1
    }

    pub fn randomise_simple(&self, params: &Parameters) -> Signature {
        let r = params.random_scalar();
        Signature(self.0 * r, self.1 * r)
    }

    pub fn randomise(&self, params: &Parameters) -> (Signature, Scalar) {
        let r = params.random_scalar();
        let r_prime = params.random_scalar();
        let h_prime = self.0 * r_prime;
        let s_prime = (self.1 * r_prime) + (h_prime * r);
        (Signature(h_prime, s_prime), r)
    }

    pub fn to_bytes(self) -> [u8; 96] {
        let mut bytes = [0u8; 96];
        bytes[..48].copy_from_slice(&self.0.to_affine().to_compressed());
        bytes[48..].copy_from_slice(&self.1.to_affine().to_compressed());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Signature> {
        Signature::try_from(bytes)
    }

    pub fn verify(
        &self,
        params: &Parameters,
        partial_verification_key: &VerificationKey,
        private_attributes: &[&Attribute],
        public_attributes: &[&Attribute],
        commitment_hash: &G1Projective,
    ) -> Result<()> {
        // Verify the commitment hash
        if !(commitment_hash == &self.0) {
            return Err(CoconutError::Verification(
                "Verification of commitment hash from signature failed".to_string(),
            ));
        }

        let alpha = partial_verification_key.alpha;

        let signed_attributes = private_attributes
            .iter()
            .chain(public_attributes.iter())
            .zip(partial_verification_key.beta_g2.iter())
            .map(|(&attr, beta_i)| beta_i * attr)
            .sum::<G2Projective>();

        // Verify the signature share
        if !check_bilinear_pairing(
            &self.0.to_affine(),
            &G2Prepared::from((alpha + signed_attributes).to_affine()),
            &self.1.to_affine(),
            params.prepared_miller_g2(),
        ) {
            return Err(CoconutError::Unblind(
                "Verification of signature share failed".to_string(),
            ));
        }

        Ok(())
    }
}

impl Bytable for Signature {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        Signature::from_bytes(slice)
    }
}

impl Base58 for Signature {}

#[derive(Debug, PartialEq, Eq)]
pub struct BlindedSignature(G1Projective, G1Projective);

impl Bytable for BlindedSignature {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        Self::from_bytes(slice)
    }
}

impl Base58 for BlindedSignature {}

impl TryFrom<&[u8]> for BlindedSignature {
    type Error = CoconutError;

    fn try_from(bytes: &[u8]) -> Result<BlindedSignature> {
        if bytes.len() != 96 {
            return Err(CoconutError::Deserialization(format!(
                "BlindedSignature must be exactly 96 bytes, got {}",
                bytes.len()
            )));
        }

        // safety: we just checked for the length so the unwraps are fine
        #[allow(clippy::expect_used)]
        let h_bytes: &[u8; 48] = &bytes[..48].try_into().expect("Slice size != 48");
        #[allow(clippy::expect_used)]
        let sig_bytes: &[u8; 48] = &bytes[48..].try_into().expect("Slice size != 48");

        let h = try_deserialize_g1_projective(
            h_bytes,
            CoconutError::Deserialization("Failed to deserialize compressed h".to_string()),
        )?;
        let sig = try_deserialize_g1_projective(
            sig_bytes,
            CoconutError::Deserialization("Failed to deserialize compressed sig".to_string()),
        )?;

        Ok(BlindedSignature(h, sig))
    }
}

impl BlindedSignature {
    pub fn unblind(
        &self,
        partial_verification_key: &VerificationKey,
        pedersen_commitments_openings: &[Scalar],
    ) -> Signature {
        // parse the signature
        let h = &self.0;
        let c = &self.1;
        let blinding_removers = partial_verification_key
            .beta_g1
            .iter()
            .zip(pedersen_commitments_openings.iter())
            .map(|(beta, opening)| beta * opening)
            .sum::<G1Projective>();

        let unblinded_c = c - blinding_removers;

        Signature(*h, unblinded_c)
    }

    pub fn unblind_and_verify(
        &self,
        params: &Parameters,
        partial_verification_key: &VerificationKey,
        private_attributes: &[&Attribute],
        public_attributes: &[&Attribute],
        commitment_hash: &G1Projective,
        pedersen_commitments_openings: &[Scalar],
    ) -> Result<Signature> {
        let unblinded = self.unblind(partial_verification_key, pedersen_commitments_openings);
        unblinded.verify(
            params,
            partial_verification_key,
            private_attributes,
            public_attributes,
            commitment_hash,
        )?;
        Ok(unblinded)
    }

    pub fn to_bytes(&self) -> [u8; 96] {
        let mut bytes = [0u8; 96];
        bytes[..48].copy_from_slice(&self.0.to_affine().to_compressed());
        bytes[48..].copy_from_slice(&self.1.to_affine().to_compressed());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<BlindedSignature> {
        BlindedSignature::try_from(bytes)
    }
}

// perhaps this should take signature by reference? we'll see how it goes
#[derive(Clone, Copy)]
pub struct SignatureShare {
    signature: Signature,
    index: SignerIndex,
}

impl From<(Signature, SignerIndex)> for SignatureShare {
    fn from(value: (Signature, SignerIndex)) -> Self {
        SignatureShare {
            signature: value.0,
            index: value.1,
        }
    }
}

impl SignatureShare {
    pub fn new(signature: Signature, index: SignerIndex) -> Self {
        SignatureShare { signature, index }
    }

    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn index(&self) -> SignerIndex {
        self.index
    }

    // pub fn aggregate(shares: &[Self]) -> Result<Signature> {
    //     aggregate_signature_shares(shares)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash_to_scalar;
    use crate::scheme::aggregation::{
        aggregate_signatures_and_verify, aggregate_verification_keys,
    };
    use crate::scheme::issuance::{blind_sign, compute_hash, prepare_blind_sign, sign};
    use crate::scheme::keygen::{keygen, ttp_keygen};
    use crate::scheme::verification::{prove_bandwidth_credential, verify, verify_credential};
    use crate::tests::helpers::random_scalars_refs;

    #[test]
    fn unblind_returns_error_if_integrity_check_on_commitment_hash_fails() {
        let params = Parameters::new(2).unwrap();
        random_scalars_refs!(private_attributes, params, 2);

        let (_commitments_openings, lambda) =
            prepare_blind_sign(&params, &private_attributes, &[]).unwrap();

        let keypair1 = keygen(&params);

        let sig1 = blind_sign(&params, keypair1.secret_key(), &lambda, &[]).unwrap();

        let wrong_commitment_opening = params.random_scalar();
        let wrong_commitment = params.gen1() * wrong_commitment_opening;
        let fake_commitment_hash = compute_hash(wrong_commitment, &[]);
        let wrong_commitments_openings = params.n_random_scalars(private_attributes.len());

        assert!(sig1
            .unblind_and_verify(
                &params,
                keypair1.verification_key(),
                &private_attributes,
                &[],
                &fake_commitment_hash,
                &wrong_commitments_openings,
            )
            .is_err());
    }

    #[test]
    fn unblind_returns_error_if_signature_verification_fails() {
        let params = Parameters::new(2).unwrap();
        let p = [hash_to_scalar("Attribute1"), hash_to_scalar("Attribute2")];
        let private_attributes = vec![&p[0], &p[1]];

        let p2 = [hash_to_scalar("Attribute3"), hash_to_scalar("Attribute4")];
        let private_attributes2 = vec![&p2[0], &p2[1]];

        let (commitments_openings, lambda) =
            prepare_blind_sign(&params, &private_attributes, &[]).unwrap();

        let keypair1 = keygen(&params);

        let sig1 = blind_sign(&params, keypair1.secret_key(), &lambda, &[]).unwrap();

        assert!(sig1
            .unblind_and_verify(
                &params,
                keypair1.verification_key(),
                &private_attributes2,
                &[],
                &lambda.get_commitment_hash(),
                &commitments_openings,
            )
            .is_err());
    }

    #[test]
    fn verification_on_two_private_attributes() {
        let params = Parameters::new(2).unwrap();
        let serial_number = params.random_scalar();
        let binding_number = params.random_scalar();
        let private_attributes = vec![&serial_number, &binding_number];

        let keypair1 = keygen(&params);
        let keypair2 = keygen(&params);

        let (commitments_openings, lambda) =
            prepare_blind_sign(&params, &private_attributes, &[]).unwrap();

        let sig1 = blind_sign(&params, keypair1.secret_key(), &lambda, &[])
            .unwrap()
            .unblind_and_verify(
                &params,
                keypair1.verification_key(),
                &private_attributes,
                &[],
                &lambda.get_commitment_hash(),
                &commitments_openings,
            )
            .unwrap();

        let sig2 = blind_sign(&params, keypair2.secret_key(), &lambda, &[])
            .unwrap()
            .unblind_and_verify(
                &params,
                keypair2.verification_key(),
                &private_attributes,
                &[],
                &lambda.get_commitment_hash(),
                &commitments_openings,
            )
            .unwrap();

        let theta1 = prove_bandwidth_credential(
            &params,
            keypair1.verification_key(),
            &sig1,
            &serial_number,
            &binding_number,
        )
        .unwrap();

        let theta2 = prove_bandwidth_credential(
            &params,
            keypair2.verification_key(),
            &sig2,
            &serial_number,
            &binding_number,
        )
        .unwrap();

        assert!(verify_credential(
            &params,
            keypair1.verification_key(),
            &theta1,
            &[],
        ));

        assert!(verify_credential(
            &params,
            keypair2.verification_key(),
            &theta2,
            &[],
        ));

        assert!(!verify_credential(
            &params,
            keypair1.verification_key(),
            &theta2,
            &[],
        ));
    }

    #[test]
    fn verification_on_two_public_attributes() {
        let params = Parameters::new(2).unwrap();
        random_scalars_refs!(attributes, params, 2);

        let keypair1 = keygen(&params);
        let keypair2 = keygen(&params);
        let sig1 = sign(&params, keypair1.secret_key(), &attributes).unwrap();
        let sig2 = sign(&params, keypair2.secret_key(), &attributes).unwrap();

        assert!(verify(
            &params,
            keypair1.verification_key(),
            &attributes,
            &sig1,
        ));

        assert!(!verify(
            &params,
            keypair2.verification_key(),
            &attributes,
            &sig1,
        ));

        assert!(!verify(
            &params,
            keypair1.verification_key(),
            &attributes,
            &sig2,
        ));
    }

    #[test]
    fn verification_on_two_public_and_two_private_attributes() {
        let params = Parameters::new(4).unwrap();
        random_scalars_refs!(public_attributes, params, 2);

        let serial_number = params.random_scalar();
        let binding_number = params.random_scalar();
        let private_attributes = vec![&serial_number, &binding_number];

        let keypair1 = keygen(&params);
        let keypair2 = keygen(&params);

        let (commitments_openings, lambda) =
            prepare_blind_sign(&params, &private_attributes, &public_attributes).unwrap();

        let sig1 = blind_sign(&params, keypair1.secret_key(), &lambda, &public_attributes)
            .unwrap()
            .unblind_and_verify(
                &params,
                keypair1.verification_key(),
                &private_attributes,
                &public_attributes,
                &lambda.get_commitment_hash(),
                &commitments_openings,
            )
            .unwrap();

        let sig2 = blind_sign(&params, keypair2.secret_key(), &lambda, &public_attributes)
            .unwrap()
            .unblind_and_verify(
                &params,
                keypair2.verification_key(),
                &private_attributes,
                &public_attributes,
                &lambda.get_commitment_hash(),
                &commitments_openings,
            )
            .unwrap();

        let theta1 = prove_bandwidth_credential(
            &params,
            keypair1.verification_key(),
            &sig1,
            &serial_number,
            &binding_number,
        )
        .unwrap();

        let theta2 = prove_bandwidth_credential(
            &params,
            keypair2.verification_key(),
            &sig2,
            &serial_number,
            &binding_number,
        )
        .unwrap();

        assert!(verify_credential(
            &params,
            keypair1.verification_key(),
            &theta1,
            &public_attributes,
        ));

        assert!(verify_credential(
            &params,
            keypair2.verification_key(),
            &theta2,
            &public_attributes,
        ));

        assert!(!verify_credential(
            &params,
            keypair1.verification_key(),
            &theta2,
            &public_attributes,
        ));
    }

    #[test]
    fn verification_on_two_public_and_two_private_attributes_from_two_signers() {
        let params = Parameters::new(4).unwrap();
        random_scalars_refs!(public_attributes, params, 2);

        let serial_number = params.random_scalar();
        let binding_number = params.random_scalar();
        let private_attributes = vec![&serial_number, &binding_number];

        let keypairs = ttp_keygen(&params, 2, 3).unwrap();

        let (commitments_openings, lambda) =
            prepare_blind_sign(&params, &private_attributes, &public_attributes).unwrap();

        let sigs = keypairs
            .iter()
            .map(|keypair| {
                blind_sign(&params, keypair.secret_key(), &lambda, &public_attributes)
                    .unwrap()
                    .unblind_and_verify(
                        &params,
                        keypair.verification_key(),
                        &private_attributes,
                        &public_attributes,
                        &lambda.get_commitment_hash(),
                        &commitments_openings,
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();

        let vks = keypairs
            .into_iter()
            .map(|keypair| keypair.verification_key().clone())
            .collect::<Vec<_>>();

        let mut attributes = Vec::with_capacity(private_attributes.len() + public_attributes.len());
        attributes.extend_from_slice(&private_attributes);
        attributes.extend_from_slice(&public_attributes);

        let aggr_vk = aggregate_verification_keys(&vks[..2], Some(&[1, 2])).unwrap();
        let aggr_sig = aggregate_signatures_and_verify(
            &params,
            &aggr_vk,
            &attributes,
            &sigs[..2],
            Some(&[1, 2]),
        )
        .unwrap();

        let theta = prove_bandwidth_credential(
            &params,
            &aggr_vk,
            &aggr_sig,
            &serial_number,
            &binding_number,
        )
        .unwrap();

        assert!(verify_credential(
            &params,
            &aggr_vk,
            &theta,
            &public_attributes,
        ));

        // taking different subset of keys and credentials
        let aggr_vk = aggregate_verification_keys(&vks[1..], Some(&[2, 3])).unwrap();
        let aggr_sig = aggregate_signatures_and_verify(
            &params,
            &aggr_vk,
            &attributes,
            &sigs[1..],
            Some(&[2, 3]),
        )
        .unwrap();

        let theta = prove_bandwidth_credential(
            &params,
            &aggr_vk,
            &aggr_sig,
            &serial_number,
            &binding_number,
        )
        .unwrap();

        assert!(verify_credential(
            &params,
            &aggr_vk,
            &theta,
            &public_attributes,
        ));
    }

    #[test]
    fn signature_bytes_roundtrip() {
        let params = Parameters::default();
        let r = params.random_scalar();
        let s = params.random_scalar();
        let signature = Signature(params.gen1() * r, params.gen1() * s);
        let bytes = signature.to_bytes();

        // also make sure it is equivalent to the internal g1 compressed bytes concatenated
        let expected_bytes = [
            signature.0.to_affine().to_compressed(),
            signature.1.to_affine().to_compressed(),
        ]
        .concat();
        assert_eq!(expected_bytes, bytes);
        assert_eq!(signature, Signature::try_from(&bytes[..]).unwrap())
    }

    #[test]
    fn blinded_signature_bytes_roundtrip() {
        let params = Parameters::default();
        let r = params.random_scalar();
        let s = params.random_scalar();
        let blinded_sig = BlindedSignature(params.gen1() * r, params.gen1() * s);
        let bytes = blinded_sig.to_bytes();

        // also make sure it is equivalent to the internal g1 compressed bytes concatenated
        let expected_bytes = [
            blinded_sig.0.to_affine().to_compressed(),
            blinded_sig.1.to_affine().to_compressed(),
        ]
        .concat();
        assert_eq!(expected_bytes, bytes);
        assert_eq!(blinded_sig, BlindedSignature::try_from(&bytes[..]).unwrap())
    }
}
