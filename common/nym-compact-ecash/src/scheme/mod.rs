use std::cell::Cell;
use std::convert::TryFrom;
use std::convert::TryInto;

use bls12_381::{G1Projective, Scalar};
use group::Curve;

use crate::error::{CompactEcashError, Result};
use crate::scheme::setup::Parameters;
use crate::traits::Bytable;
use crate::utils::try_deserialize_g1_projective;

pub mod aggregation;
pub mod keygen;
pub mod setup;
pub mod spend;
pub mod withdrawal;

pub type SignerIndex = u64;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Signature(pub(crate) G1Projective, pub(crate) G1Projective);

pub type PartialSignature = Signature;

impl TryFrom<&[u8]> for Signature {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<Signature> {
        if bytes.len() != 96 {
            return Err(CompactEcashError::Deserialization(format!(
                "Signature must be exactly 96 bytes, got {}",
                bytes.len()
            )));
        }

        let sig1_bytes: &[u8; 48] = &bytes[..48].try_into().expect("Slice size != 48");
        let sig2_bytes: &[u8; 48] = &bytes[48..].try_into().expect("Slice size != 48");

        let sig1 = try_deserialize_g1_projective(
            sig1_bytes,
            CompactEcashError::Deserialization("Failed to deserialize compressed sig1".to_string()),
        )?;

        let sig2 = try_deserialize_g1_projective(
            sig2_bytes,
            CompactEcashError::Deserialization("Failed to deserialize compressed sig2".to_string()),
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
}

impl Bytable for Signature {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        Signature::from_bytes(slice)
    }
}


#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct BlindedSignature(G1Projective, G1Projective);

pub struct SignatureShare {
    signature: Signature,
    index: SignerIndex,
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

pub struct PartialWallet {
    sig: Signature,
    v: Scalar,
    idx: Option<SignerIndex>,
}

impl PartialWallet {
    pub fn signature(&self) -> &Signature { &self.sig }
    pub fn v(&self) -> Scalar { self.v }
    pub fn index(&self) -> Option<SignerIndex> {
        self.idx
    }
}

pub struct Wallet {
    sig: Signature,
    v: Scalar,
    t: Scalar,
    l: Cell<u64>,
}

impl Wallet {
    pub fn signature(&self) -> &Signature { &self.sig }
    pub fn v(&self) -> Scalar { self.v }
    pub fn t(&self) -> Scalar { self.t }
    fn up(&self) {
        self.l.set(self.l.get() + 1);
    }
}