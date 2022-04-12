// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::proof_chunking::ProofOfChunking;
use crate::bte::proof_sharing::ProofOfSecretSharing;
use crate::bte::{
    encrypt_shares, proof_chunking, proof_sharing, Ciphertexts, Epoch, Params, PublicKey,
};
use crate::error::DkgError;
use crate::interpolation::polynomial::{Polynomial, PublicCoefficients};
use crate::interpolation::{
    perform_lagrangian_interpolation_at_origin, perform_lagrangian_interpolation_at_x,
};
use crate::{NodeIndex, Share, Threshold};
use bls12_381::{G2Projective, Scalar};
use rand_core::RngCore;
use std::collections::BTreeMap;
use zeroize::Zeroize;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Dealing {
    pub public_coefficients: PublicCoefficients,
    pub ciphertexts: Ciphertexts,
    pub proof_of_chunking: ProofOfChunking,
    pub proof_of_sharing: ProofOfSecretSharing,
}

impl Dealing {
    // I'm not a big fan of this function signature, but I'm not clear on how to improve it while
    // allowing the dealer to skip decryption of its own share if it was also one of the receivers
    pub fn create(
        mut rng: impl RngCore,
        params: &Params,
        dealer_index: NodeIndex,
        threshold: Threshold,
        epoch: Epoch,
        // BTreeMap ensures the keys are sorted by their indices
        receivers: &BTreeMap<NodeIndex, PublicKey>,
        prior_resharing_secret: Option<Scalar>,
    ) -> (Self, Option<Share>) {
        assert!(threshold > 0);

        let mut polynomial = Polynomial::new_random(&mut rng, threshold - 1);
        if let Some(prior_secret) = prior_resharing_secret {
            polynomial.set_constant_coefficient(prior_secret)
        }

        let mut shares = receivers
            .keys()
            .map(|&node_index| polynomial.evaluate_at(&Scalar::from(node_index)).into())
            .collect::<Vec<_>>();

        let remote_share_key_pairs = shares
            .iter()
            .zip(receivers.values())
            .map(|(share, key)| (share, key))
            .collect::<Vec<_>>();
        let ordered_public_keys = receivers.values().copied().collect::<Vec<_>>();

        let (ciphertexts, hazmat) =
            encrypt_shares(&remote_share_key_pairs, epoch, params, &mut rng);

        // create proofs of knowledge
        let chunking_instance = proof_chunking::Instance::new(&ordered_public_keys, &ciphertexts);
        let proof_of_chunking =
            ProofOfChunking::construct(&mut rng, chunking_instance, hazmat.r(), &shares)
                .expect("failed to construct proof of chunking");

        let combined_ciphertexts = ciphertexts.combine_ciphertexts();
        let mut combined_r = hazmat.combine_rs();
        let combined_rr = ciphertexts.combine_rs();

        let public_coefficients = polynomial.public_coefficients();
        let sharing_instance = proof_sharing::Instance::new(
            receivers,
            &public_coefficients,
            &combined_rr,
            &combined_ciphertexts,
        );
        let proof_of_sharing =
            ProofOfSecretSharing::construct(&mut rng, sharing_instance, &combined_r, &shares)
                .expect("failed to construct proof of secret sharing");

        combined_r.zeroize();

        let dealing = Dealing {
            public_coefficients,
            ciphertexts,
            proof_of_chunking,
            proof_of_sharing,
        };

        let dealers_key_index = receivers
            .keys()
            .position(|node_index| node_index == &dealer_index);
        if let Some(dealer_key_index) = dealers_key_index {
            let dealers_share = shares.remove(dealer_key_index);
            shares.zeroize();
            (dealing, Some(dealers_share))
        } else {
            (dealing, None)
        }
    }

