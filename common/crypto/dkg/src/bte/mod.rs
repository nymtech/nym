// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::DkgError;
use crate::utils::{hash_g2, RandomOracleBuilder};
use crate::{Chunk, Share};
use bitvec::field::BitField;
use bitvec::order::Msb0;
use bitvec::vec::BitVec;
use bitvec::view::BitView;
use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt};
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

// this particular domain is not for curve hashing, but might as well also follow the same naming pattern
const TREE_TAU_EXTENSION_DOMAIN: &[u8] = b"NYM_COCONUT_NIDKG_V01_CS01_SHA-256_TREE_EXTENSION";

const MAX_EPOCHS_EXP: usize = 32;
const HASH_SECURITY_PARAM: usize = 256;

// note: CHUNK_BYTES * NUM_CHUNKS must equal to SCALAR_SIZE
pub const CHUNK_BYTES: usize = 2;
pub const NUM_CHUNKS: usize = 16;
pub const SCALAR_SIZE: usize = 32;

/// In paper B; number of distinct chunks
pub const CHUNK_SIZE: usize = 1 << (CHUNK_BYTES << 3);

pub(crate) type EpochStore = u32;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
// None empty bitvec implies this is a root node
pub(crate) struct Tau(BitVec<EpochStore, Msb0>);

impl Tau {
    pub fn new_root() -> Self {
        Tau(BitVec::new())
    }

    // TODO: perhaps this should be explicitly moved to some test module
    #[cfg(test)]
    pub(crate) fn new(epoch: EpochStore) -> Self {
        Tau(epoch.view_bits().to_bitvec())
    }

    #[allow(unused)]
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
        self.height() == params.lambda_t
    }

    pub fn try_get_parent_at_height(&self, height: usize) -> Result<Self, DkgError> {
        if height > self.0.len() {
            return Err(DkgError::NotAValidParent);
        }

        Ok(Tau(self.0[..height].to_bitvec()))
    }

    // essentially is this tau prefixing the other
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

    pub fn lowest_valid_epoch_child(&self, params: &Params) -> Result<Epoch, DkgError> {
        if self.0.len() > params.lambda_t {
            // this node is already BELOW a valid leaf-epoch node. it can only happen
            // if either some invariant was broken or additional data was pushed to `tau`
            // in order compute some intermediate results, but in that case this method should have
            // never been called anyway. tl;dr: if this is called, the underlying key is malformed
            return Err(DkgError::NotAValidParent);
        }
        let mut child = self.0.clone();
        for _ in 0..(params.lambda_t - self.0.len()) {
            child.push(false)
        }

        // the unwrap here is fine as we ensure we have exactly `params.tree_height` bits here
        // (we could just propagate the error instead of unwraping and putting it behind an `Ok` anyway
        // but I'd prefer to just blow up since this would be a serious error
        Ok(Epoch::try_from_tau(&Tau(child), params).unwrap())
    }

    pub fn height(&self) -> usize {
        self.0.len()
    }

    fn extend(
        &self,
        rr: &[G1Projective; NUM_CHUNKS],
        ss: &[G1Projective; NUM_CHUNKS],
        cc: &[[G1Projective; NUM_CHUNKS]],
    ) -> Self {
        let mut random_oracle_builder = RandomOracleBuilder::new(TREE_TAU_EXTENSION_DOMAIN);
        random_oracle_builder.update_with_g1_elements(rr.iter());
        random_oracle_builder.update_with_g1_elements(ss.iter());
        for ciphertext_chunks in cc {
            random_oracle_builder.update_with_g1_elements(ciphertext_chunks.iter());
        }

        let tau_mem = self.0.as_raw_slice();
        assert_eq!(tau_mem.len(), 1, "tau length invariant was broken");
        random_oracle_builder.update(&tau_mem[0].to_be_bytes());

        let oracle_output = random_oracle_builder.finalize();
        debug_assert_eq!(oracle_output.len() * 8, HASH_SECURITY_PARAM);

        let mut extended_tau = self.clone();
        for byte in oracle_output {
            extended_tau
                .0
                .extend_from_bitslice(byte.view_bits::<Msb0>())
        }

        extended_tau
    }

    // considers all lambda_t + lambda_h bits
    fn evaluate_f(&self, params: &Params) -> G2Projective {
        self.0
            .iter()
            .by_vals()
            .zip(params.fs.iter().chain(params.fh.iter()))
            .filter(|(i, _)| *i)
            .map(|(_, f_i)| f_i)
            .fold(params.f0, |acc, f_i| acc + f_i)
    }

    // only considers up to lambda_t bits
    fn evaluate_partial_f(&self, params: &Params) -> G2Projective {
        self.0
            .iter()
            .by_vals()
            .zip(params.fs.iter())
            .filter(|(i, _)| *i)
            .map(|(_, f_i)| f_i)
            .fold(params.f0, |acc, f_i| acc + f_i)
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let len_bytes = (self.0.len() as u32).to_be_bytes();
        len_bytes
            .into_iter()
            .chain(self.0.chunks(8).map(BitField::load_be))
            .collect()
    }

    pub(crate) fn try_from_bytes(b: &[u8]) -> Result<Self, DkgError> {
        if b.len() < 4 {
            return Err(DkgError::new_deserialization_failure(
                "Tau",
                "insufficient number of bytes provided",
            ));
        }
        let tau_len = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize;

        // maximum theoretical length
        if tau_len > MAX_EPOCHS_EXP + HASH_SECURITY_PARAM {
            return Err(DkgError::new_deserialization_failure(
                "Tau",
                format!(
                    "malformed length {} is greater than maximum {}",
                    tau_len,
                    MAX_EPOCHS_EXP + HASH_SECURITY_PARAM
                ),
            ));
        }

        if tau_len == 0 {
            if b.len() != 4 {
                Err(DkgError::new_deserialization_failure(
                    "Tau",
                    "malformed bytes",
                ))
            } else {
                Ok(Tau::new_root())
            }
        } else if b.len() == 4 {
            Err(DkgError::new_deserialization_failure(
                "Tau",
                "insufficient number of bytes provided",
            ))
        } else {
            let mut inner = BitVec::repeat(false, tau_len);
            for (slot, &byte) in inner.chunks_mut(8).zip(b[4..].iter()) {
                slot.store_be(byte);
            }

            Ok(Tau(inner))
        }
    }
}

