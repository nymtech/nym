// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::encryption::Ciphertexts;
use crate::bte::{Chunk, PublicKey, Share, CHUNK_SIZE, NUM_CHUNKS};
use crate::ensure_len;
use crate::error::DkgError;
use crate::utils::{deserialize_g1, hash_to_scalar};
use crate::utils::{deserialize_scalar, RandomOracleBuilder};
use bls12_381::{G1Projective, Scalar};
use ff::Field;
use group::{Group, GroupEncoding};
use rand::Rng;
use rand_core::{RngCore, SeedableRng};

const CHUNKING_ORACLE_DOMAIN: &[u8] =
    b"NYM_COCONUT_NIDKG_V01_CS01_SHA-256_CHACHA20_CHUNKING_ORACLE";

const SECOND_CHALLENGE_DOMAIN: &[u8] =
    b"NYM_COCONUT_NIDKG_V01_CS01_WITH_BLS12381_XMD:SHA-256_SSWU_RO_PROOF_CHUNKING_SECOND_CHALLENGE";

/// The number of parallel runs batched in a single challenge,
/// `l` ($\ell$) in the DKG paper.
const PARALLEL_RUNS: usize = 32;

/// `lambda` ($\lambda$) in the DKG paper
const SECURITY_PARAMETER: usize = 256;

// note: ceiling in integer division can be achieved via q = (x + y - 1) / y;
/// ceil(SECURITY_PARAMETER / PARALLEL_RUNS) in the paper
const NUM_CHALLENGE_BITS: usize = (SECURITY_PARAMETER + PARALLEL_RUNS - 1) / PARALLEL_RUNS;

// type alias for ease of use
type FirstChallenge = Vec<Vec<Vec<u64>>>;

#[cfg_attr(test, derive(Clone))]
pub struct Instance<'a> {
    /// y_1, ..., y_n
    public_keys: &'a [PublicKey],

    /// R_1, ..., R_m
    rr: &'a [G1Projective; NUM_CHUNKS],

    /// C_{1,1}, ..., C_{n,m}
    ciphertext_chunks: &'a [[G1Projective; NUM_CHUNKS]],
}

