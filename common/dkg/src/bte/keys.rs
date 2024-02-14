// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::proof_discrete_log::ProofOfDiscreteLog;
use crate::bte::Params;
use crate::error::DkgError;
use crate::utils::{deserialize_g1, deserialize_g2, deserialize_scalar};
use bls12_381::{G1Projective, G2Projective, Scalar};
use ff::Field;
use group::GroupEncoding;
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use rand_core::RngCore;
use zeroize::Zeroize;

// produces public key and a decryption key for the root of the tree
pub fn keygen(params: &Params, mut rng: impl RngCore) -> (DecryptionKey, PublicKeyWithProof) {
    let g1 = G1Projective::generator();
    let g2 = G2Projective::generator();

    let mut x = Scalar::random(&mut rng);
    let y = g1 * x;

    let proof = ProofOfDiscreteLog::construct(&mut rng, &y, &x);

    let mut rho = Scalar::random(&mut rng);

    let a = g1 * rho;
    let b = g2 * x + params.f0 * rho;

    let dh = params.fh.iter().map(|fh_i| fh_i * rho).collect();
    let e = params.h * rho;

    let dk = DecryptionKey::new_root(a, b, dh, e);

    let public_key = PublicKey(y);
    let key_with_proof = PublicKeyWithProof {
        key: public_key,
        proof,
    };

    x.zeroize();
    rho.zeroize();

    (dk, key_with_proof)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey(pub(crate) G1Projective);

impl PublicKey {
    pub fn verify(&self, proof: &ProofOfDiscreteLog) -> bool {
        proof.verify(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKeyWithProof {
    pub(crate) key: PublicKey,
    pub(crate) proof: ProofOfDiscreteLog,
}

impl PemStorableKey for PublicKeyWithProof {
    type Error = DkgError;

    fn pem_type() -> &'static str {
        "DKG PUBLIC KEY WITH PROOF"
    }
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes)
    }
}

impl PublicKeyWithProof {
    pub fn verify(&self) -> bool {
        self.key.verify(&self.proof)
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // we have 2 G1 elements and 1 Scalar
        let mut bytes = Vec::with_capacity(2 * 48 + 32);
        bytes.extend_from_slice(self.key.0.to_bytes().as_ref());
        bytes.extend_from_slice(self.proof.rand_commitment.to_bytes().as_ref());
        bytes.extend_from_slice(self.proof.response.to_bytes().as_ref());

        bytes
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        if bytes.len() != 2 * 48 + 32 {
            return Err(DkgError::new_deserialization_failure(
                "PublicKeyWithProof",
                "provided bytes had invalid length",
            ));
        }

        let y_bytes = &bytes[..48];
        let commitment_bytes = &bytes[48..96];
        let response_bytes = &bytes[96..];

        let y = deserialize_g1(y_bytes).ok_or_else(|| {
            DkgError::new_deserialization_failure("PublicKeyWithProof.key.0", "invalid curve point")
        })?;

        let rand_commitment = deserialize_g1(commitment_bytes).ok_or_else(|| {
            DkgError::new_deserialization_failure(
                "PublicKeyWithProof.proof.rand_commitment",
                "invalid curve point",
            )
        })?;

        let response = deserialize_scalar(response_bytes).ok_or_else(|| {
            DkgError::new_deserialization_failure(
                "PublicKeyWithProof.proof.response",
                "invalid scalar",
            )
        })?;

        Ok(PublicKeyWithProof {
            key: PublicKey(y),
            proof: ProofOfDiscreteLog {
                rand_commitment,
                response,
            },
        })
    }
}

#[derive(Debug, Zeroize)]
#[zeroize(drop)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct DecryptionKey {
    // g1^rho
    pub(crate) a: G1Projective,

    // g2^x * f0^rho
    pub(crate) b: G2Projective,

    // fh_i^rho, always lambda_h elements
    pub(crate) dh: Vec<G2Projective>,

    // h^rho
    pub(crate) e: G2Projective,
}

impl PemStorableKey for DecryptionKey {
    type Error = DkgError;

    fn pem_type() -> &'static str {
        "DKG DECRYPTION KEY"
    }
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes)
    }
}

