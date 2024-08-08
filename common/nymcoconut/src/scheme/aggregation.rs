// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::iter::Sum;
use core::ops::Mul;

use bls12_381::{G2Prepared, G2Projective, Scalar};
use group::Curve;
use itertools::Itertools;

use crate::error::{CoconutError, Result};
use crate::scheme::verification::check_bilinear_pairing;
use crate::scheme::{PartialSignature, Signature, SignatureShare, SignerIndex, VerificationKey};
use crate::utils::perform_lagrangian_interpolation_at_origin;
use crate::{Attribute, Parameters, VerificationKeyShare};

pub(crate) trait Aggregatable: Sized {
    fn aggregate(aggregatable: &[Self], indices: Option<&[SignerIndex]>) -> Result<Self>;

    fn check_unique_indices(indices: &[SignerIndex]) -> bool {
        // if aggregation is a threshold one, all indices should be unique
        indices.iter().unique_by(|&index| index).count() == indices.len()
    }
}

// includes `VerificationKey`
impl<T> Aggregatable for T
where
    T: Sum,
    for<'a> T: Sum<&'a T>,
    for<'a> &'a T: Mul<Scalar, Output = T>,
{
    fn aggregate(aggregatable: &[T], indices: Option<&[u64]>) -> Result<T> {
        if aggregatable.is_empty() {
            return Err(CoconutError::Aggregation("Empty set of values".to_string()));
        }

        if let Some(indices) = indices {
            if !Self::check_unique_indices(indices) {
                return Err(CoconutError::Aggregation("Non-unique indices".to_string()));
            }
            perform_lagrangian_interpolation_at_origin(indices, aggregatable)
        } else {
            // non-threshold
            Ok(aggregatable.iter().sum())
        }
    }
}

impl Aggregatable for PartialSignature {
    fn aggregate(sigs: &[PartialSignature], indices: Option<&[u64]>) -> Result<Signature> {
        let h = sigs
            .first()
            .ok_or_else(|| CoconutError::Aggregation("Empty set of signatures".to_string()))?
            .sig1();

        // TODO: is it possible to avoid this allocation?
        let sigmas = sigs.iter().map(|sig| *sig.sig2()).collect::<Vec<_>>();
        let aggr_sigma = Aggregatable::aggregate(&sigmas, indices)?;

        Ok(Signature(*h, aggr_sigma))
    }
}

/// Ensures all provided verification keys were generated to verify the same number of attributes.
fn check_same_key_size(keys: &[VerificationKey]) -> bool {
    keys.iter().map(|vk| vk.beta_g1.len()).all_equal()
        && keys.iter().map(|vk| vk.beta_g2.len()).all_equal()
}

pub fn aggregate_verification_keys(
    keys: &[VerificationKey],
    indices: Option<&[SignerIndex]>,
) -> Result<VerificationKey> {
    if !check_same_key_size(keys) {
        return Err(CoconutError::Aggregation(
            "Verification keys are of different sizes".to_string(),
        ));
    }
    Aggregatable::aggregate(keys, indices)
}

pub fn aggregate_key_shares(shares: &[VerificationKeyShare]) -> Result<VerificationKey> {
    let (keys, indices): (Vec<_>, Vec<_>) = shares
        .iter()
        .map(|share| (share.key.clone(), share.index))
        .unzip();

    aggregate_verification_keys(&keys, Some(&indices))
}

pub fn aggregate_signatures(
    signatures: &[PartialSignature],
    indices: Option<&[SignerIndex]>,
) -> Result<Signature> {
    Aggregatable::aggregate(signatures, indices)
}

pub fn aggregate_signatures_and_verify(
    params: &Parameters,
    verification_key: &VerificationKey,
    attributes: &[&Attribute],
    signatures: &[PartialSignature],
    indices: Option<&[SignerIndex]>,
) -> Result<Signature> {
    // aggregate the signature
    let signature = aggregate_signatures(signatures, indices)?;

    // Verify the signature
    let alpha = verification_key.alpha;

    let tmp = attributes
        .iter()
        .zip(verification_key.beta_g2.iter())
        .map(|(&attr, beta_i)| beta_i * attr)
        .sum::<G2Projective>();

    if bool::from(signature.0.is_identity()){
        return Err(CoconutError::Aggregation(
            "Verification of the aggregated signature failed - h is an identity point".to_string(),
        ));
    }
    if !check_bilinear_pairing(
        &signature.0.to_affine(),
        &G2Prepared::from((alpha + tmp).to_affine()),
        &signature.1.to_affine(),
        params.prepared_miller_g2(),
    ) {
        return Err(CoconutError::Aggregation(
            "Verification of the aggregated signature failed".to_string(),
        ));
    }
    Ok(signature)
}

pub fn aggregate_signature_shares(shares: &[SignatureShare]) -> Result<Signature> {
    let (signatures, indices): (Vec<_>, Vec<_>) = shares
        .iter()
        .map(|share| (*share.signature(), share.index()))
        .unzip();

    aggregate_signatures(&signatures, Some(&indices))
}

