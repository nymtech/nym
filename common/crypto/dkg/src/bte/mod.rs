// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::proof_discrete_log::ProofOfDiscreteLog;
use crate::error::DkgError;
use crate::utils::hash_g2;
use crate::{Chunk, ChunkedShare, Share, CHUNK_SIZE, NUM_CHUNKS};
use bitvec::order::Msb0;
use bitvec::prelude::Lsb0;
use bitvec::vec::BitVec;
use bitvec::view::BitView;
use bitvec::{bits, bitvec};
use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt, Scalar};
use ff::Field;
use group::Curve;
use lazy_static::lazy_static;
use rand_core::RngCore;
use std::collections::{HashMap, VecDeque};
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

#[derive(Clone, Debug, PartialEq, PartialOrd)]
// None empty bitvec implies this is a root node
pub struct Tau(BitVec<u32, Msb0>);

impl Tau {
    fn new_root() -> Self {
        Tau(BitVec::new())
    }

    fn new(epoch: u32) -> Self {
        Tau(epoch.view_bits().to_bitvec())
    }

    fn push_left(&mut self) {
        self.0.push(false)
    }

    fn push_right(&mut self) {
        self.0.push(true)
    }

    fn is_leaf(&self, params: &Params) -> bool {
        self.len() == params.tree_height
    }

    fn try_get_parent_at_height(&self, height: usize) -> Result<Self, DkgError> {
        if height > self.0.len() {
            return Err(DkgError::NotAValidParent);
        }

        Ok(Tau(self.0[..height].to_bitvec()))
    }