    // rather than returning a bool for whether the dealing is valid or not, a Result is returned
    // instead so that we would have more information regarding a possible failure cause
    pub fn verify(
        &self,
        params: &Params,
        epoch: Epoch,
        threshold: Threshold,
        receivers: &BTreeMap<NodeIndex, PublicKey>,
        prior_resharing_public: Option<G2Projective>,
    ) -> Result<(), DkgError> {
        if threshold == 0 || threshold as usize > receivers.len() {
            return Err(DkgError::InvalidThreshold {
                actual: threshold as usize,
                participating: receivers.len(),
            });
        }

        if self.ciphertexts.ciphertext_chunks.len() != receivers.len() {
            return Err(DkgError::WrongCiphertextSize {
                actual: self.ciphertexts.ciphertext_chunks.len(),
                expected: receivers.len(),
            });
        }

        if self.public_coefficients.size() != threshold as usize {
            return Err(DkgError::WrongPublicCoefficientsSize {
                actual: self.public_coefficients.size(),
                expected: threshold as usize,
            });
        }

        if !self.ciphertexts.verify_integrity(params, epoch) {
            return Err(DkgError::FailedCiphertextIntegrityCheck);
        }

        // TODO: perhaps change the underlying arguments in proofs of knowledge to avoid this allocation?
        let sorted_receivers = receivers.values().copied().collect::<Vec<_>>();

        let chunking_instance = proof_chunking::Instance::new(&sorted_receivers, &self.ciphertexts);
        if !self.proof_of_chunking.verify(chunking_instance) {
            return Err(DkgError::InvalidProofOfChunking);
        }

        let combined_randomizer = &self.ciphertexts.combine_rs();
        let combined_ciphertexts = &self.ciphertexts.combine_ciphertexts();

        let sharing_instance = proof_sharing::Instance::new(
            receivers,
            &self.public_coefficients,
            combined_randomizer,
            combined_ciphertexts,
        );

        if !self.proof_of_sharing.verify(sharing_instance) {
            return Err(DkgError::InvalidProofOfSharing);
        }

        if let Some(prior_public) = prior_resharing_public {
            let dealt_public = &self.public_coefficients[0];
            if dealt_public != &prior_public {
                return Err(DkgError::InvalidResharing);
            }
        }

        Ok(())
    }

    // coeff_len || coeff || cc_len || cc || pi_c_len || pi_c || pi_s_len || pi_s
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let mut coefficients_bytes = self.public_coefficients.to_bytes();
        bytes.extend_from_slice(&(coefficients_bytes.len() as u32).to_be_bytes());
        bytes.append(&mut coefficients_bytes);

        let mut ciphertexts_bytes = self.ciphertexts.to_bytes();
        bytes.extend_from_slice(&(ciphertexts_bytes.len() as u32).to_be_bytes());
        bytes.append(&mut ciphertexts_bytes);

        let mut proof_sharing_bytes = self.proof_of_sharing.to_bytes();
        bytes.extend_from_slice(&(proof_sharing_bytes.len() as u32).to_be_bytes());
        bytes.append(&mut proof_sharing_bytes);

        let mut proof_chunking_bytes = self.proof_of_chunking.to_bytes();
        bytes.extend_from_slice(&(proof_chunking_bytes.len() as u32).to_be_bytes());
        bytes.append(&mut proof_chunking_bytes);

        bytes
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        // can we read the length of serialized public coefficients?
        if bytes.len() < 4 {
            return Err(DkgError::new_deserialization_failure(
                "Dealing",
                "insufficient number of bytes provided",
            ));
        }

        let mut i = 0;
        let coefficients_bytes_len =
            u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;
        let public_coefficients =
            PublicCoefficients::try_from_bytes(&bytes[i..i + coefficients_bytes_len])?;
        i += coefficients_bytes_len;

        let ciphertexts_bytes_len =
            u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;
        let ciphertexts = Ciphertexts::try_from_bytes(&bytes[i..i + ciphertexts_bytes_len])?;
        i += ciphertexts_bytes_len;

        let proof_of_sharing_bytes_len =
            u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;
        let proof_of_sharing =
            ProofOfSecretSharing::try_from_bytes(&bytes[i..i + proof_of_sharing_bytes_len])?;
        i += proof_of_sharing_bytes_len;