pub fn aggregate_signature_shares_and_verify(
    params: &Parameters,
    verification_key: &VerificationKey,
    attributes: &[&Attribute],
    shares: &[SignatureShare],
) -> Result<Signature> {
    let (signatures, indices): (Vec<_>, Vec<_>) = shares
        .iter()
        .map(|share| (*share.signature(), share.index()))
        .unzip();

    aggregate_signatures_and_verify(
        params,
        verification_key,
        attributes,
        &signatures,
        Some(&indices),
    )
}

#[cfg(test)]
mod tests {
    use crate::scheme::issuance::sign;
    use crate::scheme::keygen::ttp_keygen;
    use crate::scheme::verification::verify;
    use crate::tests::helpers::random_scalars_refs;
    use bls12_381::G1Projective;
    use group::Group;

    use super::*;

    #[test]
    fn key_aggregation_works_for_any_subset_of_keys() {
        let params = Parameters::new(2).unwrap();
        let keypairs = ttp_keygen(&params, 3, 5).unwrap();

        let vks = keypairs
            .into_iter()
            .map(|keypair| keypair.verification_key().clone())
            .collect::<Vec<_>>();

        let aggr_vk1 = aggregate_verification_keys(&vks[..3], Some(&[1, 2, 3])).unwrap();
        let aggr_vk2 = aggregate_verification_keys(&vks[2..], Some(&[3, 4, 5])).unwrap();

        assert_eq!(aggr_vk1, aggr_vk2);

        // TODO: should those two actually work or not?
        // aggregating threshold+1
        let aggr_more = aggregate_verification_keys(&vks[1..], Some(&[2, 3, 4, 5])).unwrap();
        assert_eq!(aggr_vk1, aggr_more);

        // aggregating all
        let aggr_all = aggregate_verification_keys(&vks, Some(&[1, 2, 3, 4, 5])).unwrap();
        assert_eq!(aggr_all, aggr_vk1);

        // not taking enough points (threshold was 3)
        let aggr_not_enough = aggregate_verification_keys(&vks[..2], Some(&[1, 2])).unwrap();
        assert_ne!(aggr_not_enough, aggr_vk1);

        // taking wrong index
        let aggr_bad = aggregate_verification_keys(&vks[2..], Some(&[42, 123, 100])).unwrap();
        assert_ne!(aggr_vk1, aggr_bad);
    }

    #[test]
    fn key_aggregation_doesnt_work_for_empty_set_of_keys() {
        let keys: Vec<VerificationKey> = vec![];
        assert!(aggregate_verification_keys(&keys, None).is_err());
    }

    #[test]
    fn key_aggregation_doesnt_work_if_indices_have_invalid_length() {
        let keys = vec![VerificationKey::identity(3)];

        assert!(aggregate_verification_keys(&keys, Some(&[])).is_err());
        assert!(aggregate_verification_keys(&keys, Some(&[1, 2])).is_err());
    }

    #[test]
    fn key_aggregation_doesnt_work_for_non_unique_indices() {
        let keys = vec![VerificationKey::identity(3), VerificationKey::identity(3)];

        assert!(aggregate_verification_keys(&keys, Some(&[1, 1])).is_err());
    }

    #[test]
    fn key_aggregation_doesnt_work_for_keys_of_different_size() {
        let keys = vec![VerificationKey::identity(3), VerificationKey::identity(1)];

        assert!(aggregate_verification_keys(&keys, None).is_err())
    }

