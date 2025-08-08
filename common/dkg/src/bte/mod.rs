// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::LazyLock;

use crate::utils::hash_g2;
use crate::{Chunk, Share};
use bls12_381::{G1Affine, G2Affine, G2Prepared, G2Projective, Gt};
use group::Curve;

pub mod encryption;
pub mod keys;
pub mod proof_chunking;
pub mod proof_discrete_log;
pub mod proof_sharing;

pub use encryption::{decrypt_share, encrypt_shares, Ciphertexts};
pub use keys::{keygen, DecryptionKey, PublicKey, PublicKeyWithProof};

pub(crate) static PAIRING_BASE: LazyLock<Gt> =
    LazyLock::new(|| bls12_381::pairing(&G1Affine::generator(), &G2Affine::generator()));
pub(crate) static G2_GENERATOR_PREPARED: LazyLock<G2Prepared> =
    LazyLock::new(|| G2Prepared::from(G2Affine::generator()));

// Domain tries to follow guidelines specified by:
// https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-3.1
const SETUP_DOMAIN: &[u8] = b"NYM_COCONUT_NIDKG_V01_CS01_WITH_BLS12381G2_XMD:SHA-256_SSWU_RO_SETUP";

const HASH_SECURITY_PARAM: usize = 256;

// note: CHUNK_BYTES * NUM_CHUNKS must equal to SCALAR_SIZE
pub const CHUNK_BYTES: usize = 2;
pub const NUM_CHUNKS: usize = 16;
pub const SCALAR_SIZE: usize = 32;

/// In paper B; number of distinct chunks
pub const CHUNK_SIZE: usize = 1 << (CHUNK_BYTES << 3);

// considers all lambda_h bits
pub fn evaluate_f(params: &Params) -> G2Projective {
    params.fh.iter().fold(params.f0, |acc, f_i| acc + f_i)
}

pub struct Params {
    /// Security parameter of our $H_{\Lamda_H}$ hash function
    pub lambda_h: usize,

    f0: G2Projective,
    fh: Vec<G2Projective>, // f_{lambda_h}, f_{lambda_h+1}, .... f_{lambda_h} in the paper
    h: G2Projective,

    /// Precomputed `h` used for the miller loop
    _h_prepared: G2Prepared,
}

pub fn setup() -> Params {
    let f0 = hash_g2(b"f0", SETUP_DOMAIN);

    let fh = (0..HASH_SECURITY_PARAM)
        .map(|i| hash_g2(format!("fh{i}"), SETUP_DOMAIN))
        .collect();

    let h = hash_g2(b"h", SETUP_DOMAIN);

    Params {
        lambda_h: HASH_SECURITY_PARAM,
        f0,
        fh,
        h,
        _h_prepared: G2Prepared::from(h.to_affine()),
    }
}