        let proof_of_chunking_bytes_len =
            u32::from_be_bytes((&bytes[i..i + 4]).try_into().unwrap()) as usize;
        i += 4;

        if bytes[i..].len() != proof_of_chunking_bytes_len {
            return Err(DkgError::new_deserialization_failure(
                "Dealing",
                "invalid number of bytes provided",
            ));
        }

        let proof_of_chunking = ProofOfChunking::try_from_bytes(&bytes[i..])?;

        Ok(Dealing {
            public_coefficients,
            ciphertexts,
            proof_of_chunking,
            proof_of_sharing,
        })
    }
}

// this assumes all dealings have been verified
pub fn try_recover_verification_keys(
    dealings: &[Dealing],
    threshold: Threshold,
    receivers: &BTreeMap<NodeIndex, PublicKey>,
) -> Result<(G2Projective, Vec<G2Projective>), DkgError> {
    if dealings.is_empty() {
        return Err(DkgError::NoDealingsAvailable);
    }

    let threshold_usize = threshold as usize;

    if !dealings
        .iter()
        .all(|dealing| dealing.public_coefficients.size() == threshold_usize)
    {
        return Err(DkgError::MismatchedDealings);
    }

    // currently we expect every dealer to also be a receiver. This restriction might be relaxed in the future
    if dealings.len() != receivers.len() {
        return Err(DkgError::MismatchedDealings);
    }

    let indices = receivers.keys().collect::<Vec<_>>();

    // Compute A0, ..., A_{t-1}
    let mut interpolated_coefficients = Vec::with_capacity(threshold_usize);
    for k in 0..threshold_usize {
        let mut samples = Vec::with_capacity(indices.len());
        for (j, dealing) in dealings.iter().enumerate() {
            samples.push((
                Scalar::from(*indices[j]),
                *dealing.public_coefficients.nth(k),
            ))
        }
        let interpolated = perform_lagrangian_interpolation_at_origin(&samples)?;
        interpolated_coefficients.push(interpolated);
    }

    let master_verification_key = interpolated_coefficients[0];

    let interpolated_coefficients = PublicCoefficients {
        coefficients: interpolated_coefficients,
    };

    // shvk_j = A0^{j^0} * A1^{j^1} * ... * A_{t-1}^{j^{t-1}}
    let verification_key_shares = receivers
        .keys()
        .map(|index| interpolated_coefficients.evaluate_at(&Scalar::from(*index)))
        .collect();

    Ok((master_verification_key, verification_key_shares))
}

