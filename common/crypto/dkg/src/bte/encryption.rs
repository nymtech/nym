// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::keys::{DecryptionKey, PublicKey};
use crate::bte::{Epoch, Params, CHUNK_SIZE, G2_GENERATOR_PREPARED, NUM_CHUNKS, PAIRING_BASE};
use crate::error::DkgError;
use crate::utils::{combine_g1_chunks, combine_scalar_chunks, deserialize_g1, deserialize_g2};
use crate::{Chunk, ChunkedShare, Share};
use bls12_381::{G1Affine, G1Projective, G2Prepared, G2Projective, Gt, Scalar};
use ff::Field;
use group::{Curve, Group, GroupEncoding};
use rand_core::RngCore;
use std::collections::HashMap;
use std::ops::Neg;
use zeroize::Zeroize;

#[derive(Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub struct Ciphertexts {
    pub rr: [G1Projective; NUM_CHUNKS],
    pub ss: [G1Projective; NUM_CHUNKS],
    pub zz: [G2Projective; NUM_CHUNKS],
    pub ciphertext_chunks: Vec<[G1Projective; NUM_CHUNKS]>,
}

impl Ciphertexts {
    pub fn verify_integrity(&self, params: &Params, epoch: Epoch) -> bool {
        // if this checks fails it means the ciphertext is undefined as values
        // in `r`, `s` and `z` are meaningless since technically this ciphertext
        // has been created for 0 parties
        if self.ciphertext_chunks.is_empty() {
            return false;
        }

        let g1_neg = G1Affine::generator().neg();
        let f = epoch
            .as_extended_tau(&self.rr, &self.ss, &self.ciphertext_chunks)
            .evaluate_f(params);

        // we have to use `f` in up to `NUM_CHUNKS` pairings (if everything is valid),
        // so perform some precomputation on it
        let f_prepared = G2Prepared::from(f.to_affine());

        // for each triple (R_i, S_i, Z_i) check whether e(g1, Z_i) == e(R_j, f) • e(S_i, h),
        // which is equivalent to checking whether e(R_j, f) • e(S_i, h) • e(g1, Z_i)^-1 == id
        // and due to bilinear property whether e(R_j, f) • e(S_i, h) • e(g1^-1, Z_i) == id
        for i in 0..self.rr.len() {
            let miller = bls12_381::multi_miller_loop(&[
                (&self.rr[i].to_affine(), &f_prepared),
                (&self.ss[i].to_affine(), &params._h_prepared),
                (&g1_neg, &G2Prepared::from(self.zz[i].to_affine())),
            ]);
            let res = miller.final_exponentiation();
            if !bool::from(res.is_identity()) {
                return false;
            }
        }

        true
    }

    pub fn combine_rs(&self) -> G1Projective {
        combine_g1_chunks(&self.rr)
    }

