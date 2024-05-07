// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::iter::Sum;
use core::ops::Mul;

use bls12_381::{G2Prepared, G2Projective, Scalar};
use group::Curve;
use itertools::Itertools;

use crate::common_types::{PartialSignature, Signature, SignatureShare, SignerIndex};
use crate::error::{CompactEcashError, Result};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::withdrawal::RequestInfo;
use crate::scheme::{PartialWallet, Wallet};
use crate::utils::{check_bilinear_pairing, perform_lagrangian_interpolation_at_origin};
use crate::{ecash_group_parameters, Attribute};

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
    for<'a> &'a T: Mul<Scalar, Output = T>,
{
    fn aggregate(aggregatable: &[T], indices: Option<&[u64]>) -> Result<T> {
        if aggregatable.is_empty() {
            return Err(CompactEcashError::AggregationEmptySet);
        }

        if let Some(indices) = indices {
            if !Self::check_unique_indices(indices) {
                return Err(CompactEcashError::AggregationDuplicateIndices);
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
            .ok_or(CompactEcashError::AggregationEmptySet)?
            .sig1();

        // TODO: is it possible to avoid this allocation?
        let sigmas = sigs.iter().map(|sig| *sig.sig2()).collect::<Vec<_>>();
        let aggr_sigma = Aggregatable::aggregate(&sigmas, indices)?;

        Ok(Signature {
            h: *h,
            s: aggr_sigma,
        })
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
        return Err(CompactEcashError::AggregationSizeMismatch);
    }
    Aggregatable::aggregate(keys, indices)
}

pub fn aggregate_signature_shares(
    verification_key: &VerificationKeyAuth,
    attributes: &[Attribute],
    shares: &[SignatureShare],
) -> Result<Signature> {
    let (signatures, indices): (Vec<_>, Vec<_>) = shares
        .iter()
        .map(|share| (*share.signature(), share.index()))
        .unzip();

    aggregate_signatures(verification_key, attributes, &signatures, Some(&indices))
}

pub fn aggregate_signatures(
    verification_key: &VerificationKeyAuth,
    attributes: &[Attribute],
    signatures: &[PartialSignature],
    indices: Option<&[SignerIndex]>,
) -> Result<Signature> {
    let params = ecash_group_parameters();
    // aggregate the signature

    let signature = match Aggregatable::aggregate(signatures, indices) {
        Ok(res) => res,
        Err(err) => return Err(err),
    };

    // Verify the signature
    let tmp = attributes
        .iter()
        .zip(verification_key.beta_g2.iter())
        .map(|(attr, beta_i)| beta_i * attr)
        .sum::<G2Projective>();

    if !check_bilinear_pairing(
        &signature.h.to_affine(),
        &G2Prepared::from((verification_key.alpha + tmp).to_affine()),
        &signature.s.to_affine(),
        params.prepared_miller_g2(),
    ) {
        return Err(CompactEcashError::AggregationVerification);
    }
    Ok(signature)
}

pub fn aggregate_wallets(
    verification_key: &VerificationKeyAuth,
    sk_user: &SecretKeyUser,
    wallets: &[PartialWallet],
    req_info: &RequestInfo,
) -> Result<Wallet> {
    // Aggregate partial wallets
    let signature_shares: Vec<SignatureShare> = wallets
        .iter()
        .map(|wallet| SignatureShare::new(*wallet.signature(), wallet.index()))
        .collect();

    let attributes = vec![
        sk_user.sk,
        *req_info.get_v(),
        *req_info.get_expiration_date(),
    ];
    let aggregated_signature =
        aggregate_signature_shares(verification_key, &attributes, &signature_shares)?;

    Ok(Wallet {
        sig: aggregated_signature,
        v: *req_info.get_v(),
        expiration_date: *req_info.get_expiration_date(),
        l: 0,
    })
}