pub fn verify_verification_keys(
    master_key: &G2Projective,
    shares: &[G2Projective],
    receivers: &BTreeMap<NodeIndex, PublicKey>,
    threshold: Threshold,
) -> Result<(), DkgError> {
    if shares.len() != receivers.len() {
        return Err(DkgError::NotEnoughReceiversProvided);
    }

    if threshold as usize > receivers.len() {
        return Err(DkgError::InvalidThreshold {
            actual: threshold as usize,
            participating: receivers.len(),
        });
    }

    let indices = receivers.keys().copied().collect::<Vec<_>>();

    let indices_with_origin = std::iter::once(&0)
        .chain(receivers.keys())
        .collect::<Vec<_>>();
    let all_shares = std::iter::once(master_key)
        .chain(shares.iter())
        .collect::<Vec<_>>();

    for (i, share) in shares.iter().enumerate() {
        let samples = indices_with_origin
            .iter()
            .zip(all_shares.iter())
            .map(|(&node_index, &share)| (Scalar::from(*node_index), *share))
            .take(threshold as usize)
            .collect::<Vec<_>>();
        let interpolated =
            perform_lagrangian_interpolation_at_x(&Scalar::from(indices[i]), &samples)?;
        if share != &interpolated {
            return Err(DkgError::MismatchedVerificationKey);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bte::{decrypt_share, keygen, setup};
    use crate::combine_shares;
    use rand_core::SeedableRng;

    #[test]
    fn recovering_partial_verification_keys() {
        // START OF SETUP
        let dummy_seed = [42u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let threshold = 2;
        let node_indices = vec![1, 4, 7];

        let mut receivers = BTreeMap::new();
        let mut full_keys = Vec::new();
        for index in &node_indices {
            let (dk, pk) = keygen(&params, &mut rng);
            receivers.insert(*index, *pk.public_key());
            full_keys.push((dk, pk))
        }

        // start off in a defined epoch (i.e. not root);
        let epoch = Epoch::new(2);

        let dealings = node_indices
            .iter()
            .map(|&dealer_index| {
                Dealing::create(
                    &mut rng,
                    &params,
                    dealer_index,
                    threshold,
                    epoch,
                    &receivers,
                    None,
                )
                .0
            })
            .collect::<Vec<_>>();

        let mut derived_secrets = Vec::new();
        for (i, (ref mut dk, _)) in full_keys.iter_mut().enumerate() {
            dk.try_update_to(epoch, &params, &mut rng).unwrap();

            let shares = dealings
                .iter()
                .map(|dealing| decrypt_share(dk, i, &dealing.ciphertexts, epoch, None).unwrap())
                .collect();
            derived_secrets.push(
                combine_shares(shares, &receivers.keys().copied().collect::<Vec<_>>()).unwrap(),
            )
        }

        let master_secret = perform_lagrangian_interpolation_at_origin(&[
            (Scalar::from(node_indices[2]), derived_secrets[2]),
            (Scalar::from(node_indices[1]), derived_secrets[1]),
        ])
        .unwrap();

        // END OF SETUP
        let (recovered_master, recovered_partials) =
            try_recover_verification_keys(&dealings, threshold, &receivers).unwrap();

        let g2 = G2Projective::generator();
        assert_eq!(g2 * master_secret, recovered_master);

        assert_eq!(g2 * derived_secrets[0], recovered_partials[0]);
        assert_eq!(g2 * derived_secrets[1], recovered_partials[1]);
        assert_eq!(g2 * derived_secrets[2], recovered_partials[2]);
    }

    #[test]
    fn verifying_partial_verification_keys() {
        let dummy_seed = [42u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let threshold = 2;
        let node_indices = vec![1, 4, 7];

        let mut receivers = BTreeMap::new();
        let mut full_keys = Vec::new();
        for index in &node_indices {
            let (dk, pk) = keygen(&params, &mut rng);
            receivers.insert(*index, *pk.public_key());
            full_keys.push((dk, pk))
        }

        // start off in a defined epoch (i.e. not root);
        let epoch = Epoch::new(2);

        let dealings = node_indices
            .iter()
            .map(|&dealer_index| {
                Dealing::create(
                    &mut rng,
                    &params,
                    dealer_index,
                    threshold,
                    epoch,
                    &receivers,
                    None,
                )
                .0
            })
            .collect::<Vec<_>>();

        let (recovered_master, recovered_partials) =
            try_recover_verification_keys(&dealings, threshold, &receivers).unwrap();

        assert!(verify_verification_keys(
            &recovered_master,
            &recovered_partials,
            &receivers,
            threshold
        )
        .is_ok())
    }

    #[test]
    fn dealing_roundtrip() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = setup();

        let parties = 5;
        let threshold = ((parties as f32 * 2.) / 3. + 1.) as Threshold;
        let node_indices = (1..=parties).collect::<Vec<_>>();
        let epoch = Epoch::new(2);

        let mut receivers = BTreeMap::new();
        for index in &node_indices {
            let (_, pk) = keygen(&params, &mut rng);
            receivers.insert(*index, *pk.public_key());
        }

        let (dealing, _) = Dealing::create(
            &mut rng,
            &params,
            node_indices[0],
            threshold,
            epoch,
            &receivers,
            None,
        );

        let bytes = dealing.to_bytes();
        let recovered = Dealing::try_from_bytes(&bytes).unwrap();
        assert_eq!(dealing, recovered);
    }
}
