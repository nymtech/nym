// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::bandwidth::IssuanceBandwidthCredential;
use crate::error::Error;
use log::{debug, warn};
use nym_credentials_interface::{
    aggregate_verification_keys, Signature, SignatureShare, VerificationKey,
};
use nym_validator_client::client::CoconutApiClient;

pub fn obtain_aggregate_verification_key(
    api_clients: &[CoconutApiClient],
) -> Result<VerificationKey, Error> {
    if api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }

    let indices: Vec<_> = api_clients
        .iter()
        .map(|api_client| api_client.node_id)
        .collect();
    let shares: Vec<_> = api_clients
        .iter()
        .map(|api_client| api_client.verification_key.clone())
        .collect();

    Ok(aggregate_verification_keys(&shares, Some(&indices))?)
}

pub async fn obtain_aggregate_signature(
    voucher: &IssuanceBandwidthCredential,
    coconut_api_clients: &[CoconutApiClient],
    threshold: u64,
) -> Result<Signature, Error> {
    if coconut_api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }
    let mut shares = Vec::with_capacity(coconut_api_clients.len());
    let verification_key = obtain_aggregate_verification_key(coconut_api_clients)?;

    let request = voucher.prepare_for_signing();

    for coconut_api_client in coconut_api_clients.iter() {
        debug!(
            "attempting to obtain partial credential from {}",
            coconut_api_client.api_client.api_url()
        );

        match voucher
            .obtain_partial_bandwidth_voucher_credential(
                &coconut_api_client.api_client,
                &coconut_api_client.verification_key,
                Some(request.clone()),
            )
            .await
        {
            Ok(signature) => {
                let share = SignatureShare::new(signature, coconut_api_client.node_id);
                shares.push(share)
            }
            Err(err) => {
                warn!(
                    "failed to obtain partial credential from {}: {err}",
                    coconut_api_client.api_client.api_url()
                );
            }
        };
    }
    if shares.len() < threshold as usize {
        return Err(Error::NotEnoughShares);
    }

    voucher.aggregate_signature_shares(&verification_key, &shares)
}

pub(crate) mod scalar_serde_helper {
    use bls12_381::Scalar;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use zeroize::Zeroizing;

    pub fn serialize<S: Serializer>(scalar: &Scalar, serializer: S) -> Result<S::Ok, S::Error> {
        scalar.to_bytes().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Scalar, D::Error> {
        let b = <[u8; 32]>::deserialize(deserializer)?;

        // make sure the bytes get zeroed
        let bytes = Zeroizing::new(b);

        let maybe_scalar: Option<Scalar> = Scalar::from_bytes(&bytes).into();
        maybe_scalar.ok_or(serde::de::Error::custom(
            "did not construct a valid bls12-381 scalar out of the provided bytes",
        ))
    }
}