    // required for the purposes of the proof of secret sharing
    pub fn combine_ciphertexts(&self) -> Vec<G1Projective> {
        self.ciphertext_chunks
            .iter()
            .map(|share_ciphertext| combine_g1_chunks(share_ciphertext))
            .collect()
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let num_receivers = self.ciphertext_chunks.len();

        let mut bytes = Vec::with_capacity(NUM_CHUNKS * ((num_receivers + 2) * 48 + 96) + 4);
        for r_i in &self.rr {
            bytes.extend_from_slice(r_i.to_bytes().as_ref())
        }
        for s_i in &self.ss {
            bytes.extend_from_slice(s_i.to_bytes().as_ref())
        }
        for z_i in &self.zz {
            bytes.extend_from_slice(z_i.to_bytes().as_ref())
        }

        bytes.extend_from_slice(&(num_receivers as u32).to_be_bytes());
        for c_i in &self.ciphertext_chunks {
            for c_ij in c_i {
                bytes.extend_from_slice(c_ij.to_bytes().as_ref())
            }
        }

        bytes
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        // at the very minimum we must have enough bytes for a single receiver
        if bytes.len() < NUM_CHUNKS * (3 * 48 + 96) + 4 {
            return Err(DkgError::new_deserialization_failure(
                "Ciphertexts",
                "insufficient number of bytes provided",
            ));
        }

        let mut rr = Vec::with_capacity(NUM_CHUNKS);
        let mut ss = Vec::with_capacity(NUM_CHUNKS);
        let mut zz = Vec::with_capacity(NUM_CHUNKS);

        let mut i = 0;
        for _ in 0..NUM_CHUNKS {
            rr.push(deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
                DkgError::new_deserialization_failure("Ciphertexts.r", "invalid curve point")
            })?);
            i += 48;
        }
        for _ in 0..NUM_CHUNKS {
            ss.push(deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
                DkgError::new_deserialization_failure("Ciphertexts.s", "invalid curve point")
            })?);
            i += 48;
        }
        for _ in 0..NUM_CHUNKS {
            zz.push(deserialize_g2(&bytes[i..i + 96]).ok_or_else(|| {
                DkgError::new_deserialization_failure("Ciphertexts.z", "invalid curve point")
            })?);
            i += 96;
        }

        let num_receivers = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        i += 4;

        if bytes[i..].len() != num_receivers * NUM_CHUNKS * 48 {
            return Err(DkgError::new_deserialization_failure(
                "Ciphertexts",
                "invalid number of bytes provided",
            ));
        }

        let mut ciphertext_chunks = Vec::with_capacity(num_receivers);

        for _ in 0..num_receivers {
            let mut ci = Vec::with_capacity(NUM_CHUNKS);
            for _ in 0..NUM_CHUNKS {
                ci.push(deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
                    DkgError::new_deserialization_failure(
                        "Ciphertexts.ciphertext_chunks",
                        "invalid curve point",
                    )
                })?);
                i += 48;
            }

            // this unwrap is fine as we have exactly NUM_CHUNKS elements in each vector
            ciphertext_chunks.push(ci.try_into().unwrap())
        }

        // and the same is true here, the unwraps are fine as we have exactly NUM_CHUNKS elements in each as required
        Ok(Ciphertexts {
            rr: rr.try_into().unwrap(),
            ss: ss.try_into().unwrap(),
            zz: zz.try_into().unwrap(),
            ciphertext_chunks,
        })
    }
}

#[derive(Zeroize)]
#[zeroize(drop)]
/// Randomness generated during ciphertext generation that is required for proofs of knowledge.
/// It must be handled with extreme care as its misuse might help malicious parties to recover
/// the underlying plaintext.
pub struct HazmatRandomness {
    r: [Scalar; NUM_CHUNKS],
    s: [Scalar; NUM_CHUNKS],
}

impl HazmatRandomness {
    pub fn r(&self) -> &[Scalar; NUM_CHUNKS] {
        &self.r
    }

    pub fn s(&self) -> &[Scalar; NUM_CHUNKS] {
        &self.s
    }

    pub fn combine_rs(&self) -> Scalar {
        combine_scalar_chunks(&self.r)
    }
}

pub fn encrypt_shares(
    shares: &[(&Share, &PublicKey)],
    epoch: Epoch,
    params: &Params,
    mut rng: impl RngCore,
) -> (Ciphertexts, HazmatRandomness) {
    let g1 = G1Projective::generator();

    let mut rand_rs = Vec::with_capacity(NUM_CHUNKS);
    let mut rand_ss = Vec::with_capacity(NUM_CHUNKS);
    let mut rr = Vec::with_capacity(NUM_CHUNKS);
    let mut ss = Vec::with_capacity(NUM_CHUNKS);

    // generate relevant re-usable pseudorandom data
    for _ in 0..NUM_CHUNKS {
        let rand_r = Scalar::random(&mut rng);
        let rand_s = Scalar::random(&mut rng);

        // g1^r
        let rr_i = g1 * rand_r;
        // g1^s
        let ss_i = g1 * rand_s;

        rand_rs.push(rand_r);
        rand_ss.push(rand_s);

        rr.push(rr_i);
        ss.push(ss_i);
    }

    // produce per-chunk ciphertexts
    let mut cc = Vec::with_capacity(shares.len());

    for (share, pk) in shares {
        let m = share.to_chunks();

        let mut ci = Vec::with_capacity(NUM_CHUNKS);

        for (j, chunk) in m.chunks.iter().enumerate() {
            // can't really have a more efficient implementation until https://github.com/zkcrypto/bls12_381/pull/70 is merged...
            let c = pk.0 * rand_rs[j] + g1 * Scalar::from(*chunk as u64);
            ci.push(c)
        }

        // the conversion must succeed since we must have EXACTLY `NUM_CHUNKS` elements
        cc.push(ci.try_into().unwrap())
    }

    // convert into arrays, note that the unwraps are fine as we have exactly `NUM_CHUNKS` elements in each vector
    let rr = rr.try_into().unwrap();
    let ss = ss.try_into().unwrap();

    let f = epoch.as_extended_tau(&rr, &ss, &cc).evaluate_f(params);

    let mut zz = Vec::with_capacity(NUM_CHUNKS);
    for i in 0..NUM_CHUNKS {
        zz.push(f * rand_rs[i] + params.h * rand_ss[i]);
    }

    // the conversions here must also succeed since the other vecs also have `NUM_CHUNKS` elements
    (
        Ciphertexts {
            rr,
            ss,
            zz: zz.try_into().unwrap(),
            ciphertext_chunks: cc,
        },
        HazmatRandomness {
            r: rand_rs.try_into().unwrap(),
            s: rand_ss.try_into().unwrap(),
        },
    )
}

