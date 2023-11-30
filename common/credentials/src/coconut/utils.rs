// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::bandwidth::BandwidthVoucher;
use crate::error::Error;
use log::{debug, warn};
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_coconut_interface::{
    aggregate_signature_shares, aggregate_verification_keys, prove_bandwidth_credential, Attribute,
    Credential, Parameters, Signature, SignatureShare, VerificationKey,
};
use nym_validator_client::client::CoconutApiClient;

pub async fn obtain_aggregate_verification_key(
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

async fn obtain_partial_credential(
    params: &Parameters,
    voucher: &BandwidthVoucher,
    client: &nym_validator_client::client::NymApiClient,
    validator_vk: &VerificationKey,
) -> Result<Signature, Error> {
    let public_attributes_plain = voucher.get_public_attributes_plain();
    let blind_sign_request = voucher.blind_sign_request();
    let request_signature = voucher.sign();

    let blind_sign_request_body = BlindSignRequestBody::new(
        blind_sign_request.clone(),
        voucher.tx_hash(),
        request_signature,
        public_attributes_plain,
    );
    let response = client.blind_sign(&blind_sign_request_body).await?;

    let blinded_signature = response.blinded_signature;

    let public_attributes = voucher.get_public_attributes();
    let private_attributes = voucher.get_private_attributes();

    let unblinded_signature = blinded_signature.unblind_and_verify(
        params,
        validator_vk,
        &private_attributes,
        &public_attributes,
        &blind_sign_request.get_commitment_hash(),
        voucher.pedersen_commitments_openings(),
    )?;

    Ok(unblinded_signature)
}

pub async fn obtain_aggregate_signature(
    params: &Parameters,
    voucher: &BandwidthVoucher,
    coconut_api_clients: &[CoconutApiClient],
    threshold: u64,
) -> Result<Signature, Error> {
    if coconut_api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }
    let mut shares = Vec::with_capacity(coconut_api_clients.len());
    let validators_partial_vks: Vec<_> = coconut_api_clients
        .iter()
        .map(|api_client| api_client.verification_key.clone())
        .collect();
    let indices: Vec<_> = coconut_api_clients
        .iter()
        .map(|api_client| api_client.node_id)
        .collect();
    let verification_key =
        aggregate_verification_keys(&validators_partial_vks, Some(indices.as_ref()))?;

    for coconut_api_client in coconut_api_clients.iter() {
        debug!(
            "attempting to obtain partial credential from {}",
            coconut_api_client.api_client.api_url()
        );

        match obtain_partial_credential(
            params,
            voucher,
            &coconut_api_client.api_client,
            &coconut_api_client.verification_key,
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

    let public_attributes = voucher.get_public_attributes();
    let private_attributes = voucher.get_private_attributes();

    let mut attributes = Vec::with_capacity(private_attributes.len() + public_attributes.len());
    attributes.extend_from_slice(&private_attributes);
    attributes.extend_from_slice(&public_attributes);

    aggregate_signature_shares(params, &verification_key, &attributes, &shares)
        .map_err(Error::SignatureAggregationError)
}

// TODO: better type flow
#[allow(clippy::too_many_arguments)]
pub fn prepare_credential_for_spending(
    params: &Parameters,
    voucher_value: u64,
    voucher_info: String,
    serial_number: &Attribute,
    binding_number: &Attribute,
    epoch_id: u64,
    signature: &Signature,
    verification_key: &VerificationKey,
) -> Result<Credential, Error> {
    let theta = prove_bandwidth_credential(
        params,
        verification_key,
        signature,
        serial_number,
        binding_number,
    )?;

    Ok(Credential::new(
        BandwidthVoucher::ENCODED_ATTRIBUTES,
        theta,
        voucher_value,
        voucher_info,
        epoch_id,
    ))
}