impl DecryptionKey {
    fn new_root(a: G1Projective, b: G2Projective, dh: Vec<G2Projective>, e: G2Projective) -> Self {
        DecryptionKey { a, b, dh, e }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let g1_elements = 1;
        let g2_elements = self.dh.len() + 2;

        // the extra 8 comes from the triple u32 we use for encoding lengths of ds and dh
        let mut bytes = Vec::with_capacity(g1_elements * 48 + g2_elements * 96 + 8);

        bytes.extend_from_slice(self.a.to_bytes().as_ref());
        bytes.extend_from_slice(self.b.to_bytes().as_ref());
        bytes.extend_from_slice(&((self.dh.len() as u32).to_be_bytes()));
        for dh_i in &self.dh {
            bytes.extend_from_slice(dh_i.to_bytes().as_ref());
        }
        bytes.extend_from_slice(self.e.to_bytes().as_ref());

        bytes
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        // at the very least we require bytes for:
        // - a ( 48 )
        // - b ( 96 )
        // - length indication of dh ( 4 )
        // - e ( 96 )
        if bytes.len() < 48 + 96 + 4 + 96 {
            return Err(DkgError::new_deserialization_failure(
                "Node",
                "insufficient number of bytes provided",
            ));
        }

        let mut i = 0;
        let a = deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
            DkgError::new_deserialization_failure("Node.a", "invalid curve point")
        })?;
        i += 48;

        let b = deserialize_g2(&bytes[i..i + 96]).ok_or_else(|| {
            DkgError::new_deserialization_failure("Node.b", "invalid curve point")
        })?;
        i += 96;

        let dh_len = u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;

        if bytes[i..].len() != (dh_len + 1) * 96 {
            return Err(DkgError::new_deserialization_failure(
                "Node",
                "insufficient number of bytes provided (dh)",
            ));
        }

        let mut dh = Vec::with_capacity(dh_len);
        for j in 0..dh_len {
            let dh_i = deserialize_g2(&bytes[i..i + 96]).ok_or_else(|| {
                DkgError::new_deserialization_failure(format!("Node.dh_{j}"), "invalid curve point")
            })?;

            dh.push(dh_i);
            i += 96;
        }

        let e = deserialize_g2(&bytes[i..]).ok_or_else(|| {
            DkgError::new_deserialization_failure("Node.h", "invalid curve point")
        })?;

        Ok(Self { a, b, dh, e })
    }
}

pub struct KeyPair {
    pub(crate) private_key: DecryptionKey,
    pub(crate) public_key: PublicKeyWithProof,
}

impl KeyPair {
    pub fn new(params: &Params, rng: impl RngCore) -> Self {
        let (dk, pk) = keygen(params, rng);
        Self {
            private_key: dk,
            public_key: pk,
        }
    }
    pub fn private_key(&self) -> &DecryptionKey {
        &self.private_key
    }

    pub fn public_key(&self) -> &PublicKeyWithProof {
        &self.public_key
    }

    pub fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Result<Self, DkgError> {
        Ok(KeyPair {
            private_key: DecryptionKey::try_from_bytes(priv_bytes)?,
            public_key: PublicKeyWithProof::try_from_bytes(pub_bytes)?,
        })
    }
}

impl PemStorableKeyPair for KeyPair {
    type PrivatePemKey = DecryptionKey;
    type PublicPemKey = PublicKeyWithProof;

    fn private_key(&self) -> &Self::PrivatePemKey {
        self.private_key()
    }

    fn public_key(&self) -> &Self::PublicPemKey {
        self.public_key()
    }

    fn from_keys(private_key: Self::PrivatePemKey, public_key: Self::PublicPemKey) -> Self {
        KeyPair {
            private_key,
            public_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bte::setup;
    use rand_core::SeedableRng;

    #[test]
    fn public_key_with_proof_roundtrip() {
        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (_, pk) = keygen(&params, &mut rng);
        let bytes = pk.to_bytes();
        let recovered = PublicKeyWithProof::try_from_bytes(&bytes).unwrap();

        assert_eq!(pk, recovered)
    }
}
