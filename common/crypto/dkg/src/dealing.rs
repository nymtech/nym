// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::proof_chunking::ProofOfChunking;
use crate::bte::proof_sharing::ProofOfSecretSharing;
use crate::bte::{
    encrypt_shares, proof_chunking, proof_sharing, Ciphertexts, Params, PublicKey, Tau,
};
use crate::error::DkgError;
use crate::interpolation::polynomial::{Polynomial, PublicCoefficients};
use crate::{NodeIndex, Share, Threshold};
use bls12_381::Scalar;
use rand_core::RngCore;
use std::collections::BTreeMap;
use zeroize::Zeroize;

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
        epoch: &Tau,
        // BTreeMap ensures the keys are sorted by their indices
        receivers: &BTreeMap<NodeIndex, PublicKey>,
    ) -> (Self, Option<Share>) {
        // TODO: perhaps this implies `Tau` should be somehow split to assert this via a stronger type?
        assert!(epoch.is_valid_epoch(params));
        assert!(threshold > 0);

        let polynomial = Polynomial::new_random(&mut rng, threshold - 1);
        let mut shares = receivers
            .keys()
            .map(|&node_index| polynomial.evaluate(&Scalar::from(node_index)).into())
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
}

impl Dealing {
    // rather than returning a bool for whether the dealing is valid or not, a Result is returned
    // instead so that we would have more information regarding a possible failure cause
    pub fn verify(
        &self,
        params: &Params,
        epoch: &Tau,
        threshold: Threshold,
        receivers: &BTreeMap<NodeIndex, PublicKey>,
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
        Ok(())
    }
}