pub fn decrypt_share(
    dk: &DecryptionKey,
    // in the case of multiple receivers, specifies which index of ciphertext chunks should be used
    i: usize,
    ciphertext: &Ciphertexts,
    epoch: Epoch,
    lookup_table: Option<&BabyStepGiantStepLookup>,
) -> Result<Share, DkgError> {
    let mut plaintext = ChunkedShare::default();

    let decryption_node = dk.try_get_compatible_node(epoch)?;
    let extended_tau = epoch.as_extended_tau(
        &ciphertext.rr,
        &ciphertext.ss,
        &ciphertext.ciphertext_chunks,
    );

    if i >= ciphertext.ciphertext_chunks.len() {
        return Err(DkgError::UnavailableCiphertext(i));
    }

    let height = decryption_node.tau.height();
    let b_neg = decryption_node
        .ds
        .iter()
        .chain(decryption_node.dh.iter())
        .zip(extended_tau.0.iter().by_vals().skip(height))
        .filter(|(_, i)| *i)
        .map(|(d_i, _)| d_i)
        .fold(decryption_node.b, |acc, d_i| acc + d_i)
        .neg()
        .to_affine();

    let e_neg = decryption_node.e.neg().to_affine();

    for j in 0..NUM_CHUNKS {
        let rr_j = &ciphertext.rr[j];
        let ss_j = &ciphertext.ss[j];
        let zz_j = ciphertext.zz[j].to_affine();
        let cc_ij = &ciphertext.ciphertext_chunks[i][j];

        let miller = bls12_381::multi_miller_loop(&[
            (&cc_ij.to_affine(), &G2_GENERATOR_PREPARED),
            (&rr_j.to_affine(), &G2Prepared::from(b_neg)),
            (&decryption_node.a.to_affine(), &G2Prepared::from(zz_j)),
            (&ss_j.to_affine(), &G2Prepared::from(e_neg)),
        ]);
        let m = miller.final_exponentiation();

        plaintext.chunks[j] = baby_step_giant_step(&m, &PAIRING_BASE, lookup_table)?;
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

    pub fn try_solve(&self, target: &Gt) -> Result<Chunk, DkgError> {
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
    use crate::bte::{keygen, setup, DEFAULT_BSGS_TABLE};
    use rand_core::SeedableRng;

    fn verify_hazmat_rand(ciphertext: &Ciphertexts, randomness: &HazmatRandomness) {
        let g1 = G1Projective::generator();

        for i in 0..ciphertext.rr.len() {
            assert_eq!(ciphertext.rr[i], g1 * randomness.r[i]);
            assert_eq!(ciphertext.ss[i], g1 * randomness.s[i]);
        }
    }

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
    fn share_decryption_20() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let (decryption_key1, public_key1) = keygen(&params, &mut rng);
        let (decryption_key2, public_key2) = keygen(&params, &mut rng);
        let epoch = Epoch::new(0);

        let lookup_table = &DEFAULT_BSGS_TABLE;

        for _ in 0..10 {
            let m1 = Share::random(&mut rng);
            let m2 = Share::random(&mut rng);
            let shares = &[(&m1, &public_key1.key), (&m2, &public_key2.key)];

            let (ciphertext, hazmat) = encrypt_shares(shares, epoch, &params, &mut rng);
            verify_hazmat_rand(&ciphertext, &hazmat);

            let recovered1 =
                decrypt_share(&decryption_key1, 0, &ciphertext, epoch, Some(lookup_table)).unwrap();
            let recovered2 =
                decrypt_share(&decryption_key2, 1, &ciphertext, epoch, Some(lookup_table)).unwrap();
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
        let epoch = Epoch::new(12345);
        decryption_key1
            .try_update_to(epoch, &params, &mut rng)
            .unwrap();
        decryption_key2
            .try_update_to(epoch, &params, &mut rng)
            .unwrap();

        let lookup_table = &DEFAULT_BSGS_TABLE;

        for _ in 0..10 {
            let m1 = Share::random(&mut rng);
            let m2 = Share::random(&mut rng);
            let shares = &[(&m1, &public_key1.key), (&m2, &public_key2.key)];

            let (ciphertext, hazmat) = encrypt_shares(shares, epoch, &params, &mut rng);
            verify_hazmat_rand(&ciphertext, &hazmat);

            let recovered1 =
                decrypt_share(&decryption_key1, 0, &ciphertext, epoch, Some(lookup_table)).unwrap();
            let recovered2 =
                decrypt_share(&decryption_key2, 1, &ciphertext, epoch, Some(lookup_table)).unwrap();
            assert_eq!(m1, recovered1);
            assert_eq!(m2, recovered2);
        }
    }

    #[test]
    fn decryption_with_root_key() {
        let dummy_seed = [42u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let (root_key, public_key) = keygen(&params, &mut rng);

        let share = Share::random(&mut rng);

        let epoch0 = Epoch::new(0);
        let epoch42 = Epoch::new(42);
        let epoch_big = Epoch::new(3292547435);

        let (ciphertext1, hazmat1) =
            encrypt_shares(&[(&share, &public_key.key)], epoch0, &params, &mut rng);
        verify_hazmat_rand(&ciphertext1, &hazmat1);

        let (ciphertext2, hazmat2) =
            encrypt_shares(&[(&share, &public_key.key)], epoch42, &params, &mut rng);
        verify_hazmat_rand(&ciphertext2, &hazmat2);

        let (ciphertext3, hazmat3) =
            encrypt_shares(&[(&share, &public_key.key)], epoch_big, &params, &mut rng);
        verify_hazmat_rand(&ciphertext3, &hazmat3);

        let recovered1 = decrypt_share(&root_key, 0, &ciphertext1, epoch0, None).unwrap();
        let recovered2 = decrypt_share(&root_key, 0, &ciphertext2, epoch42, None).unwrap();
        let recovered3 = decrypt_share(&root_key, 0, &ciphertext3, epoch_big, None).unwrap();

        assert_eq!(share, recovered1);
        assert_eq!(share, recovered2);
        assert_eq!(share, recovered3);
    }

    #[test]
    fn update_and_decrypt_10() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let (mut decryption_key, public_key) = keygen(&params, &mut rng);

        for epoch_value in 0..10 {
            let epoch = Epoch::new(epoch_value);
            let share = Share::random(&mut rng);
            decryption_key
                .try_update_to(epoch, &params, &mut rng)
                .unwrap();

            let (ciphertext, hazmat) =
                encrypt_shares(&[(&share, &public_key.key)], epoch, &params, &mut rng);
            verify_hazmat_rand(&ciphertext, &hazmat);

            let recovered = decrypt_share(&decryption_key, 0, &ciphertext, epoch, None).unwrap();
            assert_eq!(share, recovered);
        }
    }

    #[test]
    fn reblinding_node_doesnt_affect_decryption() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let (mut decryption_key, public_key) = keygen(&params, &mut rng);

        let epoch = Epoch::new(12345);
        decryption_key
            .try_update_to(epoch, &params, &mut rng)
            .unwrap();
        for node in decryption_key.nodes.iter_mut() {
            node.reblind(&params, &mut rng);
        }
        let share = Share::random(&mut rng);

        let (ciphertext, hazmat) =
            encrypt_shares(&[(&share, &public_key.key)], epoch, &params, &mut rng);
        verify_hazmat_rand(&ciphertext, &hazmat);

        let recovered = decrypt_share(&decryption_key, 0, &ciphertext, epoch, None).unwrap();
        assert_eq!(share, recovered);

        // attempt to update the key again so we have to derive fresh nodes using previous reblinded results
        let epoch2 = Epoch::new(67890);
        decryption_key
            .try_update_to(epoch2, &params, &mut rng)
            .unwrap();
        for node in decryption_key.nodes.iter_mut() {
            node.reblind(&params, &mut rng);
        }
        let share2 = Share::random(&mut rng);

        let (ciphertext, hazmat) =
            encrypt_shares(&[(&share2, &public_key.key)], epoch2, &params, &mut rng);
        verify_hazmat_rand(&ciphertext, &hazmat);

        let recovered = decrypt_share(&decryption_key, 0, &ciphertext, epoch2, None).unwrap();
        assert_eq!(share2, recovered);
    }

    #[test]
    fn ciphertext_integrity_check_passes_for_valid_data() {
        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (mut dk, public_key) = keygen(&params, &mut rng);
        let epoch = Epoch::new(1);

        dk.try_update_to(epoch, &params, &mut rng).unwrap();
        let share = Share::random(&mut rng);
        let (ciphertext, _) =
            encrypt_shares(&[(&share, &public_key.key)], epoch, &params, &mut rng);
        assert!(ciphertext.verify_integrity(&params, epoch))
    }

    #[test]
    fn ciphertext_integrity_check_passes_fails_for_malformed_data() {
        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (mut dk, public_key) = keygen(&params, &mut rng);
        let epoch = Epoch::new(1);

        dk.try_update_to(epoch, &params, &mut rng).unwrap();
        let share = Share::random(&mut rng);
        let (ciphertext, _) =
            encrypt_shares(&[(&share, &public_key.key)], epoch, &params, &mut rng);

        let mut bad_cipher1 = ciphertext.clone();
        bad_cipher1.rr[4] = G1Projective::generator();
        assert!(!bad_cipher1.verify_integrity(&params, epoch));

        let mut bad_cipher2 = ciphertext.clone();
        bad_cipher2.ss[4] = G1Projective::generator();
        assert!(!bad_cipher2.verify_integrity(&params, epoch));

        let mut bad_cipher3 = ciphertext;
        bad_cipher3.zz[4] = G2Projective::generator();
        assert!(!bad_cipher3.verify_integrity(&params, epoch));
    }

    #[test]
    fn ciphertext_integrity_check_passes_fails_for_wrong_epoch() {
        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (mut dk, public_key) = keygen(&params, &mut rng);
        let epoch = Epoch::new(1);

        dk.try_update_to(epoch, &params, &mut rng).unwrap();
        let share = Share::random(&mut rng);
        let (ciphertext, _) =
            encrypt_shares(&[(&share, &public_key.key)], epoch, &params, &mut rng);

        let another_epoch = Epoch::new(2);
        assert!(!ciphertext.verify_integrity(&params, another_epoch))
    }

    #[test]
    fn ciphertext_combining() {
        let params = setup();

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let nodes = 3;

        let mut shares = Vec::new();
        let mut public_keys = Vec::new();
        for _ in 0..nodes {
            shares.push(Share::random(&mut rng));
            let (_, pk) = keygen(&params, &mut rng);
            public_keys.push(*pk.public_key());
        }

        let refs = shares.iter().zip(public_keys.iter()).collect::<Vec<_>>();
        let (ciphertext, hazmat) = encrypt_shares(&refs, Epoch::new(42), &params, &mut rng);

        let combined_r = combine_scalar_chunks(hazmat.r());
        let combined_rr = ciphertext.combine_rs();
        let combined_ciphertexts = ciphertext.combine_ciphertexts();

        let g1 = G1Projective::generator();
        for i in 0..nodes {
            let expected = public_keys[i].0 * combined_r + g1 * shares[i].0;
            assert_eq!(expected, combined_ciphertexts[i]);
            assert_eq!(combined_rr, g1 * combined_r);
        }
    }

    #[test]
    fn ciphertexts_roundtrip() {
        fn random_ciphertexts(mut rng: impl RngCore, num_receivers: usize) -> Ciphertexts {
            Ciphertexts {
                rr: (0..NUM_CHUNKS)
                    .map(|_| G1Projective::random(&mut rng))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
                ss: (0..NUM_CHUNKS)
                    .map(|_| G1Projective::random(&mut rng))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
                zz: (0..NUM_CHUNKS)
                    .map(|_| G2Projective::random(&mut rng))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
                ciphertext_chunks: (0..num_receivers)
                    .map(|_| {
                        (0..NUM_CHUNKS)
                            .map(|_| G1Projective::random(&mut rng))
                            .collect::<Vec<_>>()
                            .try_into()
                            .unwrap()
                    })
                    .collect(),
            }
        }

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let good_ciphertexts = vec![
            random_ciphertexts(&mut rng, 1),
            random_ciphertexts(&mut rng, 2),
            random_ciphertexts(&mut rng, 10),
        ];

        for ciphertexts in &good_ciphertexts {
            let bytes = ciphertexts.to_bytes();
            let recovered = Ciphertexts::try_from_bytes(&bytes).unwrap();
            assert_eq!(ciphertexts, &recovered);
        }

        // ciphertext for 0 receivers is invalid by default
        let ciphertexts = random_ciphertexts(&mut rng, 0);
        let bytes = ciphertexts.to_bytes();
        assert!(Ciphertexts::try_from_bytes(&bytes).is_err());
    }
}