impl Zeroize for Tau {
    fn zeroize(&mut self) {
        for v in self.0.as_raw_mut_slice() {
            v.zeroize()
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Epoch(EpochStore);

impl Epoch {
    pub fn new(value: EpochStore) -> Self {
        Epoch(value)
    }

    pub(crate) fn as_tau(&self) -> Tau {
        (*self).into()
    }

    pub(crate) fn as_extended_tau(
        &self,
        rr: &[G1Projective; NUM_CHUNKS],
        ss: &[G1Projective; NUM_CHUNKS],
        cc: &[[G1Projective; NUM_CHUNKS]],
    ) -> Tau {
        self.as_tau().extend(rr, ss, cc)
    }

    pub(crate) fn try_from_tau(tau: &Tau, params: &Params) -> Result<Self, DkgError> {
        if !tau.is_leaf(params) {
            Err(DkgError::MalformedEpoch)
        } else {
            Ok(Epoch(tau.0.load_be()))
        }
    }
}

impl From<Epoch> for Tau {
    fn from(epoch: Epoch) -> Self {
        Tau(epoch.0.view_bits().to_bitvec())
    }
}

impl From<EpochStore> for Epoch {
    fn from(epoch: EpochStore) -> Self {
        Epoch(epoch)
    }
}

pub struct Params {
    /// Maximum size of an epoch, in bits.
    pub lambda_t: usize,

    /// Security parameter of our $H_{\Lamda_H}$ hash function
    pub lambda_h: usize,

    // keeping f0 separate from the rest of the curve points makes it easier to work with tau
    f0: G2Projective,
    fs: Vec<G2Projective>, // f_1, f_2, .... f_{lambda_t} in the paper
    fh: Vec<G2Projective>, // f_{lambda_t+1}, f_{lambda_t+1}, .... f_{lambda_t+lambda_h} in the paper
    h: G2Projective,

    /// Precomputed `h` used for the miller loop
    _h_prepared: G2Prepared,
}

pub fn setup() -> Params {
    let f0 = hash_g2(b"f0", SETUP_DOMAIN);

    let fs = (1..=MAX_EPOCHS_EXP)
        .map(|i| hash_g2(format!("f{}", i), SETUP_DOMAIN))
        .collect();

    let fh = (0..HASH_SECURITY_PARAM)
        .map(|i| hash_g2(format!("fh{}", i), SETUP_DOMAIN))
        .collect();

    let h = hash_g2(b"h", SETUP_DOMAIN);

    Params {
        lambda_t: MAX_EPOCHS_EXP,
        lambda_h: HASH_SECURITY_PARAM,
        f0,
        fs,
        fh,
        h,
        _h_prepared: G2Prepared::from(h.to_affine()),
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

    #[test]
    fn converting_tau_to_epoch() {
        let params = setup();

        let tau0: Tau = Epoch::new(0).into();
        let tau1: Tau = Epoch::new(1).into();
        let tau42: Tau = Epoch::new(42).into();
        let tau_big: Tau = Epoch::new(3292547435).into();

        assert_eq!(Epoch::new(0), Epoch::try_from_tau(&tau0, &params).unwrap());
        assert_eq!(Epoch::new(1), Epoch::try_from_tau(&tau1, &params).unwrap());
        assert_eq!(
            Epoch::new(42),
            Epoch::try_from_tau(&tau42, &params).unwrap()
        );
        assert_eq!(
            Epoch::new(3292547435),
            Epoch::try_from_tau(&tau_big, &params).unwrap()
        );

        assert!(Epoch::try_from_tau(&Tau(BitVec::new()), &params).is_err());
        assert!(Epoch::try_from_tau(&Tau(bitvec![u32, Msb0; 1,0,1,1,0]), &params).is_err());
        let _31bit_tau = Tau(bitvec![u32, Msb0;
            1, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1,
            0, 1
        ]);
        assert!(Epoch::try_from_tau(&_31bit_tau, &params).is_err());

        let _33bit_tau = Tau(bitvec![u32, Msb0;
            1, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1,
            0, 1, 1, 0
        ]);
        assert!(Epoch::try_from_tau(&_33bit_tau, &params).is_err());
    }

    #[test]
    fn tau_roundtrip() {
        let good_taus = vec![
            Tau::new_root(),
            Tau::new(0),
            Tau::new(1),
            Tau::new(2),
            Tau::new(42),
            Tau::new(123456),
            Tau::new(3292547435),
            Tau::new(u32::MAX),
        ];

        for tau in good_taus {
            let bytes = tau.to_bytes();
            let recovered = Tau::try_from_bytes(&bytes).unwrap();
            assert_eq!(tau, recovered);
        }

        // more valid variants
        let mut another_tau = Tau::new(u32::MAX);
        another_tau.0.push(true);
        another_tau.0.push(false);
        another_tau.0.push(true);

        let bytes = another_tau.to_bytes();
        let recovered = Tau::try_from_bytes(&bytes).unwrap();
        assert_eq!(another_tau, recovered);

        // ensure there are no panics
        let big_length_bytes = [255, 255, 255, 255, 42];
        assert!(Tau::try_from_bytes(&big_length_bytes).is_err());

        assert!(Tau::try_from_bytes(&[]).is_err());
        assert!(Tau::try_from_bytes(&[1, 1, 1, 1]).is_err());
        assert!(Tau::try_from_bytes(&[0, 0, 0, 1]).is_err());
        assert!(Tau::try_from_bytes(&[1, 0, 0, 0]).is_err());
        assert!(Tau::try_from_bytes(&[1, 0, 0]).is_err());
    }
}