impl<'a> Instance<'a> {
    pub fn new(public_keys: &'a [PublicKey], ciphertext: &'a Ciphertexts) -> Instance<'a> {
        Instance {
            public_keys,
            rr: &ciphertext.rr,
            ciphertext_chunks: &ciphertext.ciphertext_chunks,
        }
    }

    fn validate(&self) -> bool {
        if self.public_keys.is_empty() {
            return false;
        }

        if self.public_keys.len() != self.ciphertext_chunks.len() {
            return false;
        }

        true
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub struct ProofOfChunking {
    y0: G1Projective,
    bb: Vec<G1Projective>,
    cc: Vec<G1Projective>,
    dd: Vec<G1Projective>,
    yy: G1Projective,
    responses_r: Vec<Scalar>,
    responses_chunks: Vec<u64>,
    response_beta: Scalar,
}

impl ProofOfChunking {
    // Some decisions made in this function code can be questionable, however, I will attempt to justify them.
    // I decided to operate on u64 when constructing responses for the chunks rather than using
    // `Scalar` directly that could have future-proofed everything in case we decided to significantly
    // increase our chunk sizes alongside PARALLEL_RUNS, number of nodes, etc. such that it would no longer
    // fit within u64. However, this is not very likely and runtime checks are in place to ensure it hasn't happened.
    //
    // As for the actual issue, at the time of writing this code, there is no way of ordering two Scalars.
    // And conceptually it makes sense as ordering of scalars in finite fields doesn't make much
    // sense mathematically. Anyway, in my original proof of concept code I had to compare byte representations
    // of the scalars. While in principle it still makes sense (assuming they're in the canonical representations),
    // we run into a problem if say we wanted to compare Scalar(1) with Scalar(-1). Logically the -1 variant
    // should have been smaller. However, internally everything is represented mod field order and thus
    // Scalar(-1) would in reality be Scalar(q - 1), which is greater than Scalar(1) and opposite to
    // what we wanted.
    pub fn construct(
        mut rng: impl RngCore,
        instance: Instance,
        witness_r: &[Scalar; NUM_CHUNKS],
        witnesses_s: &[Share],
    ) -> Result<Self, DkgError> {
        if !instance.validate() {
            return Err(DkgError::MalformedProofOfChunkingInstance);
        }

        let g1 = G1Projective::generator();

        let y0 = G1Projective::random(&mut rng);

        // define bounds for the blinding factors
        let n = instance.public_keys.len();
        let m = NUM_CHUNKS;
        let ee = 1 << NUM_CHALLENGE_BITS;

        // CHUNK_MAX corresponds to paper's B
        let ss = (n * m * (CHUNK_SIZE - 1) * (ee - 1)) as u64;
        let zz = (2 * (PARALLEL_RUNS as u64))
            .checked_mul(ss)
            .expect("overflow in Z = 2 * l * S");

        let ss_scalar = Scalar::from(ss);

        // rather than generating blinding factors in [-S, Z-1] directly,
        // do it via [0, Z - 1 + S + 1] and deal with the shift later.
        let combined_upper_range = (zz - 1)
            .checked_add(ss + 1)
            .expect("overflow in Z - 1 + S + 1");

        let mut betas = Vec::with_capacity(PARALLEL_RUNS);
        let mut bs = Vec::with_capacity(PARALLEL_RUNS);

        for _ in 0..PARALLEL_RUNS {
            let beta = Scalar::random(&mut rng);

            // g1 ^ beta_l
            let bb = g1 * beta;

            betas.push(beta);
            bs.push(bb);
        }

        let mut attempt = 0;
        let (first_challenge, responses_chunks, cs) = 'retry_loop: loop {
            attempt += 1;
            if attempt == SECURITY_PARAMETER {
                return Err(DkgError::AbortedProofOfChunking);
            }

            // let mut blinding_factors = Vec::with_capacity(PARALLEL_RUNS);
            let mut shifted_blinding_factors = Vec::with_capacity(PARALLEL_RUNS);
            let mut cs = Vec::with_capacity(PARALLEL_RUNS);

            // I think this part is more readable with a range loop
            #[allow(clippy::needless_range_loop)]
            for i in 0..PARALLEL_RUNS {
                // scalar in range of [0, Z - 1 + S]
                let shifted = rng.gen_range(0..=combined_upper_range);
                // [-S, Z - 1] as required
                let blinding_factor = Scalar::from(shifted) - ss_scalar;

                // y0 ^ beta_l • g1 ^ sigma_l
                let cc = y0 * betas[i] + g1 * blinding_factor;

                // blinding_factors.push(blinding_factor);
                shifted_blinding_factors.push(shifted);
                cs.push(cc);
            }

            let first_challenge = Self::compute_first_challenge(&instance, &y0, &bs, &cs, n, m);

            // compute:
            // z_{s,1} = sum(i <- 1..n; (sum j <- j..m; e_{i,j,1} • s_{i,j} + sigma_1))
            // ...
            // z_{s,l} = sum(i <- 1..n; (sum j <- j..m; e_{i,j,l} • s_{i,j} + sigma_l))
            // such that 0 <= z_{s,l} < Z
            let mut responses_chunks = Vec::with_capacity(PARALLEL_RUNS);

            // I think this part is more readable with a range loop
            #[allow(clippy::needless_range_loop)]
            for l in 0..PARALLEL_RUNS {
                let mut sum = 0;

                for (i, witness_i) in witnesses_s.iter().enumerate() {
                    for (j, witness_ij) in witness_i.to_chunks().chunks.iter().enumerate() {
                        debug_assert!(std::mem::size_of::<Chunk>() <= std::mem::size_of::<u64>());
                        sum += first_challenge[i][j][l] * (*witness_ij as u64)
                    }
                }

                if sum + shifted_blinding_factors[l] < ss {
                    continue 'retry_loop;
                }
                // shifted_blinding_factors[l] - ss restores it to "proper" [-S, Z - 1] range
                let response = sum + shifted_blinding_factors[l] - ss;
                if response < zz {
                    responses_chunks.push(response)
                } else {
                    continue 'retry_loop;
                }
            }

            break (first_challenge, responses_chunks, cs);
        };

        let mut deltas = Vec::with_capacity(n + 1);
        let mut ds = Vec::with_capacity(n + 1);

        for _ in 0..=n {
            let delta = Scalar::random(&mut rng);
            let dd = g1 * delta;

            deltas.push(delta);
            ds.push(dd);
        }

        //  Y = y_0 ^ delta_0 • y_1 ^ delta_1 • ... • y_n ^ delta_n
        let mut yy = y0 * deltas[0];
        for (i, delta_i) in deltas.iter().enumerate().skip(1) {
            yy += instance.public_keys[i - 1].0 * delta_i;
        }

        let second_challenge =
            Self::compute_second_challenge(&first_challenge, &responses_chunks, &ds, &yy);

        // compute responses

        let mut responses_r = Vec::with_capacity(n);

        for (i, e_i) in first_challenge.iter().enumerate() {
            let mut response_r_k = deltas[i + 1];
            for (j, e_ij) in e_i.iter().enumerate() {
                // c^1 in first iteration, c^k in the last
                let mut challenge_pow = second_challenge;
                for e_ijk in e_ij.iter() {
                    response_r_k += Scalar::from(*e_ijk) * witness_r[j] * challenge_pow;
                    challenge_pow *= second_challenge
                }
            }

            responses_r.push(response_r_k);
        }

        let mut response_beta = deltas[0];
        // c^1 in first iteration, c^k in the last
        let mut challenge_pow = second_challenge;
        for beta_k in betas {
            // r_beta = beta_1 • challenge^1 + beta_2 • challenge^2 + ... beta_k • challenge^k + delta_0
            response_beta += beta_k * challenge_pow;
            challenge_pow *= second_challenge
        }

        Ok(ProofOfChunking {
            y0,
            bb: bs,
            cc: cs,
            dd: ds,
            yy,
            responses_r,
            responses_chunks,
            response_beta,
        })
    }

    pub fn verify(&self, instance: Instance) -> bool {
        if !instance.validate() {
            return false;
        }

        let g1 = G1Projective::generator();
        let n = instance.public_keys.len();
        let m = instance.rr.len();

        ensure_len!(&self.bb, PARALLEL_RUNS);
        ensure_len!(&self.cc, PARALLEL_RUNS);
        ensure_len!(&self.dd, n + 1);
        ensure_len!(&self.responses_r, n);
        ensure_len!(&self.responses_chunks, PARALLEL_RUNS);

        let ee = 1 << NUM_CHALLENGE_BITS;

        // CHUNK_MAX corresponds to paper's B
        let ss = (n * m * (CHUNK_SIZE - 1) * (ee - 1)) as u64;
        let zz = 2 * (PARALLEL_RUNS as u64) * ss;

        for response_chunk in &self.responses_chunks {
            if response_chunk >= &zz {
                return false;
            }
        }

        let first_challenge =
            Self::compute_first_challenge(&instance, &self.y0, &self.bb, &self.cc, n, m);

        let second_challenge = Self::compute_second_challenge(
            &first_challenge,
            &self.responses_chunks,
            &self.dd,
            &self.yy,
        );

        // memoise chlg^k for k in 1..l since they're used in multiple checks, so there's
        // no point in recomputing them every time
        let mut challenge_pows = Vec::with_capacity(PARALLEL_RUNS);
        challenge_pows.push(second_challenge);
        for k in 1..PARALLEL_RUNS {
            challenge_pows.push(challenge_pows[k - 1] * second_challenge)
        }

        // for i in [1..n] check if:
        // R_1 ^ (e_{i,1,1} * chlg^1 + ... + e_{i,1,l} * chlg^l) • ... • R_m ^ (e_{i,m,1} * chlg^1 + ... + e_{i,m,l} * chlg^l) • D_i
        // ==
        // g1^reponses_r_i
        for (i, response_r_i) in self.responses_r.iter().enumerate() {
            // rhs = D_i
            let mut product = self.dd[i + 1];
            for (j, e_ij) in first_challenge[i].iter().enumerate() {
                // intermediate (e_{i,j,1} * chlg^1 + ... + e_{i,j,l} * chlg^l) sum
                let mut sum = Scalar::zero();
                for (k, e_ijk) in e_ij.iter().enumerate() {
                    sum += Scalar::from(*e_ijk) * challenge_pows[k]
                }
                // for j in [1..m]
                // rhs = D_i • R_1 ^ sum_1 • ... • R_m ^ sum_m
                product += instance.rr[j] * sum
            }

            if product != g1 * response_r_i {
                return false;
            }
        }

        // check if B_{1}^{chlg^1} • ... • B_{l}^{chlg^l} • D_0 == g1^{response_beta}
        let mut product = self.dd[0];
        for (k, b_k) in self.bb.iter().enumerate() {
            product += b_k * challenge_pows[k]
        }

        if product != g1 * self.response_beta {
            return false;
        }

        // check if
        // (C_{1,1} ^ e_{1,1,1} • ... • C_{n,m} ^ e_{n,m,1}) ^ chlg^1 • ... (C_{1,1} ^ e_{1,1,l} • ... • C_{n,m} ^ e_{n,m,l}) ^ chlg^l  • Y
        // ==
        // pk_1 ^ responses_r_1 • ... • pk_n ^ responses_r_n • y_0^{response_beta} • g1^{response_chunks_1 • chlg^1 + ... + response_chunks_l • chlg^l}

        let mut lhs = self.yy;

        // compute product (C_{1,1} ^ e_{1,1,1} • ... • C_{n,m} ^ e_{n,m,1}) ^ chlg^1 • ... (C_{1,1} ^ e_{1,1,l} • ... • C_{n,m} ^ e_{n,m,l}) ^ chlg^l
        // I think this part is more readable with a range loop
        #[allow(clippy::needless_range_loop)]
        for k in 0..PARALLEL_RUNS {
            let mut inner_acc = G1Projective::identity();
            for (i, c_i) in instance.ciphertext_chunks.iter().enumerate() {
                for (j, c_ij) in c_i.iter().enumerate() {
                    inner_acc += c_ij * Scalar::from(first_challenge[i][j][k]);
                }
            }
            // TODO: can this be simplified?
            inner_acc *= challenge_pows[k];
            lhs += inner_acc
        }

        // finally multiply by C_{1}^{chlg^1} • ... • C_{l}^{chlg^l}
        for (k, c) in self.cc.iter().enumerate() {
            lhs += c * challenge_pows[k]
        }

        // calculate intermediate product pk_1 ^ responses_r_1 • ... • pk_n ^ responses_r_n
        let mut product = G1Projective::identity();
        for (i, pk) in instance.public_keys.iter().enumerate() {
            product += pk.0 * self.responses_r[i]
        }

        // calculate intermediate sum response_chunks_1 • chlg^1 + ... + response_chunks_l • chlg^l
        let mut sum = Scalar::zero();
        for (k, response_chunk) in self.responses_chunks.iter().enumerate() {
            sum += Scalar::from(*response_chunk) * challenge_pows[k]
        }

        let rhs = product + self.y0 * self.response_beta + g1 * sum;

        if lhs != rhs {
            return false;
        }

        true
    }

    fn compute_first_challenge(
        instance: &Instance,
        y0: &G1Projective,
        bb: &[G1Projective],
        cc: &[G1Projective],
        n: usize,
        m: usize,
    ) -> FirstChallenge {
        // lambda_e = n • m • l • ceil(lambda / l)
        let lambda_e = n * m * PARALLEL_RUNS * NUM_CHALLENGE_BITS;

        let mut random_oracle_builder = RandomOracleBuilder::new(CHUNKING_ORACLE_DOMAIN);
        random_oracle_builder.update_with_g1_elements(instance.public_keys.iter().map(|pk| &pk.0));
        random_oracle_builder.update_with_g1_elements(instance.rr.iter());
        instance
            .ciphertext_chunks
            .iter()
            .for_each(|chunks| random_oracle_builder.update_with_g1_elements(chunks.iter()));
        random_oracle_builder.update(y0.to_bytes());
        random_oracle_builder.update_with_g1_elements(bb.iter());
        random_oracle_builder.update_with_g1_elements(cc.iter());
        random_oracle_builder.update(lambda_e.to_be_bytes());

        let mut oracle = rand_chacha::ChaCha20Rng::from_seed(random_oracle_builder.finalize());
        let range_max_excl = 1 << NUM_CHALLENGE_BITS;

        (0..n)
            .map(|_| {
                (0..m)
                    .map(|_| {
                        (0..PARALLEL_RUNS)
                            .map(|_| oracle.gen_range(0..range_max_excl))
                            .collect()
                    })
                    .collect()
            })
            .collect()
    }

    fn compute_second_challenge(
        first_challenge: &FirstChallenge,
        responses_chunks: &[u64],
        ds: &[G1Projective],
        y: &G1Projective,
    ) -> Scalar {
        let scalars =
            first_challenge.len() * first_challenge[0].len() * first_challenge[0][0].len()
                + responses_chunks.len();
        let g1s = ds.len() + 1;

        let mut bytes = Vec::with_capacity(scalars * 32 + g1s * 48);

        for e_i in first_challenge {
            for e_ij in e_i {
                for e_ijk in e_ij {
                    bytes.extend_from_slice(e_ijk.to_be_bytes().as_ref());
                }
            }
        }

        for z in responses_chunks {
            bytes.extend_from_slice(z.to_be_bytes().as_ref())
        }

        for d in ds {
            bytes.extend_from_slice(d.to_bytes().as_ref())
        }

        bytes.extend_from_slice(y.to_bytes().as_ref());

        hash_to_scalar(bytes, SECOND_CHALLENGE_DOMAIN)
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let g1s = self.bb.len() + self.cc.len() + self.dd.len() + 2;
        let scalars = self.responses_r.len() + 1;
        let u64s = self.responses_chunks.len();

        // we also need length indicators for bb, cc, dd, responses_r and responses_chunks
        let mut bytes = Vec::with_capacity(g1s * 48 + scalars * 32 + u64s * 8 + 5 * 4);
        bytes.extend_from_slice(self.y0.to_bytes().as_ref());

        bytes.extend_from_slice(&(self.bb.len() as u32).to_be_bytes());
        for b in &self.bb {
            bytes.extend_from_slice(b.to_bytes().as_ref());
        }

        bytes.extend_from_slice(&(self.cc.len() as u32).to_be_bytes());
        for c in &self.cc {
            bytes.extend_from_slice(c.to_bytes().as_ref());
        }

        bytes.extend_from_slice(&(self.dd.len() as u32).to_be_bytes());
        for d in &self.dd {
            bytes.extend_from_slice(d.to_bytes().as_ref());
        }

        bytes.extend_from_slice(self.yy.to_bytes().as_ref());

        bytes.extend_from_slice(&(self.responses_r.len() as u32).to_be_bytes());
        for rr in &self.responses_r {
            bytes.extend_from_slice(rr.to_bytes().as_ref());
        }

        bytes.extend_from_slice(&(self.responses_chunks.len() as u32).to_be_bytes());
        for rc in &self.responses_chunks {
            bytes.extend_from_slice(rc.to_be_bytes().as_ref());
        }

        bytes.extend_from_slice(self.response_beta.to_bytes().as_ref());

        bytes
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        // determining the minimum number of bytes is tricky, so we'll be checking if we have enough as we go

        // can we read y0 and length of bb?
        if bytes.len() < 48 + 4 {
            return Err(DkgError::new_deserialization_failure(
                "ProofOfChunking",
                "insufficient number of bytes provided",
            ));
        }

        let mut i = 0;
        let y0 = deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
            DkgError::new_deserialization_failure("ProofOfChunking.y0", "invalid curve point")
        })?;
        i += 48;

        let bb_len = u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;

        // can we read bb and length of cc?
        if bytes[i..].len() < 48 * bb_len + 4 {
            return Err(DkgError::new_deserialization_failure(
                "ProofOfChunking",
                "insufficient number of bytes provided",
            ));
        }

        let mut bb = Vec::with_capacity(bb_len);
        for _ in 0..bb_len {
            bb.push(deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
                DkgError::new_deserialization_failure("ProofOfChunking.bb", "invalid curve point")
            })?);
            i += 48;
        }

        let cc_len = u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;

        // can we read cc and length of dd?
        if bytes[i..].len() < 48 * cc_len + 4 {
            return Err(DkgError::new_deserialization_failure(
                "ProofOfChunking",
                "insufficient number of bytes provided",
            ));
        }

        let mut cc = Vec::with_capacity(cc_len);
        for _ in 0..cc_len {
            cc.push(deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
                DkgError::new_deserialization_failure("ProofOfChunking.cc", "invalid curve point")
            })?);
            i += 48;
        }

        let dd_len = u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;

        // can we read dd, yy and length of responses_r?
        if bytes[i..].len() < 48 * dd_len + 48 + 4 {
            return Err(DkgError::new_deserialization_failure(
                "ProofOfChunking",
                "insufficient number of bytes provided",
            ));
        }

        let mut dd = Vec::with_capacity(dd_len);
        for _ in 0..dd_len {
            dd.push(deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
                DkgError::new_deserialization_failure("ProofOfChunking.dd", "invalid curve point")
            })?);
            i += 48;
        }

        let yy = deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
            DkgError::new_deserialization_failure("ProofOfChunking.y0", "invalid curve point")
        })?;
        i += 48;

        let responses_r_len = u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;

        // can we read responses_r and length of responses_chunks?
        if bytes[i..].len() < 32 * responses_r_len + 4 {
            return Err(DkgError::new_deserialization_failure(
                "ProofOfChunking",
                "insufficient number of bytes provided",
            ));
        }

        let mut responses_r = Vec::with_capacity(responses_r_len);
        for _ in 0..responses_r_len {
            responses_r.push(deserialize_scalar(&bytes[i..i + 32]).ok_or_else(|| {
                DkgError::new_deserialization_failure(
                    "ProofOfChunking.responses_r",
                    "invalid scalar",
                )
            })?);
            i += 32;
        }

        let responses_chunks_len =
            u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;

        // can we read the rest of the proof, i.e. responses_chunks and response_beta?
        if bytes[i..].len() != responses_chunks_len * 8 + 32 {
            return Err(DkgError::new_deserialization_failure(
                "ProofOfChunking",
                "invalid number of bytes provided",
            ));
        }

        let mut responses_chunks = Vec::with_capacity(responses_chunks_len);
        for _ in 0..responses_chunks_len {
            responses_chunks.push(u64::from_be_bytes((&bytes[i..i + 8]).try_into().unwrap()));
            i += 8;
        }

        let response_beta = deserialize_scalar(&bytes[i..i + 32]).ok_or_else(|| {
            DkgError::new_deserialization_failure("ProofOfChunking.response_beta", "invalid scalar")
        })?;

        Ok(ProofOfChunking {
            y0,
            bb,
            cc,
            dd,
            yy,
            responses_r,
            responses_chunks,
            response_beta,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bte::Share;
    use crate::ChunkedShare;

    // limit number of nodes to some reasonable-ish value, as it significantly affects
    // time it takes to compute and verify the proof
    const NODES: usize = 20;

    struct OwnedInstance {
        public_keys: Vec<PublicKey>,
        randomizers_r: [G1Projective; NUM_CHUNKS],
        ciphertext_chunks: Vec<[G1Projective; NUM_CHUNKS]>,
    }

    fn setup(mut rng: impl RngCore) -> (OwnedInstance, [Scalar; NUM_CHUNKS], Vec<Share>) {
        let g1 = G1Projective::generator();

        let mut pks = Vec::with_capacity(NODES);

        for _ in 0..NODES {
            pks.push(PublicKey(g1 * Scalar::random(&mut rng)));
        }

        let mut r = Vec::with_capacity(NUM_CHUNKS);
        let mut rr = Vec::with_capacity(NUM_CHUNKS);

        for _ in 0..NUM_CHUNKS {
            let r_i = Scalar::random(&mut rng);
            rr.push(g1 * r_i);
            r.push(r_i);
        }

        let mut ciphertext_chunks = Vec::with_capacity(NODES);
        let mut shares = Vec::with_capacity(NODES);

        for pk_i in &pks {
            let share = Share::random(&mut rng);

            let mut ciphertext_chunk_i = Vec::with_capacity(NUM_CHUNKS);
            for (j, chunk) in share.to_chunks().chunks.iter().enumerate() {
                let c = pk_i.0 * r[j] + g1 * Scalar::from(*chunk as u64);
                ciphertext_chunk_i.push(c)
            }

            ciphertext_chunks.push(ciphertext_chunk_i.try_into().unwrap());
            shares.push(share);
        }

        (
            OwnedInstance {
                public_keys: pks,
                randomizers_r: rr.try_into().unwrap(),
                ciphertext_chunks,
            },
            r.try_into().unwrap(),
            shares,
        )
    }

    #[test]
    fn should_fail_to_create_proof_with_invalid_instance() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (owned_instance, _, _) = setup(&mut rng);
        let good_instance = Instance {
            public_keys: &owned_instance.public_keys,
            rr: &owned_instance.randomizers_r,
            ciphertext_chunks: &owned_instance.ciphertext_chunks,
        };

        // sanity check
        assert!(good_instance.validate());

        // no keys
        let mut bad_instance = good_instance.clone();
        bad_instance.public_keys = &[];
        assert!(!bad_instance.validate());

        // too many keys
        let mut bad_keys = owned_instance.public_keys.clone();
        bad_keys.push(PublicKey(
            G1Projective::generator() * Scalar::random(&mut rng),
        ));

        let mut bad_instance = good_instance.clone();
        bad_instance.public_keys = &bad_keys;
        assert!(!bad_instance.validate());

        // too few keys
        let mut bad_keys = owned_instance.public_keys.clone();
        bad_keys.truncate(bad_keys.len() - 1);

        let mut bad_instance = good_instance.clone();
        bad_instance.public_keys = &bad_keys;
        assert!(!bad_instance.validate());

        // no ciphertexts
        let mut bad_instance = good_instance.clone();
        bad_instance.ciphertext_chunks = &[];
        assert!(!bad_instance.validate());

        // too many ciphertexts
        let mut bad_ciphertexts = owned_instance.ciphertext_chunks.clone();
        bad_ciphertexts.push(Default::default());

        let mut bad_instance = good_instance.clone();
        bad_instance.ciphertext_chunks = &bad_ciphertexts;
        assert!(!bad_instance.validate());

        // too few ciphertexts
        let mut bad_ciphertexts = owned_instance.ciphertext_chunks.clone();
        bad_ciphertexts.truncate(bad_ciphertexts.len() - 1);

        let mut bad_instance = good_instance.clone();
        bad_instance.ciphertext_chunks = &bad_ciphertexts;
        assert!(!bad_instance.validate());
    }

    #[test]
    fn should_verify_a_valid_proof() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (owned_instance, r, shares) = setup(&mut rng);

        let instance = Instance {
            public_keys: &owned_instance.public_keys,
            rr: &owned_instance.randomizers_r,
            ciphertext_chunks: &owned_instance.ciphertext_chunks,
        };

        let chunking_proof =
            ProofOfChunking::construct(&mut rng, instance.clone(), &r, &shares).unwrap();

        assert!(chunking_proof.verify(instance))
    }

    #[test]
    fn works_with_chunks_of_extreme_sizes() {
        // Note: by extreme I mean CHUNK_MAX or 0
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let g1 = G1Projective::generator();

        let mut pks = Vec::with_capacity(3);

        for _ in 0..3 {
            pks.push(PublicKey(g1 * Scalar::random(&mut rng)));
        }

        let mut r = Vec::with_capacity(NUM_CHUNKS);
        let mut rr = Vec::with_capacity(NUM_CHUNKS);

        for _ in 0..NUM_CHUNKS {
            let r_i = Scalar::random(&mut rng);
            rr.push(g1 * r_i);
            r.push(r_i);
        }

        let mut ciphertext_chunks = Vec::with_capacity(3);

        let share1 = Share(Scalar::zero());
        let share2 = Share(Scalar::one());

        let chunks1 = share1.to_chunks();
        let chunks2 = share2.to_chunks();

        // note that we can't have just [Chunk::MAX; NUM_CHUNKS] as it cannot be converted
        // back to scalar since its byte representation would be just 1s and it's not a canonical
        // Scalar reduced mod q
        let chunks3 = ChunkedShare {
            chunks: [
                1,
                Chunk::MAX,
                3,
                4,
                Chunk::MAX,
                Chunk::MAX - 1,
                0,
                0,
                0,
                0,
                42,
                0,
                0,
                Chunk::MAX,
                0,
                0,
            ],
        };

        let share3 = chunks3.clone().try_into().unwrap();

        let shares = vec![share1, share2, share3];
        let chunks = vec![chunks1, chunks2, chunks3];

        for (i, pk_i) in pks.iter().enumerate() {
            let mut ciphertext_chunk_i = Vec::with_capacity(NUM_CHUNKS);
            for (j, chunk) in chunks[i].chunks.iter().enumerate() {
                let c = pk_i.0 * r[j] + g1 * Scalar::from(*chunk as u64);
                ciphertext_chunk_i.push(c)
            }

            ciphertext_chunks.push(ciphertext_chunk_i.try_into().unwrap());
        }

        let randomizers_r = rr.try_into().unwrap();
        let witness_r = r.try_into().unwrap();

        let instance = Instance {
            public_keys: &pks,
            rr: &randomizers_r,
            ciphertext_chunks: &ciphertext_chunks,
        };

        let chunking_proof =
            ProofOfChunking::construct(&mut rng, instance.clone(), &witness_r, &shares).unwrap();

        assert!(chunking_proof.verify(instance))
    }

    #[test]
    fn should_fail_to_verify_proof_with_invalid_instance() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (owned_instance, r, shares) = setup(&mut rng);
        let good_instance = Instance {
            public_keys: &owned_instance.public_keys,
            rr: &owned_instance.randomizers_r,
            ciphertext_chunks: &owned_instance.ciphertext_chunks,
        };

        let chunking_proof =
            ProofOfChunking::construct(&mut rng, good_instance.clone(), &r, &shares).unwrap();

        // no keys
        let mut bad_instance = good_instance.clone();
        bad_instance.public_keys = &[];
        assert!(!chunking_proof.verify(bad_instance));

        // too many keys
        let mut bad_keys = owned_instance.public_keys.clone();
        bad_keys.push(PublicKey(
            G1Projective::generator() * Scalar::random(&mut rng),
        ));

        let mut bad_instance = good_instance.clone();
        bad_instance.public_keys = &bad_keys;
        assert!(!chunking_proof.verify(bad_instance));

        // too few keys
        let mut bad_keys = owned_instance.public_keys.clone();
        bad_keys.truncate(bad_keys.len() - 1);

        let mut bad_instance = good_instance.clone();
        bad_instance.public_keys = &bad_keys;
        assert!(!chunking_proof.verify(bad_instance));

        // no ciphertexts
        let mut bad_instance = good_instance.clone();
        bad_instance.ciphertext_chunks = &[];
        assert!(!chunking_proof.verify(bad_instance));

        // too many ciphertexts
        let mut bad_ciphertexts = owned_instance.ciphertext_chunks.clone();
        bad_ciphertexts.push(Default::default());

        let mut bad_instance = good_instance.clone();
        bad_instance.ciphertext_chunks = &bad_ciphertexts;
        assert!(!chunking_proof.verify(bad_instance));

        // too few ciphertexts
        let mut bad_ciphertexts = owned_instance.ciphertext_chunks.clone();
        bad_ciphertexts.truncate(bad_ciphertexts.len() - 1);

        let mut bad_instance = good_instance.clone();
        bad_instance.ciphertext_chunks = &bad_ciphertexts;
        assert!(!chunking_proof.verify(bad_instance));
    }

    #[test]
    fn should_fail_to_verify_proof_with_wrong_instance() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (owned_instance, r, shares) = setup(&mut rng);
        let instance = Instance {
            public_keys: &owned_instance.public_keys,
            rr: &owned_instance.randomizers_r,
            ciphertext_chunks: &owned_instance.ciphertext_chunks,
        };

        let chunking_proof =
            ProofOfChunking::construct(&mut rng, instance.clone(), &r, &shares).unwrap();

        let (owned_instance2, _, _) = setup(&mut rng);
        let bad_instance = Instance {
            public_keys: &owned_instance2.public_keys,
            rr: &owned_instance2.randomizers_r,
            ciphertext_chunks: &owned_instance2.ciphertext_chunks,
        };

        assert!(!chunking_proof.verify(bad_instance));
    }

    #[test]
    fn should_fail_to_verify_invalid_proof() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (owned_instance, r, shares) = setup(&mut rng);
        let instance = Instance {
            public_keys: &owned_instance.public_keys,
            rr: &owned_instance.randomizers_r,
            ciphertext_chunks: &owned_instance.ciphertext_chunks,
        };

        let good_proof =
            ProofOfChunking::construct(&mut rng, instance.clone(), &r, &shares).unwrap();

        // essentially mess with every field in some way

        let mut bad_proof = good_proof.clone();
        bad_proof.y0 = G1Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        ////

        let mut bad_proof = good_proof.clone();
        bad_proof.bb[0] = G1Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.bb.push(G1Projective::generator());
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.bb.truncate(bad_proof.bb.len() - 1);
        assert!(!bad_proof.verify(instance.clone()));

        ////

        let mut bad_proof = good_proof.clone();
        bad_proof.cc[0] = G1Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.cc.push(G1Projective::generator());
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.cc.truncate(bad_proof.cc.len() - 1);
        assert!(!bad_proof.verify(instance.clone()));

        ////

        let mut bad_proof = good_proof.clone();
        bad_proof.dd[0] = G1Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.dd.push(G1Projective::generator());
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.dd.truncate(bad_proof.dd.len() - 1);
        assert!(!bad_proof.verify(instance.clone()));

        ////

        let mut bad_proof = good_proof.clone();
        bad_proof.yy = G1Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        ////

        let mut bad_proof = good_proof.clone();
        bad_proof.responses_r[0] = Scalar::one();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.responses_r.push(Scalar::one());
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof
            .responses_r
            .truncate(bad_proof.responses_r.len() - 1);
        assert!(!bad_proof.verify(instance.clone()));

        ////

        let mut bad_proof = good_proof.clone();
        bad_proof.responses_chunks[0] = 12345;
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.responses_chunks.push(12345);
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof
            .responses_chunks
            .truncate(bad_proof.responses_chunks.len() - 1);
        assert!(!bad_proof.verify(instance.clone()));

        ////

        let mut bad_proof = good_proof;
        bad_proof.response_beta = Scalar::one();
        assert!(!bad_proof.verify(instance));
    }

    #[test]
    fn proof_of_chunking_roundtrip() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (owned_instance, r, shares) = setup(&mut rng);
        let instance = Instance {
            public_keys: &owned_instance.public_keys,
            rr: &owned_instance.randomizers_r,
            ciphertext_chunks: &owned_instance.ciphertext_chunks,
        };

        let good_proof =
            ProofOfChunking::construct(&mut rng, instance.clone(), &r, &shares).unwrap();

        let bytes = good_proof.to_bytes();
        let recovered = ProofOfChunking::try_from_bytes(&bytes).unwrap();
        assert_eq!(good_proof, recovered)
    }
}
