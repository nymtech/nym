// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::DkgError;
use crate::utils::hash_g2;
use crate::{Chunk, Share};
use bitvec::order::Msb0;
use bitvec::vec::BitVec;
use bitvec::view::BitView;
use bls12_381::{G1Affine, G2Affine, G2Prepared, G2Projective, Gt};
use group::Curve;
use lazy_static::lazy_static;
use zeroize::Zeroize;

pub mod encryption;
pub mod keys;
pub mod proof_chunking;
pub mod proof_discrete_log;
pub mod proof_sharing;

pub use encryption::{decrypt_share, encrypt_shares, Ciphertexts};
pub use keys::{keygen, DecryptionKey, PublicKey, PublicKeyWithProof};

lazy_static! {
    pub(crate) static ref PAIRING_BASE: Gt =
        bls12_381::pairing(&G1Affine::generator(), &G2Affine::generator());
    pub(crate) static ref G2_GENERATOR_PREPARED: G2Prepared =
        G2Prepared::from(G2Affine::generator());
    pub(crate) static ref DEFAULT_BSGS_TABLE: encryption::BabyStepGiantStepLookup =
        encryption::BabyStepGiantStepLookup::default();
}

// Domain tries to follow guidelines specified by:
// https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-3.1
const SETUP_DOMAIN: &[u8] = b"NYM_COCONUT_NIDKG_V01_CS01_WITH_BLS12381G2_XMD:SHA-256_SSWU_RO_SETUP";
const MAX_EPOCHS_EXP: usize = 32;

// note: CHUNK_BYTES * NUM_CHUNKS must equal to SCALAR_SIZE
pub const CHUNK_BYTES: usize = 2;
pub const NUM_CHUNKS: usize = 16;
pub const SCALAR_SIZE: usize = 32;

/// In paper B; number of distinct chunks
pub const CHUNK_SIZE: usize = 1 << (CHUNK_BYTES << 3);

#[derive(Clone, Debug, PartialEq, PartialOrd)]
// None empty bitvec implies this is a root node
pub struct Tau(BitVec<u32, Msb0>);

impl Tau {
    pub fn new_root() -> Self {
        Tau(BitVec::new())
    }

    pub fn new(epoch: u32) -> Self {
        Tau(epoch.view_bits().to_bitvec())
    }

    pub fn is_valid_epoch(&self, params: &Params) -> bool {
        self.is_leaf(params)
    }

    pub fn left_child(&self) -> Self {
        let mut child = self.0.clone();
        child.push(false);
        Tau(child)
    }

    pub fn right_child(&self) -> Self {
        let mut child = self.0.clone();
        child.push(true);
        Tau(child)
    }

    pub fn is_leaf(&self, params: &Params) -> bool {
        self.height() == params.tree_height
    }

    pub fn try_get_parent_at_height(&self, height: usize) -> Result<Self, DkgError> {
        if height > self.0.len() {
            return Err(DkgError::NotAValidParent);
        }

        Ok(Tau(self.0[..height].to_bitvec()))
    }

    // essentially is this those prefixing the other
    pub fn is_parent_of(&self, other: &Tau) -> bool {
        if self.0.len() > other.0.len() {
            return false;
        }

        for (i, b) in self.0.iter().enumerate() {
            if b != other.0[i] {
                return false;
            }
        }

        true
    }

    pub fn lowest_valid_epoch_child(&self, params: &Params) -> Result<Self, DkgError> {
        if self.0.len() > params.tree_height {
            // this node is already BELOW a valid leaf-epoch node. it can only happen
            // if either some invariant was broken or additional data was pushed to `tau`
            // in order compute some intermediate results, but in that case this method should have
            // never been called anyway. tl;dr: if this is called, the underlying key is malformed
            return Err(DkgError::NotAValidParent);
        }
        let mut child = self.0.clone();
        for _ in 0..(params.tree_height - self.0.len()) {
            child.push(false)
        }

        Ok(Tau(child))
    }

    pub fn height(&self) -> usize {
        self.0.len()
    }

    fn evaluate_f(&self, params: &Params) -> G2Projective {
        // right now completely ignore existence of f_h
        self.0
            .iter()
            .by_vals()
            .zip(params.fs.iter())
            .filter(|(i, _)| *i)
            .map(|(_, f_i)| f_i)
            .fold(params.f0, |acc, f_i| acc + f_i)
    }
}

impl Zeroize for Tau {
    fn zeroize(&mut self) {
        for v in self.0.as_raw_mut_slice() {
            v.zeroize()
        }
    }
}

pub struct Params {
    /// In paper $\lambda$
    tree_height: usize,

    // keeping f0 separate from the rest of the curve points makes it easier to work with tau
    f0: G2Projective,
    fs: Vec<G2Projective>, // f_1, f_2, .... f_i in the paper
    h: G2Projective,

    /// Precomputed `h` used for the miller loop
    _h_prepared: G2Prepared,
}

pub fn setup() -> Params {
    let f0 = hash_g2(b"f0", SETUP_DOMAIN);

    // is there a point in generating ALL of them at start?
    let fs = (1..=MAX_EPOCHS_EXP)
        .map(|i| hash_g2(format!("f{}", i), SETUP_DOMAIN))
        .collect();

    let h = hash_g2(b"h", SETUP_DOMAIN);

    Params {
        tree_height: MAX_EPOCHS_EXP,
        f0,
        fs,
        _h_prepared: G2Prepared::from(h.to_affine()),
        h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::bitvec;
    use bitvec::order::Msb0;

    #[test]
    fn creating_tau_from_epoch() {
        assert!(Tau::new_root().0.is_empty());

        let zero = Tau::new(0);
        assert!(zero.0.iter().by_vals().all(|b| !b));

        let one = Tau::new(1);
        let mut iter = one.0.iter().by_vals();
        // first 31 bits are 0, the last one is 1
        for _ in 0..31 {
            assert!(!iter.next().unwrap())
        }
        assert!(iter.next().unwrap());

        // 101010 in binary
        let forty_two = Tau::new(42);
        // first 26 bits are not set
        let mut iter = forty_two.0.iter().by_vals();
        for _ in 0..26 {
            assert!(!iter.next().unwrap())
        }
        assert!(iter.next().unwrap());
        assert!(!iter.next().unwrap());
        assert!(iter.next().unwrap());
        assert!(!iter.next().unwrap());
        assert!(iter.next().unwrap());
        assert!(!iter.next().unwrap());

        // value that requires an actual u32 (i.e. takes 4 bytes to represent)
        // 11000100_01000000_01001001_01101011 in binary
        let big_val = Tau::new(3292547435);
        let expected = bitvec![u32, Msb0;
            1, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1,
            0, 1, 1
        ];
        assert_eq!(expected, big_val.0)
    }

    #[test]
    fn getting_parent_at_height() {
        let tau = Tau(bitvec![u32, Msb0; 1,0,1,1,0,0,1]);

        let expected_0 = Tau(BitVec::new());
        let expected_1 = Tau(bitvec![u32, Msb0; 1]);
        let expected_5 = Tau(bitvec![u32, Msb0; 1,0,1,1,0]);

        assert_eq!(expected_0, tau.try_get_parent_at_height(0).unwrap());
        assert_eq!(expected_1, tau.try_get_parent_at_height(1).unwrap());
        assert_eq!(expected_5, tau.try_get_parent_at_height(5).unwrap());
        assert_eq!(tau, tau.try_get_parent_at_height(7).unwrap());
        assert!(tau.try_get_parent_at_height(8).is_err())
    }
}
