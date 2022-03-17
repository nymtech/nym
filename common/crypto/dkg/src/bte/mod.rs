// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::proof_discrete_log::ProofOfDiscreteLog;
use crate::error::DkgError;
use crate::utils::hash_g2;
use bitvec::vec::BitVec;
use bitvec::view::BitView;
use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt, Scalar};
use ff::Field;
use group::{Curve, GroupEncoding};
use lazy_static::lazy_static;
use rand_core::RngCore;
use std::collections::HashMap;
use std::ops::Neg;
use zeroize::Zeroize;

pub mod proof_chunking;
pub mod proof_discrete_log;
pub mod proof_sharing;

// lambda - height of tree with 2^lambda leaves
// tau - node path?; root has empty path with l = 0, while for a leaf l = lambda
// tau - vector of bits

// l - tree height
// lambda - MAX height

lazy_static! {
    static ref PAIRING_BASE: Gt =
        bls12_381::pairing(&G1Affine::generator(), &G2Affine::generator());
    static ref G2_GENERATOR_PREPARED: G2Prepared = G2Prepared::from(G2Affine::generator());
    static ref DEFAULT_BSGS_TABLE: BabyStepGiantStepLookup = BabyStepGiantStepLookup::default();
}

// Domain tries to follow guidelines specified by:
// https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-3.1
const SETUP_DOMAIN: &[u8] = b"NYM_COCONUT_NIDKG_V01_CS01_WITH_BLS12381G2_XMD:SHA-256_SSWU_RO_SETUP";

#[derive(Clone)]
struct Epoch(BitVec<u32>);

impl Epoch {
    fn new(epoch: u32) -> Self {
        Epoch(epoch.view_bits().to_bitvec())
    }

    fn extend(&self) -> Self {
        todo!()
    }

