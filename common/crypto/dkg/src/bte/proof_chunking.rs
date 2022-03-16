// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::{Ciphertext, PublicKey, CHUNK_MAX, NUM_CHUNKS};
use crate::error::DkgError;
use crate::utils::{hash_to_scalar, hash_to_scalars};
use bls12_381::{G1Projective, G2Projective, Scalar};
use ff::Field;
use group::{Group, GroupEncoding};
use rand::Rng;
use rand_core::{RngCore, SeedableRng};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

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
type FirstChallenge = Vec<Vec<Vec<Scalar>>>;

// TODO: perhaps break it down into separate arguments after all
#[cfg_attr(test, derive(Clone))]
pub(crate) struct Instance<'a> {
    /// y_1, ..., y_n
    public_keys: &'a [PublicKey],

    /// R_1, ..., R_m
    randomizers_r: &'a [G1Projective; NUM_CHUNKS],

    /// C_{1,1}, ..., C_{n,m}
    ciphertext_chunks: &'a [[G1Projective; NUM_CHUNKS]],
}

impl<'a> Instance<'a> {
    pub(crate) fn new(public_keys: &'a [PublicKey], ciphertext: &'a Ciphertext) -> Instance<'a> {
        Instance {
            public_keys,
            randomizers_r: &ciphertext.r,
            ciphertext_chunks: &ciphertext.ciphertext_chunks,
        }
    }

    // TODO: possibly no longer needed after all
    fn to_bytes(&self) -> Vec<u8> {
        let elements =
            self.public_keys.len() + NUM_CHUNKS + NUM_CHUNKS * self.ciphertext_chunks.len();

        let mut bytes = Vec::with_capacity(48 * elements);

        for pk in self.public_keys {
            bytes.extend_from_slice(pk.0.to_bytes().as_ref())
        }
        for rr in self.randomizers_r {
            bytes.extend_from_slice(rr.to_bytes().as_ref())
        }

        for ciphertext_chunks in self.ciphertext_chunks {
            for chunk in ciphertext_chunks {
                bytes.extend_from_slice(chunk.to_bytes().as_ref())
            }
        }

        bytes
    }

    fn validate(&self) -> bool {
        if self.public_keys.is_empty() || self.randomizers_r.is_empty() {
            return false;
        }

        if self.public_keys.len() != self.ciphertext_chunks.len() {
            return false;
        }

        true
    }
}

#[cfg_attr(test, derive(Clone))]
pub struct ProofOfChunking {
    // TODO: ask @AP for better names for those
    y0: G1Projective,
    bb: Vec<G1Projective>,
    cc: Vec<G1Projective>,
    dd: Vec<G1Projective>,
    yy: G1Projective,
    responses_r: Vec<Scalar>,
    responses_chunks: Vec<Scalar>,
    response_beta: Scalar,
}

