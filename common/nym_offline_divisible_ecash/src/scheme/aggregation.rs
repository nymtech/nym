use core::iter::Sum;
use core::ops::Mul;
use std::cell::Cell;

use bls12_381::{G2Prepared, G2Projective, pairing, Scalar};
use group::Curve;
use itertools::Itertools;

use crate::Attribute;
use crate::error::{DivisibleEcashError, Result};
use crate::scheme::{PartialWallet, Wallet};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::GroupParameters;
use crate::utils::{
    check_bilinear_pairing, PartialSignature, perform_lagrangian_interpolation_at_origin,
    Signature, SignatureShare, SignerIndex,
};

pub(crate) trait Aggregatable: Sized {
    fn aggregate(aggregatable: &[Self], indices: Option<&[SignerIndex]>) -> Result<Self>;

    fn check_unique_indices(indices: &[SignerIndex]) -> bool {
        // if aggregation is a threshold one, all indices should be unique
        indices.iter().unique_by(|&index| index).count() == indices.len()
    }
}

impl<T> Aggregatable for T
    where
        T: Sum,
        for<'a> T: Sum<&'a T>,
        for<'a> &'a T: Mul<Scalar, Output=T>,
{
    fn aggregate(aggregatable: &[T], indices: Option<&[u64]>) -> Result<T> {
        if aggregatable.is_empty() {
            return Err(DivisibleEcashError::Aggregation(
                "Empty set of values".to_string(),
            ));
        }

        if let Some(indices) = indices {
            if !Self::check_unique_indices(indices) {
                return Err(DivisibleEcashError::Aggregation(
                    "Non-unique indices".to_string(),
                ));
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
            .get(0)
            .ok_or_else(|| DivisibleEcashError::Aggregation("Empty set of signatures".to_string()))?
            .sig1();

        // TODO: is it possible to avoid this allocation?
        let sigmas = sigs.iter().map(|sig| *sig.sig2()).collect::<Vec<_>>();
        let aggr_sigma = Aggregatable::aggregate(&sigmas, indices)?;

        Ok(Signature(*h, aggr_sigma))
    }
}

/// Ensures all provided verification keys were generated to verify the same number of attributes.
fn check_same_key_size(keys: &[VerificationKeyAuth]) -> bool {
    keys.iter().map(|vk| vk.beta_g1.len()).all_equal()
        && keys.iter().map(|vk| vk.beta_g2.len()).all_equal()
}

pub fn aggregate_verification_keys(
    keys: &[VerificationKeyAuth],
    indices: Option<&[SignerIndex]>,
) -> Result<VerificationKeyAuth> {
    if !check_same_key_size(keys) {
        return Err(DivisibleEcashError::Aggregation(
            "Verification keys are of different sizes".to_string(),
        ));
    }
    Aggregatable::aggregate(keys, indices)
}

pub fn aggregate_signature_shares(
    params: &GroupParameters,
    verification_key: &VerificationKeyAuth,
    attributes: &[Attribute],
    shares: &[SignatureShare],
) -> Result<Signature> {
    let (signatures, indices): (Vec<_>, Vec<_>) = shares
        .iter()
        .map(|share| (*share.signature(), share.index()))
        .unzip();

    aggregate_signatures(
        params,
        verification_key,
        attributes,
        &signatures,
        Some(&indices),
    )
}

pub fn aggregate_signatures(
    params: &GroupParameters,
    verification_key: &VerificationKeyAuth,
    attributes: &[Attribute],
    signatures: &[PartialSignature],
    indices: Option<&[SignerIndex]>,
) -> Result<Signature> {
    // aggregate the signature

    let signature = match Aggregatable::aggregate(signatures, indices) {
        Ok(res) => res,
        Err(err) => return Err(err),
    };

    // Verify the signature
    let alpha = verification_key.alpha;

    let tmp = attributes
        .iter()
        .zip(verification_key.beta_g2.iter())
        .map(|(attr, beta_i)| beta_i * attr)
        .sum::<G2Projective>();

    if !check_bilinear_pairing(
        &signature.0.to_affine(),
        &G2Prepared::from((alpha + tmp).to_affine()),
        &signature.1.to_affine(),
        params.prepared_miller_g2(),
    ) {
        return Err(DivisibleEcashError::Aggregation(
            "Verification of the aggregated signature failed".to_string(),
        ));
    }
    Ok(signature)
}

pub fn aggregate_wallets(
    grp: &GroupParameters,
    verification_key: &VerificationKeyAuth,
    sk_user: &SecretKeyUser,
    wallets: &[PartialWallet],
) -> Result<Wallet> {
    let signature_shares: Vec<SignatureShare> = wallets
        .iter()
        .enumerate()
        .map(|(idx, wallet)| SignatureShare::new(*wallet.signature(), (idx + 1) as u64))
        .collect();

    let v = wallets.get(0).unwrap().v;
    let attributes = vec![sk_user.sk, v];
    let aggregated_signature =
        aggregate_signature_shares(&grp, &verification_key, &attributes, &signature_shares)?;

    Ok(Wallet {
        sig: aggregated_signature,
        v,
        l: Cell::new(1),
    })
}