    fn evaluate_f(&self, params: &Params) -> G2Projective {
        // temp assertion
        assert_eq!(self.0.len(), params.fs.len());

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

impl Zeroize for Epoch {
    fn zeroize(&mut self) {
        for v in self.0.as_raw_mut_slice() {
            v.zeroize()
        }
    }
}

#[derive(PartialEq, Eq, Debug, Zeroize)]
#[cfg_attr(test, derive(Clone))]
#[zeroize(drop)]
pub struct Share(pub(crate) Scalar);

impl Share {
    #[cfg(test)]
    pub(crate) fn random(mut rng: impl RngCore) -> Self {
        Share(Scalar::random(&mut rng))
    }

    pub(crate) fn to_chunks(&self) -> ChunkedShare {
        let mut chunks = [0; NUM_CHUNKS];
        let mut bytes = self.0.to_bytes();

        for (chunk, chunk_bytes) in chunks.iter_mut().zip(bytes[..].chunks_exact(CHUNK_BYTES)) {
            let mut tmp = [0u8; CHUNK_BYTES];
            tmp.copy_from_slice(chunk_bytes);
            *chunk = Chunk::from_be_bytes(tmp)
        }

        bytes.zeroize();
        ChunkedShare { chunks }
    }
}

// TODO: rename
#[derive(Default, Zeroize)]
#[zeroize(drop)]
pub(crate) struct ChunkedShare {
    pub(crate) chunks: [Chunk; NUM_CHUNKS],
}

pub type Chunk = u16;

// note: CHUNK_BYTES * NUM_CHUNKS must equal to SCALAR_SIZE
pub const CHUNK_BYTES: usize = 2;
pub const NUM_CHUNKS: usize = 16;
pub const SCALAR_SIZE: usize = 32;
pub const CHUNK_MAX: usize = 1 << (CHUNK_BYTES << 3);

// pub type Chunk = u32;
//
// pub const CHUNK_BYTES: usize = 4;
// pub const NUM_CHUNKS: usize = 8;
// pub const SCALAR_SIZE: usize = 32;
// pub const CHUNK_MAX: usize = 1 << (CHUNK_BYTES << 3);

impl From<Share> for ChunkedShare {
    fn from(share: Share) -> ChunkedShare {
        share.to_chunks()
    }
}

impl TryFrom<ChunkedShare> for Share {
    type Error = DkgError;

    fn try_from(chunked: ChunkedShare) -> Result<Share, Self::Error> {
        let mut bytes = [0u8; SCALAR_SIZE];
        for (chunk, chunk_bytes) in chunked
            .chunks
            .iter()
            .zip(bytes[..].chunks_exact_mut(CHUNK_BYTES))
        {
            let tmp = chunk.to_be_bytes();
            chunk_bytes.copy_from_slice(&tmp[..]);
        }

        let recovered = Option::from(Scalar::from_bytes(&bytes))
            .map(Share)
            .ok_or(DkgError::MalformedShare)?;

        bytes.zeroize();
        Ok(recovered)
    }
}

const MAX_EPOCHS_EXP: usize = 32;

pub struct Params {
    /// In paper $\lambda$
    tree_height: usize,

    // keeping f0 separate from the rest of the curve points makes it easier to work with tau
    f0: G2Projective,
    fs: Vec<G2Projective>, // f_1, f_2, .... f_i in the paper
    h: G2Projective,
    // pub lambda_t: usize,
    // pub lambda_h: usize,
    // pub f0: G2Projective,       // f_0 in the paper.
    // pub f: Vec<G2Projective>,   // f_1, ..., f_{lambda_T} in the paper.
    // pub f_h: Vec<G2Projective>, // The remaining lambda_H f_i's in the paper.
    // pub h: G2Projective,
}

pub struct Ciphertext {
    pub r: [G1Projective; NUM_CHUNKS],
    pub s: [G1Projective; NUM_CHUNKS],
    pub z: [G2Projective; NUM_CHUNKS],
    pub ciphertext_chunks: Vec<[G1Projective; NUM_CHUNKS]>,
}

struct SingleChunkCiphertext {
    r: G1Projective,
    s: G1Projective,
    z: G2Projective,
    c: G1Projective,
}

#[derive(Debug, Clone)]
pub struct PublicKey(pub(crate) G1Projective);

impl PublicKey {
    pub(crate) fn inner(&self) -> &G1Projective {
        &self.0
    }

    pub fn verify(&self, proof: &ProofOfDiscreteLog) -> bool {
        proof.verify(&self.0)
    }
}

// TODO: that will need to be moved elsewhere

pub struct PublicKeyWithProof {
    key: PublicKey,
    proof: ProofOfDiscreteLog,
}

impl PublicKeyWithProof {
    pub fn verify(&self) -> bool {
        self.key.verify(&self.proof)
    }
}

pub struct DecryptionKey {
    // TODO: why not just a single node?
    nodes: Vec<Node>,
}

impl DecryptionKey {
    fn update(&mut self) {
        //
    }
}

#[derive(Zeroize)]
#[zeroize(drop)]
pub(crate) struct Node {
    epoch: Epoch,

    // g1^rho
    a: G1Projective,

    // g2^x
    b: G2Projective,

    // f_i^rho
    ds: Vec<G2Projective>,

    // h^rho
    e: G2Projective,
}

// params include message space M and height \lambda for a binary tree
// message space is within [-R, S]
fn setup() -> Params {
    let f0 = hash_g2(b"f0", SETUP_DOMAIN);

    // is there a point in generating ALL of them at start?
    let fs = (1..=MAX_EPOCHS_EXP)
        .map(|i| hash_g2(format!("f{}", i), SETUP_DOMAIN))
        .collect();

    let h = hash_g2(b"h", SETUP_DOMAIN);

    // fh with extra 256 elements??

    Params {
        tree_height: MAX_EPOCHS_EXP,
        f0,
        fs,
        h,
    }
}

// produces public key and a decryption key for the root of the tree
fn keygen(params: &Params, mut rng: impl RngCore) -> (PublicKeyWithProof, DecryptionKey) {
    let g1 = G1Projective::generator();
    let g2 = G2Projective::generator();

    let mut x = Scalar::random(&mut rng);
    let y = g1 * x;

    let proof = ProofOfDiscreteLog::construct(&mut rng, &y, &x);

    let mut rho = Scalar::random(&mut rng);

    let a = g1 * rho;
    let b = g2 * x;

    let ds = params.fs.iter().map(|f_i| f_i * rho).collect();
    let e = params.h * rho;

    let dk = DecryptionKey {
        nodes: vec![Node {
            epoch: Epoch::new(0),
            a,
            b,
            ds,
            e,
        }],
    };

    let public_key = PublicKey(y);
    let key_with_proof = PublicKeyWithProof {
        key: public_key,
        proof,
    };

    x.zeroize();
    rho.zeroize();

    (key_with_proof, dk)
}

fn verify_key(pk: &PublicKey, proof: &ProofOfDiscreteLog) -> bool {
    proof.verify(&pk.0)
}

// evolve?
// update?
// epoch is epoch_bit I think
fn derive_key(dk: DecryptionKey, epoch: Epoch) -> DecryptionKey {
    todo!()
}

#[inline]
fn encrypt_chunk(
    m: &Chunk,
    pk: &PublicKey,
    epoch: &Epoch,
    params: &Params,
    mut rng: impl RngCore,
) -> SingleChunkCiphertext {
    let g1 = G1Projective::generator();

    // $r,s \leftarrow \mathbb{Z}_p$
    let rand_r = Scalar::random(&mut rng);
    let rand_s = Scalar::random(&mut rng);

    // g1^r
    let r = g1 * rand_r;
    // g1^s
    let s = g1 * rand_s;

    // can't really have a more efficient implementation until https://github.com/zkcrypto/bls12_381/pull/70 is merged...
    let c = pk.0 * rand_r + g1 * Scalar::from(*m as u64);

    // f0 * f1^t1 * ... * fi^ti * h^s
    // let z = epoch.evaluate_f(params) + params.h * rand_s;
    let z = params.h * rand_s;

    SingleChunkCiphertext { r, s, z, c }
}

fn encrypt_shares(
    shares: &[(Share, &PublicKey)],
    epoch: &Epoch,
    params: &Params,
    mut rng: impl RngCore,
) -> Ciphertext {
    let g1 = G1Projective::generator();

    // those will be relevant later for proofs of knowledge
    let mut rand_rs = Vec::with_capacity(NUM_CHUNKS);
    let mut rand_ss = Vec::with_capacity(NUM_CHUNKS);

    let mut rs = Vec::with_capacity(NUM_CHUNKS);
    let mut ss = Vec::with_capacity(NUM_CHUNKS);
    let mut zs = Vec::with_capacity(NUM_CHUNKS);

    // generate relevant re-usable pseudorandom data
    for _ in 0..NUM_CHUNKS {
        let rand_r = Scalar::random(&mut rng);
        let rand_s = Scalar::random(&mut rng);

        // g1^r
        let r = g1 * rand_r;
        // g1^s
        let s = g1 * rand_s;

        // z will require additional fancy operations to make it work 'properly', but at the current
        // step it's fine
        let z = params.h * rand_s;

        rand_rs.push(rand_r);
        rand_ss.push(rand_s);

        rs.push(r);
        ss.push(s);
        zs.push(z);
    }

    // produce per-chunk ciphertexts
    let mut cc = Vec::with_capacity(shares.len());

    for (share, pk) in shares {
        let m = share.to_chunks();

        let mut ci = Vec::with_capacity(NUM_CHUNKS);

        for (j, chunk) in m.chunks.iter().enumerate() {
            let c = pk.0 * rand_rs[j] + g1 * Scalar::from(*chunk as u64);
            ci.push(c)
        }

        // the conversion must succeed since we must have EXACTLY `NUM_CHUNKS` elements
        cc.push(ci.try_into().unwrap())
    }

    // the conversions here must also succeed since the other vecs also have `NUM_CHUNKS` elements
    Ciphertext {
        r: rs.try_into().unwrap(),
        s: ss.try_into().unwrap(),
        z: zs.try_into().unwrap(),
        ciphertext_chunks: cc,
    }

    // ciphertext data

    // let mut cs = Vec::with_capacity(NUM_CHUNKS);
    // let mut zs = Vec::with_capacity(NUM_CHUNKS);
    //
    // for chunk in m.chunks.iter() {
    //     let rand_r = Scalar::random(&mut rng);
    //     let rand_s = Scalar::random(&mut rng);
    //
    //     // g1^r
    //     let r = g1 * rand_r;
    //     // g1^s
    //     let s = g1 * rand_s;
    //
    //     let c = pk.0 * rand_r + g1 * Scalar::from(*chunk as u64);
    //     // let z = epoch.evaluate_f(params) + params.h * rand_s;
    //     let z = params.h * rand_s;
    //
    //     rand_rs.push(rand_r);
    //     rand_ss.push(rand_s);
    //
    //     rs.push(r);
    //     ss.push(s);
    //     cs.push(c);
    //     zs.push(z);
    // }
    //
    // // we know the conversions must succeed since `ShareEncryptionPlaintext` must also have `NUM_CHUNKS` chunks
    // Ciphertext {
    //     r: rs.try_into().unwrap(),
    //     s: ss.try_into().unwrap(),
    //     z: zs.try_into().unwrap(),
    //     ciphertext_chunks: vec![cs.try_into().unwrap()],
    // }
}

#[inline]
fn decrypt_chunk(
    dk: &DecryptionKey,
    r: &G1Projective,
    s: &G1Projective,
    z: &G2Projective,
    c: &G1Projective,
    epoch: &Epoch,
    lookup_table: Option<&BabyStepGiantStepLookup>,
) -> Result<Chunk, DkgError> {
    // TODO: if we go with forward secrecy then we presumably need to evolve dk

    // TODO2: verify the pairing from descryption 1.1.5, i.e. e(g1, Z_j) = e(R_j, f0*PROD(fi^ti) * e(Oj, h)

    let b_neg = dk.nodes[0].b.neg().to_affine();
    let e_neg = dk.nodes[0].e.neg().to_affine();
    let z_affine = z.to_affine();

    // M = e(C, g2) • e(R, b)^-1 • e(a, Z) • e(S, e)^-1
    // compute the miller loop separately to only perform a single final exponentiation
    let miller = bls12_381::multi_miller_loop(&[
        (&c.to_affine(), &G2_GENERATOR_PREPARED),
        (&r.to_affine(), &G2Prepared::from(b_neg)),
        (&dk.nodes[0].a.to_affine(), &G2Prepared::from(z_affine)),
        (&s.to_affine(), &G2Prepared::from(e_neg)),
    ]);
    let m = miller.final_exponentiation();

    baby_step_giant_step(&m, &PAIRING_BASE, lookup_table)
}

fn decrypt_share(
    dk: &DecryptionKey,
    // in the case of multiple receivers, specifies which index of ciphertext chunks should be used
    i: usize,
    ciphertext: &Ciphertext,
    epoch: &Epoch,
    lookup_table: Option<&BabyStepGiantStepLookup>,
) -> Result<Share, DkgError> {
    let mut plaintext = ChunkedShare::default();

    if i >= ciphertext.ciphertext_chunks.len() {
        return Err(DkgError::UnavailableCiphertext(i));
    }

    for j in 0..NUM_CHUNKS {
        plaintext.chunks[j] = decrypt_chunk(
            dk,
            &ciphertext.r[j],
            &ciphertext.s[j],
            &ciphertext.z[j],
            &ciphertext.ciphertext_chunks[i][j],
            epoch,
            lookup_table,
        )?;
    }

    plaintext.try_into()
}

pub struct BabyStepGiantStepLookup {
    base: Gt,
    m: Chunk,
    lookup: HashMap<[u8; 576], Chunk>,
}

impl BabyStepGiantStepLookup {
    pub fn precompute(base: &Gt) -> Self {
        let mut lookup = HashMap::new();
        let mut g = Gt::identity();

        // 1. m ← Ceiling(√n)
        let m = (CHUNK_MAX as f32).sqrt().ceil() as Chunk;

        // 2. For all j where 0 ≤ j < m:
        for j in 0..m {
            // Compute α^j and store the pair (j, α^j) in a table.
            lookup.insert(g.to_uncompressed(), j);
            g += base;
        }

        BabyStepGiantStepLookup {
            base: *base,
            m,
            lookup,
        }
    }

    fn try_solve(&self, target: &Gt) -> Result<Chunk, DkgError> {
        // 3. Compute α^{−m}
        let m_neg = Scalar::from(self.m as u64).neg();
        let alpha_m = self.base * m_neg;

        // 4. γ ← β. (set γ = β)
        let mut gamma = *target;

        // 5. For all i where 0 ≤ i < m:
        for i in 0..self.m {
            // 1. Check to see if γ is the second component (αj) of any pair in the table.
            if let Some(j) = self.lookup.get(&gamma.to_uncompressed()) {
                // 2. If so, return im + j.
                return Ok(i * self.m + j);
            } else {
                // 3. If not, γ ← γ • α^{−m}.
                gamma += alpha_m;
            }
        }

        Err(DkgError::UnsolvableDiscreteLog)
    }
}

impl Default for BabyStepGiantStepLookup {
    fn default() -> Self {
        BabyStepGiantStepLookup::precompute(&PAIRING_BASE)
    }
}

/// Attempts to solve the discrete log problem g^m, where g is in the Gt group and
/// m should be within the [0, CHUNK_MAX] range.
///
/// The implementation follows the following algorithm: https://en.wikipedia.org/wiki/Baby-step_giant-step#The_algorithm
///
/// # Arguments
///
/// * `target`: the result of the exponentiation, M in M = g^m,
/// * `base`: the base used for exponentiation, g in M = g^m
/// * `lookup_table`: precomputed table containing (j, α^j) pairs
pub fn baby_step_giant_step(
    target: &Gt,
    base: &Gt,
    lookup_table: Option<&BabyStepGiantStepLookup>,
) -> Result<Chunk, DkgError> {
    if let Some(lookup_table) = lookup_table {
        // compute expected m to make sure the provided lookup is valid
        let m = (CHUNK_MAX as f32).sqrt().ceil() as Chunk;

        if &lookup_table.base != base || lookup_table.lookup.len() != m as usize {
            return Err(DkgError::MismatchedLookupTable);
        }

        lookup_table.try_solve(target)
    } else {
        BabyStepGiantStepLookup::precompute(base).try_solve(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use group::Group;
    use rand_core::SeedableRng;

    #[test]
    fn baby_giant_100_without_table() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        for i in 0u64..100 {
            let base = Gt::random(&mut rng);
            let x = (rng.next_u64() + i) % CHUNK_MAX as u64;
            let target = base * Scalar::from(x);

            assert_eq!(
                baby_step_giant_step(&target, &base, None).unwrap(),
                x as Chunk
            );
        }
    }

    #[test]
    fn baby_giant_100_with_table() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let base = Gt::random(&mut rng);
        let lookup_table = BabyStepGiantStepLookup::precompute(&base);
        let table = Some(&lookup_table);

        for i in 0u64..100 {
            let x = (rng.next_u64() + i) % CHUNK_MAX as u64;
            let target = base * Scalar::from(x);

            assert_eq!(
                baby_step_giant_step(&target, &base, table).unwrap(),
                x as Chunk
            );
        }
    }

    #[test]
    fn single_chunk_decryption_100() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let (public_key, decryption_key) = keygen(&params, &mut rng);
        let epoch = Epoch::new(0);

        for i in 0u64..100 {
            let m = ((rng.next_u64() + i) % CHUNK_MAX as u64) as Chunk;
            let ciphertext = encrypt_chunk(&m, &public_key.key, &epoch, &params, &mut rng);

            let recovered = decrypt_chunk(
                &decryption_key,
                &ciphertext.r,
                &ciphertext.s,
                &ciphertext.z,
                &ciphertext.c,
                &epoch,
                Some(&DEFAULT_BSGS_TABLE),
            )
            .unwrap();
            assert_eq!(m, recovered);
        }
    }

    #[test]
    fn share_decryption_20() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let (public_key1, decryption_key1) = keygen(&params, &mut rng);
        let (public_key2, decryption_key2) = keygen(&params, &mut rng);
        let epoch = Epoch::new(0);

        let lookup_table = &DEFAULT_BSGS_TABLE;

        for _ in 0..10 {
            let m1 = Share::random(&mut rng);
            let m2 = Share::random(&mut rng);
            let shares = &[
                (m1.clone(), &public_key1.key),
                (m2.clone(), &public_key2.key),
            ];

            let ciphertext = encrypt_shares(shares, &epoch, &params, &mut rng);

            let recovered1 =
                decrypt_share(&decryption_key1, 0, &ciphertext, &epoch, Some(lookup_table))
                    .unwrap();
            let recovered2 =
                decrypt_share(&decryption_key2, 1, &ciphertext, &epoch, Some(lookup_table))
                    .unwrap();
            assert_eq!(m1, recovered1);
            assert_eq!(m2, recovered2);
        }
    }
}