    #[test]
    fn signature_aggregation_works_for_any_subset_of_signatures() {
        let params = Parameters::new(2).unwrap();
        random_scalars_refs!(attributes, params, 2);

        let keypairs = ttp_keygen(&params, 3, 5).unwrap();

        let (sks, vks): (Vec<_>, Vec<_>) = keypairs
            .into_iter()
            .map(|keypair| {
                (
                    keypair.secret_key().clone(),
                    keypair.verification_key().clone(),
                )
            })
            .unzip();

        let sigs = sks
            .iter()
            .map(|sk| sign(&params, sk, &attributes).unwrap())
            .collect::<Vec<_>>();

        // aggregating (any) threshold works
        let aggr_vk_1 = aggregate_verification_keys(&vks[..3], Some(&[1, 2, 3])).unwrap();
        let aggr_sig1 = aggregate_signatures_and_verify(
            &params,
            &aggr_vk_1,
            &attributes,
            &sigs[..3],
            Some(&[1, 2, 3]),
        )
        .unwrap();

        let aggr_vk_2 = aggregate_verification_keys(&vks[2..], Some(&[3, 4, 5])).unwrap();
        let aggr_sig2 = aggregate_signatures_and_verify(
            &params,
            &aggr_vk_1,
            &attributes,
            &sigs[2..],
            Some(&[3, 4, 5]),
        )
        .unwrap();
        assert_eq!(aggr_sig1, aggr_sig2);

        // verify credential for good measure
        assert!(verify(&params, &aggr_vk_1, &attributes, &aggr_sig1));
        assert!(verify(&params, &aggr_vk_2, &attributes, &aggr_sig2));

        // aggregating threshold+1 works
        let aggr_vk_more = aggregate_verification_keys(&vks[1..], Some(&[2, 3, 4, 5])).unwrap();
        let aggr_more = aggregate_signatures_and_verify(
            &params,
            &aggr_vk_more,
            &attributes,
            &sigs[1..],
            Some(&[2, 3, 4, 5]),
        )
        .unwrap();
        assert_eq!(aggr_sig1, aggr_more);

        // aggregating all
        let aggr_vk_all = aggregate_verification_keys(&vks, Some(&[1, 2, 3, 4, 5])).unwrap();
        let aggr_all = aggregate_signatures_and_verify(
            &params,
            &aggr_vk_all,
            &attributes,
            &sigs,
            Some(&[1, 2, 3, 4, 5]),
        )
        .unwrap();
        assert_eq!(aggr_all, aggr_sig1);

        // not taking enough points (threshold was 3) should fail
        let aggr_vk_not_enough = aggregate_verification_keys(&vks[..2], Some(&[1, 2])).unwrap();
        let aggr_not_enough = aggregate_signatures_and_verify(
            &params,
            &aggr_vk_not_enough,
            &attributes,
            &sigs[..2],
            Some(&[1, 2]),
        )
        .unwrap();
        assert_ne!(aggr_not_enough, aggr_sig1);

        // taking wrong index should fail
        let aggr_vk_bad = aggregate_verification_keys(&vks[2..], Some(&[1, 2, 3])).unwrap();
        assert!(aggregate_signatures_and_verify(
            &params,
            &aggr_vk_bad,
            &attributes,
            &sigs[2..],
            Some(&[42, 123, 100]),
        )
        .is_err());
    }

    fn random_signature() -> Signature {
        let mut rng = rand::thread_rng();
        Signature(
            G1Projective::random(&mut rng),
            G1Projective::random(&mut rng),
        )
    }

    #[test]
    fn signature_aggregation_doesnt_work_for_empty_set_of_signatures() {
        let signatures: Vec<Signature> = vec![];
        let params = Parameters::new(2).unwrap();
        random_scalars_refs!(attributes, params, 2);
        let keypairs = ttp_keygen(&params, 3, 5).unwrap();

        let (_, vks): (Vec<_>, Vec<_>) = keypairs
            .into_iter()
            .map(|keypair| {
                (
                    keypair.secret_key().clone(),
                    keypair.verification_key().clone(),
                )
            })
            .unzip();

        let aggr_vk_all = aggregate_verification_keys(&vks, None).unwrap();
        assert!(aggregate_signatures_and_verify(
            &params,
            &aggr_vk_all,
            &attributes,
            &signatures,
            None
        )
        .is_err());
    }

    #[test]
    fn signature_aggregation_doesnt_work_if_indices_have_invalid_length() {
        let signatures = vec![random_signature()];
        let params = Parameters::new(2).unwrap();
        random_scalars_refs!(attributes, params, 2);
        let keypairs = ttp_keygen(&params, 3, 5).unwrap();
        let (_, vks): (Vec<_>, Vec<_>) = keypairs
            .into_iter()
            .map(|keypair| {
                (
                    keypair.secret_key().clone(),
                    keypair.verification_key().clone(),
                )
            })
            .unzip();
        let aggr_vk_all = aggregate_verification_keys(&vks, None).unwrap();

        assert!(aggregate_signatures_and_verify(
            &params,
            &aggr_vk_all,
            &attributes,
            &signatures,
            Some(&[])
        )
        .is_err());
        assert!(aggregate_signatures_and_verify(
            &params,
            &aggr_vk_all,
            &attributes,
            &signatures,
            Some(&[1, 2]),
        )
        .is_err());
    }

    #[test]
    fn signature_aggregation_doesnt_work_for_non_unique_indices() {
        let signatures = vec![random_signature(), random_signature()];
        let params = Parameters::new(2).unwrap();
        random_scalars_refs!(attributes, params, 2);
        let keypairs = ttp_keygen(&params, 3, 5).unwrap();
        let (_, vks): (Vec<_>, Vec<_>) = keypairs
            .into_iter()
            .map(|keypair| {
                (
                    keypair.secret_key().clone(),
                    keypair.verification_key().clone(),
                )
            })
            .unzip();
        let aggr_vk_all = aggregate_verification_keys(&vks, None).unwrap();

        assert!(aggregate_signatures_and_verify(
            &params,
            &aggr_vk_all,
            &attributes,
            &signatures,
            Some(&[1, 1]),
        )
        .is_err());
    }

    // TODO: test for aggregating non-threshold keys
}