impl ProofOfChunking {
    pub(crate) fn construct(
        mut rng: impl RngCore,
        instance: Instance,
        witness_r: &[Scalar; NUM_CHUNKS],
        // TODO: is it `Scalar` or `Chunk`?
        witnesses_s: &[[Scalar; NUM_CHUNKS]],
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
        let ss = (n * m * (CHUNK_MAX - 1) * (ee - 1)) as u64;
        let zz = 2 * (PARALLEL_RUNS as u64) * ss;

        let ss_scalar = Scalar::from(ss);

        // generating pseudorandom values uniformly in range is way simpler with u64 compared to Scalars,
        // plus the absolute upper bound, i.e. z - 1 + s + 1 is way smaller than u64::MAX so it's safe
        // to do it this way
        let combined_upper_range = zz - 1 + ss + 1;

        let mut betas = Vec::with_capacity(PARALLEL_RUNS);
        let mut bs = Vec::with_capacity(PARALLEL_RUNS);

        for _ in 0..PARALLEL_RUNS {
            let beta = Scalar::random(&mut rng);

            // g1 ^ beta_l
            let bb = g1 * beta;

            betas.push(beta);
            bs.push(bb);
        }

        let mut zz_be_bytes = Scalar::from(zz).to_bytes();
        zz_be_bytes.reverse();

        let mut attempt = 0;
        let (first_challenge, responses_chunks, cs) = 'retry_loop: loop {
            attempt += 1;
            if attempt == SECURITY_PARAMETER {
                return Err(DkgError::AbortedProofOfChunking);
            }

            let mut blinding_factors = Vec::with_capacity(PARALLEL_RUNS);
            let mut cs = Vec::with_capacity(PARALLEL_RUNS);

            for i in 0..PARALLEL_RUNS {
                // scalar in range of [0, Z - 1 + S]
                let shifted = Scalar::from(rng.gen_range(0..=combined_upper_range));
                // [-S, Z - 1] as required
                // TODO: CHECK FOR OFF BY ONE ERRORS
                let blinding_factor = shifted - ss_scalar;

                // y0 ^ beta_l • g1 ^ sigma_l
                let cc = y0 * betas[i] + g1 * blinding_factor;

                blinding_factors.push(blinding_factor);
                cs.push(cc);
            }

            let first_challenge = Self::compute_first_challenge(&instance, &y0, &bs, &cs, n, m);

            // compute:
            // z_{s,1} = sum(i <- 1..n; (sum j <- j..m; e_{i,j,1} • s_{i,j} + sigma_1))
            // ...
            // z_{s,l} = sum(i <- 1..n; (sum j <- j..m; e_{i,j,l} • s_{i,j} + sigma_l))
            // such that 0 <= z_{s,l} < Z
            let mut responses_chunks = Vec::with_capacity(PARALLEL_RUNS);

            for l in 0..PARALLEL_RUNS {
                let mut acc = Scalar::zero();
                for (i, e_i) in first_challenge.iter().enumerate() {
                    for (j, e_ij) in e_i.iter().enumerate() {
                        acc += e_ij[l] * witnesses_s[i][j]
                    }
                }
                acc += blinding_factors[l];

                // technically it doesn't really make much sense as there's no such thing as ordering in finite fields,
                // but that's the best we can do to follow the requirements
                let mut acc_be_bytes = acc.to_bytes();
                acc_be_bytes.reverse();

                if acc_be_bytes >= zz_be_bytes {
                    println!("{:?} >= {:?}", acc, zz);
                    continue 'retry_loop;
                } else {
                    responses_chunks.push(acc)
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

        // TODO: is this correct capacity?
        let mut responses_r = Vec::with_capacity(PARALLEL_RUNS);

        for (i, e_i) in first_challenge.iter().enumerate() {
            let mut response_r_k = deltas[i + 1];
            for (j, e_ij) in e_i.iter().enumerate() {
                // c^1 in first iteration, c^k in the last
                let mut challenge_pow = second_challenge;
                for e_ijk in e_ij.iter() {
                    response_r_k += (e_ijk * witness_r[j] * challenge_pow);
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

    pub(crate) fn verify(&self) -> bool {
        false
    }

    // note for future self: this doesn't work as challenge items need to be in the [0, E - 1] range...
    // pub(crate) fn compute_first_challenge_old(
    //     instance: Instance,
    //     y0: &G1Projective,
    //     bb: &[G1Projective],
    //     cc: &[G1Projective],
    //     n: usize,
    //     m: usize,
    // ) -> Vec<Vec<Vec<Scalar>>> {
    //     let mut bytes = instance.to_bytes();
    //     bytes.extend_from_slice(y0.to_bytes().as_ref());
    //     for b in bb {
    //         bytes.extend_from_slice(b.to_bytes().as_ref())
    //     }
    //
    //     for c in cc {
    //         bytes.extend_from_slice(c.to_bytes().as_ref())
    //     }
    //
    //     let lambda_e = n * m * PARALLEL_RUNS * NUM_CHALLENGE_BITS;
    //     bytes.extend_from_slice(lambda_e.to_be_bytes().as_ref());
    //
    //     let output_size = n * m * PARALLEL_RUNS;
    //     let mut out = hash_to_scalars(&bytes, FIRST_CHALLENGE_DOMAIN, output_size);
    //
    //     // TODO: possibly might have to swap m and n around. not sure yet.
    //     (0..m)
    //         .map(|_| (0..n).map(|_| out.split_off(PARALLEL_RUNS)).collect())
    //         .collect()
    // }

    // TODO: possibly return Vec<Vec<Vec<u64>>> after all
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
        random_oracle_builder.update_with_g1_elements(instance.randomizers_r.iter());
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

        // TODO: possibly might have to swap m and n around. not sure yet.
        (0..n)
            .map(|_| {
                (0..m)
                    .map(|_| {
                        (0..PARALLEL_RUNS)
                            .map(|_| Scalar::from(oracle.gen_range(0..range_max_excl)))
                            .collect()
                    })
                    .collect()
            })
            .collect()
    }

    fn compute_second_challenge(
        first_challenge: &FirstChallenge,
        responses_chunks: &[Scalar],
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
                    bytes.extend_from_slice(e_ijk.to_bytes().as_ref());
                }
            }
        }

        for z in responses_chunks {
            bytes.extend_from_slice(z.to_bytes().as_ref())
        }

        for d in ds {
            bytes.extend_from_slice(d.to_bytes().as_ref())
        }

        bytes.extend_from_slice(y.to_bytes().as_ref());

        hash_to_scalar(bytes, SECOND_CHALLENGE_DOMAIN)
    }
}

// TODO: perhaps move to a common/utils part of the dkg crate?
struct RandomOracleBuilder {
    inner_state: Sha256,
}

impl RandomOracleBuilder {
    fn new(domain: &[u8]) -> Self {
        let mut inner_state = Sha256::new();
        inner_state.update(domain);

        RandomOracleBuilder { inner_state }
    }

    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.inner_state.update(data)
    }

    fn update_with_g1_elements<'a, I>(&mut self, items: I)
    where
        I: Iterator<Item = &'a G1Projective>,
    {
        items.for_each(|item| self.update(item.to_bytes()))
    }

    fn finalize(self) -> [u8; 32] {
        self.inner_state.finalize().into()
    }
}