    // essentially is this those prefixing the other
    fn is_parent_of(&self, other: &Tau) -> bool {
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

    // essentially height of the tree this tau represents
    fn len(&self) -> usize {
        self.0.len()
    }

    fn height(&self) -> usize {
        self.len()
    }

    fn extend(&self, oracle_output: &[u8]) -> Self {
        let mut extended_tau = self.clone();
        for byte in oracle_output {
            extended_tau
                .0
                .extend_from_bitslice(byte.view_bits::<Lsb0>())
        }

        extended_tau
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

impl Ciphertext {
    pub fn verify_integrity(&self) -> bool {
        true
    }
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
    // TODO: wait, what was wrong with normal Vec again?
    // nodes: VecDeque<Node>,
    // note that the nodes are ordered from "right" to "left"
    nodes: Vec<Node>,
}

impl Zeroize for DecryptionKey {
    fn zeroize(&mut self) {
        for node in self.nodes.iter_mut() {
            node.zeroize()
        }
        self.nodes.clear();
    }
}

impl Drop for DecryptionKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl DecryptionKey {
    fn new_root(root_node: Node) -> Self {
        // let mut nodes = VecDeque::new();
        // nodes.push_front(root_node);

        let mut nodes = Vec::new();
        nodes.push(root_node);

        DecryptionKey { nodes }
    }

    fn update(&mut self) {
        //
    }

    fn current(&self) -> Result<&Node, DkgError> {
        // we must have at least a single node, otherwise we have a malformed key
        self.nodes.last().ok_or(DkgError::MalformedDecryptionKey)
    }

    fn current_epoch(&self) -> Result<&Tau, DkgError> {
        self.current().map(|node| &node.tau)
    }

    /*
        fn find_prefix<'a>(dks: &'a SecretKey, tau: &[Bit]) -> Option<&'a BTENode> {
        for node in dks.bte_nodes.iter() {
            if is_prefix(&node.tau, tau) {
                return Some(node);
            }
        }
        None
    }
         */

    fn try_get_compatible_node(&self, epoch: &Tau) -> Result<&Node, DkgError> {
        self.nodes
            .iter()
            .rev()
            .find(|node| node.tau.is_parent_of(epoch))
            .ok_or(DkgError::OutdatedKey)
    }

    /// Attempts to update `self` to the provided `epoch`. If the update is not possible,
    /// because the target was in the past or the key is malformed, an error is returned.
    ///
    /// Note that this method mutates the key in place and if the original key was malformed,
    /// there are no guarantees about its internal state post-call.    
    pub fn try_update_to(
        &mut self,
        target_epoch: &Tau,
        params: &Params,
        mut rng: impl RngCore,
    ) -> Result<(), DkgError> {
        println!("updating to {}", target_epoch.0);
        if self.nodes.is_empty() {
            // somehow we have an empty decryption key
            return Err(DkgError::MalformedDecryptionKey);
        }

        if !target_epoch.is_leaf(params) {
            return Err(DkgError::MalformedEpoch);
        }

        let current_epoch = self.current_epoch()?;
        if current_epoch == target_epoch {
            // our key is already updated to the target
            return Ok(());
        }

        if current_epoch > target_epoch {
            // we cannot derive keys for past epochs
            return Err(DkgError::TargetEpochUpdateInThePast);
        }

        // drop the nodes that are no longer required and get the most direct parent for the target epoch available
        let parent = loop {
            if let Some(tail) = self.nodes.pop() {
                if tail.tau.is_parent_of(target_epoch) {
                    break tail;
                }
            } else {
                // the key is malformed since we checked that the target_epoch > current_epoch,
                // hence the update should have been possible
                return Err(DkgError::MalformedDecryptionKey);
            }
        };

        // accumulators, note that the previous elements have already been included by the parent
        // new_b_accumulator = b * d1^{tau_1} * d2^{tau_2} * ... * dn^{tau_n}
        // new_f_accumulator = f0 * f1^{tau_1} * f2^{tau_2} * ... * fn^{tau_n}
        let mut new_b_accumulator = parent.b;
        let mut new_f_accumulator = parent.tau.evaluate_f(params);

        let mut ds = parent.ds.clone();

        // let mut tau = parent.tau.clone();
        // path to the child from the parent
        for (i, bit) in target_epoch
            .0
            .iter()
            .by_vals()
            .enumerate()
            .skip(parent.tau.len())
        {
            // if the bit is NOT set..., push the right '1' subtree (for future keys)
            if !bit {
                let mut right_branch = target_epoch.try_get_parent_at_height(i)?;
                right_branch.push_right();

                // TODO: put this in node.derive_child_with_partials() because it's definitely possible
                // but first do it all in loop since its "easier". then write tests.
                let delta = Scalar::random(&mut rng);
                let a = parent.a + G1Projective::generator() * delta;

                let d0 = ds.pop_front().unwrap();
                let n = right_branch.height();
                let b = new_b_accumulator + d0 + (new_f_accumulator + params.fs[n - 1]) * delta;

                assert_eq!(d0, parent.ds[i - parent.tau.len()]);

                // TODO: CHECK FOR OFF BY ONE ERRORS HERE!!!
                // let b = new_b_accumulator
                //     + parent.ds[n - i - 1]
                //     + (new_f_accumulator + params.fs[n - 1]) * delta;
                let e = parent.e + params.h * delta;

                // TODO: CHECK FOR OFF BY ONE ERRORS HERE!!!
                let ds = ds
                    .iter()
                    .zip(params.fs.iter().skip(right_branch.height()))
                    .map(|(d_i, f_i)| d_i + f_i * delta)
                    .collect();

                let ds2 = parent
                    .ds
                    .iter()
                    .skip(i - parent.tau.len() + 1)
                    .zip(params.fs.iter().skip(right_branch.height()))
                    .map(|(d_i, f_i)| d_i + f_i * delta)
                    .collect::<Vec<_>>();

                assert_eq!(ds, ds2);

                self.nodes.push(Node {
                    tau: right_branch,
                    a,
                    b,
                    ds,
                    e,
                });
            } else {
                // only update the accumulators when the bit is set, as d^0 == identity, so there's
                // no point in doing anything else;
                // note that we don't have to generate any new nodes when going into the right branch
                // of the tree as everything on the left would have been in the past, so we don't care about them

                // TODO: CHECK FOR OFF BY ONE ERRORS HERE!!!
                // TODO: CHECK FOR OFF BY ONE ERRORS HERE!!!
                // TODO: CHECK FOR OFF BY ONE ERRORS HERE!!!
                // TODO: CHECK FOR OFF BY ONE ERRORS HERE!!!
                let d0 = ds.pop_front().unwrap();
                assert_eq!(d0, parent.ds[i - parent.tau.len()]);

                // new_b_accumulator += parent.ds[target_epoch.height() - i - 1];
                new_b_accumulator += d0;
                new_f_accumulator += params.fs[i]
            }

            // continue going to the child
            // tau.0.push(bit);
        }

        // finally derive the actual target node
        let delta = Scalar::random(&mut rng);
        let a = parent.a + G1Projective::generator() * delta;

        let n = target_epoch.height();
        // TODO: CHECK FOR OFF BY ONE ERRORS HERE!!!
        let b = new_b_accumulator + new_f_accumulator * delta;
        let e = parent.e + params.h * delta;

        // TODO: CHECK FOR OFF BY ONE ERRORS HERE!!!
        let ds = parent
            .ds
            .iter()
            .skip(n)
            .zip(params.fs.iter().skip(n))
            .map(|(d_i, f_i)| d_i + f_i * delta)
            .collect();

        self.nodes.push(Node {
            tau: target_epoch.clone(),
            a,
            b,
            ds,
            e,
        });

        Ok(())
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub(crate) struct Node {
    tau: Tau,

    // g1^rho
    a: G1Projective,

    // g2^x
    b: G2Projective,

    // f_i^rho
    // during the key update we need to be able to pop from the front of the collection,
    // so normal Vec<T> wouldn't have done the trick
    ds: VecDeque<G2Projective>,

    // h^rho
    e: G2Projective,
}

// TODO: this might go away in favour of derive if `ds` is represented with Vec as opposed to VecDeque
impl Zeroize for Node {
    fn zeroize(&mut self) {
        self.tau.zeroize();
        self.a.zeroize();
        self.b.zeroize();
        self.e.zeroize();

        for d in self.ds.iter_mut() {
            d.zeroize();
        }

        self.ds.clear();
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        self.zeroize()
    }
}

impl Node {
    fn new_root(
        a: G1Projective,
        b: G2Projective,
        ds: VecDeque<G2Projective>,
        e: G2Projective,
    ) -> Self {
        Node {
            tau: Tau::new_root(),
            a,
            b,
            ds,
            e,
        }
    }

    fn is_root(&self) -> bool {
        self.tau.0.is_empty()
    }

    fn derive_child_with_partials(
        &self,
        params: &Params,
        partial_b: &G2Projective,
        partial_f: &G2Projective,
        mut rng: impl RngCore,
    ) -> Self {
        let delta = Scalar::random(&mut rng);
        let a = self.a + G1Projective::generator() * delta;

        todo!()
    }

    // tau_l == 0
    fn derive_left_child(&self) -> Self {
        todo!()
    }

    // tau_l == 1
    fn derive_right_child(&self) -> Self {
        // this is probably missing A LOT OF arguments, but lets leave it like this temporarily

        let mut new_tau = self.tau.clone();
        new_tau.0.push(true);

        Node {
            tau: new_tau,

            // THOSE ARE WRONG BTW
            a: self.a,
            b: self.b,
            ds: self.ds.clone(),
            e: self.e,
        }
    }
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
fn keygen(params: &Params, mut rng: impl RngCore) -> (DecryptionKey, PublicKeyWithProof) {
    let g1 = G1Projective::generator();
    let g2 = G2Projective::generator();

    let mut x = Scalar::random(&mut rng);
    let y = g1 * x;

    let proof = ProofOfDiscreteLog::construct(&mut rng, &y, &x);

    let mut rho = Scalar::random(&mut rng);

    let a = g1 * rho;
    let b = g2 * x + params.f0 * rho;

    let ds = params.fs.iter().map(|f_i| f_i * rho).collect();
    let e = params.h * rho;

    let dk = DecryptionKey::new_root(Node::new_root(a, b, ds, e));

    let public_key = PublicKey(y);
    let key_with_proof = PublicKeyWithProof {
        key: public_key,
        proof,
    };

    x.zeroize();
    rho.zeroize();

    (dk, key_with_proof)
}

fn verify_key(pk: &PublicKey, proof: &ProofOfDiscreteLog) -> bool {
    proof.verify(&pk.0)
}

// evolve?
// update?
// epoch is epoch_bit I think
fn derive_key(dk: DecryptionKey, epoch: Tau) -> DecryptionKey {
    todo!()
}

#[inline]
fn encrypt_chunk(
    m: &Chunk,
    pk: &PublicKey,
    epoch: &Tau,
    params: &Params,
    mut rng: impl RngCore,
) -> SingleChunkCiphertext {
    // TODO:
    let extended_epoch = epoch;

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

    // (f0 * f1^t1 * ... * fi^ti)^r * h^s
    let z = extended_epoch.evaluate_f(params) * rand_r + params.h * rand_s;

    SingleChunkCiphertext { r, s, z, c }
}

fn encrypt_shares(
    shares: &[(Share, &PublicKey)],
    epoch: &Tau,
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

    let f = epoch.evaluate_f(params);

    // generate relevant re-usable pseudorandom data
    for _ in 0..NUM_CHUNKS {
        let rand_r = Scalar::random(&mut rng);
        let rand_s = Scalar::random(&mut rng);

        // g1^r
        let r = g1 * rand_r;
        // g1^s
        let s = g1 * rand_s;

        let z = f * rand_r + params.h * rand_s;

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
}

#[inline]
fn decrypt_chunk(
    dk: &DecryptionKey,
    r: &G1Projective,
    s: &G1Projective,
    z: &G2Projective,
    c: &G1Projective,
    epoch: &Tau,
    lookup_table: Option<&BabyStepGiantStepLookup>,
) -> Result<Chunk, DkgError> {
    // TODO: verify the pairing from description 1.1.5, i.e. e(g1, Z_j) = e(R_j, f0*PROD(fi^ti) * e(Oj, h)

    let epoch_node = dk.try_get_compatible_node(epoch)?;
    let b_neg1 = epoch
        .0
        .iter()
        .by_vals()
        .zip(epoch_node.ds.iter())
        .filter(|(i, _)| *i)
        .map(|(_, d_i)| d_i)
        .fold(epoch_node.b, |acc, d_i| acc + d_i)
        .neg()
        .to_affine();

    let b_neg = epoch_node.b.neg().to_affine();

    println!("{:?}", epoch);
    assert_eq!(b_neg1, b_neg);

    let e_neg = epoch_node.e.neg().to_affine();
    let z_affine = z.to_affine();

    // M = e(C, g2) • e(R, b)^-1 • e(a, Z) • e(S, e)^-1
    // compute the miller loop separately to only perform a single final exponentiation
    let miller = bls12_381::multi_miller_loop(&[
        (&c.to_affine(), &G2_GENERATOR_PREPARED),
        (&r.to_affine(), &G2Prepared::from(b_neg)),
        (&epoch_node.a.to_affine(), &G2Prepared::from(z_affine)),
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
    epoch: &Tau,
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
        let m = (CHUNK_SIZE as f32).sqrt().ceil() as Chunk;

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
        let m = (CHUNK_SIZE as f32).sqrt().ceil() as Chunk;

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
    use bitvec::order::Msb0;
    use group::Group;
    use rand_core::SeedableRng;

    #[test]
    fn baby_giant_100_without_table() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        for i in 0u64..100 {
            let base = Gt::random(&mut rng);
            let x = (rng.next_u64() + i) % CHUNK_SIZE as u64;
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
            let x = (rng.next_u64() + i) % CHUNK_SIZE as u64;
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

        let (decryption_key, public_key) = keygen(&params, &mut rng);
        let epoch = Tau::new(0);

        for i in 0u64..100 {
            let m = ((rng.next_u64() + i) % CHUNK_SIZE as u64) as Chunk;
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

        let (decryption_key1, public_key1) = keygen(&params, &mut rng);
        let (decryption_key2, public_key2) = keygen(&params, &mut rng);
        let epoch = Tau::new(0);

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

    #[test]
    fn share_encryption_under_nonzero_epoch() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let (mut decryption_key1, public_key1) = keygen(&params, &mut rng);
        let (mut decryption_key2, public_key2) = keygen(&params, &mut rng);
        let epoch = Tau::new(12345);
        decryption_key1
            .try_update_to(&epoch, &params, &mut rng)
            .unwrap();
        decryption_key2
            .try_update_to(&epoch, &params, &mut rng)
            .unwrap();

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
        let expected = bitvec![
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
    fn basic_coverage_nodes() {
        // it's some basic test I've been performing when writing the update function, but figured
        // might as well put it into a unit test. note that it doesn't check the entire structure,
        // but just the few last nodes of low height

        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (mut dk, _) = keygen(&params, &mut rng);

        let root_node_copy = dk.nodes.clone();

        // this is a root node
        assert_eq!(dk.nodes.len(), 1);
        assert!(dk.nodes[0].is_root());

        // we have to have a node for right branch on each height (1, 01, 001, ... etc)
        // plus an additional one for the two left-most leaves (epochs "0" and "1")
        dk.try_update_to(&Tau::new(0), &params, &mut rng).unwrap();
        assert_eq!(dk.nodes.len(), 33);

        let expected_last = Tau::new(0);
        // (and yes, I had to look up those names in a thesaurus)
        let expected_penultimate = Tau::new(1);
        // note that this value is 31bit long
        let expected_antepenultimate = Tau(bitvec![u32, Msb0;
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1
        ]);

        let mut nodes_iter = dk.nodes.iter().rev();
        assert_eq!(expected_last, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_penultimate, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_antepenultimate, nodes_iter.next().unwrap().tau);

        let mut epoch_zero_nodes = dk.nodes.clone();

        // nodes for epoch1 should be identical for those for epoch0 minus the 00..00 leaf
        dk.try_update_to(&Tau::new(1), &params, &mut rng).unwrap();
        assert_eq!(dk.nodes.len(), 32);
        epoch_zero_nodes.pop().unwrap();
        assert_eq!(
            epoch_zero_nodes
                .iter()
                .map(|node| node.tau.clone())
                .collect::<Vec<_>>(),
            dk.nodes
                .iter()
                .map(|node| node.tau.clone())
                .collect::<Vec<_>>()
        );

        dk.try_update_to(&Tau::new(2), &params, &mut rng).unwrap();
        dk.try_update_to(&Tau::new(3), &params, &mut rng).unwrap();
        dk.try_update_to(&Tau::new(4), &params, &mut rng).unwrap();

        let expected_last = Tau::new(4);
        let expected_penultimate = Tau::new(5);
        let expected_antepenultimate = Tau(bitvec![u32, Msb0;
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1
        ]);
        let expected_preantepenultimate = Tau(bitvec![u32, Msb0;
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1
        ]);
        assert_eq!(dk.nodes.len(), 32);
        let mut nodes_iter = dk.nodes.iter().rev();
        assert_eq!(expected_last, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_penultimate, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_antepenultimate, nodes_iter.next().unwrap().tau);
        assert_eq!(expected_preantepenultimate, nodes_iter.next().unwrap().tau);

        // the result should be the same of regardless if we update incrementally or go to the target immediately
        let mut new_root = DecryptionKey {
            nodes: root_node_copy,
        };
        new_root
            .try_update_to(&Tau::new(4), &params, &mut rng)
            .unwrap();
        assert_eq!(
            dk.nodes
                .iter()
                .map(|node| node.tau.clone())
                .collect::<Vec<_>>(),
            new_root
                .nodes
                .iter()
                .map(|node| node.tau.clone())
                .collect::<Vec<_>>()
        );

        // getting expected nodes for those epochs is non-trivial for test purposes, but the last node
        // should ALWAYS be equal to the target epoch
        dk.try_update_to(&Tau::new(42), &params, &mut rng).unwrap();
        assert_eq!(dk.nodes.last().unwrap().tau, Tau::new(42));
        dk.try_update_to(&Tau::new(123456), &params, &mut rng)
            .unwrap();
        assert_eq!(dk.nodes.last().unwrap().tau, Tau::new(123456));
        dk.try_update_to(&Tau::new(3292547435), &params, &mut rng)
            .unwrap();
        assert_eq!(dk.nodes.last().unwrap().tau, Tau::new(3292547435));

        // trying to go to past epochs fails
        assert!(dk.try_update_to(&Tau::new(531), &params, &mut rng).is_err())
    }
}

// TODO: write a sanity-check test to see if root node allows you to decrypt everything

// TODO: benchmark whether for key updates it's quicker to do a^delta as opposed to a + g1^delta (same for b, d, e, etc.)
