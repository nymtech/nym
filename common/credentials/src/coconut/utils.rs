// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::Scalar;
use coconut_interface::{
    aggregate_signature_shares, aggregate_verification_keys, prepare_blind_sign,
    prove_bandwidth_credential, Attribute, BlindSignRequest, BlindSignRequestBody, Credential,
    Parameters, Signature, SignatureShare, VerificationKey,
};
use url::Url;

use crate::coconut::bandwidth::{BandwidthVoucher, PRIVATE_ATTRIBUTES};
use crate::error::Error;

/// Contacts all provided validators and then aggregate their verification keys.
///
/// # Arguments
///
/// * `validators`: list of validators to obtain verification keys from.
///
/// Note: list of validators must be correctly ordered by the polynomial coordinates used
/// during key generation and it is responsibility of the caller to ensure that correct
/// number of them is provided
///
/// # Examples
///
/// ```no_run
/// use url::{Url, ParseError};
/// use credentials::obtain_aggregate_verification_key;
///
/// async fn example() -> Result<(), ParseError> {
///     let validators = vec!["https://sandbox-validator1.nymtech.net/api".parse()?, "https://sandbox-validator2.nymtech.net/api".parse()?];
///     let aggregated_key = obtain_aggregate_verification_key(&validators).await;
///     // deal with the obtained Result
///     Ok(())
/// }
/// ```
pub async fn obtain_aggregate_verification_key(
    validators: &[Url],
) -> Result<VerificationKey, Error> {
    if validators.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }

    let mut indices = Vec::with_capacity(validators.len());
    let mut shares = Vec::with_capacity(validators.len());

    let mut client = validator_client::ApiClient::new(validators[0].clone());
    let response = client.get_coconut_verification_key().await?;

    indices.push(1);
    shares.push(response.key);

    for (id, validator_url) in validators.iter().enumerate().skip(1) {
        client.change_validator_api(validator_url.clone());
        let response = client.get_coconut_verification_key().await?;
        indices.push((id + 1) as u64);
        shares.push(response.key);
    }

    Ok(aggregate_verification_keys(&shares, Some(&indices))?)
}

async fn obtain_partial_credential(
    params: &Parameters,
    attributes: &BandwidthVoucher,
    pedersen_commitments_openings: &[Scalar],
    blind_sign_request: &BlindSignRequest,
    client: &validator_client::ApiClient,
    validator_vk: &VerificationKey,
) -> Result<Signature, Error> {
    let public_attributes = attributes.get_public_attributes();
    let private_attributes = attributes.get_private_attributes();
    let blind_sign_request_body = BlindSignRequestBody::new(
        blind_sign_request,
        attributes.tx_hash().to_string(),
        attributes.sign(blind_sign_request),
        &public_attributes,
        (public_attributes.len() + private_attributes.len()) as u32,
    );

    let blinded_signature = client
        .blind_sign(&blind_sign_request_body)
        .await?
        .blinded_signature;

    let unblinded_signature = blinded_signature.unblind(
        params,
        validator_vk,
        &private_attributes,
        &public_attributes,
        &blind_sign_request.get_commitment_hash(),
        &*pedersen_commitments_openings,
    )?;

    Ok(unblinded_signature)
}

pub async fn obtain_aggregate_signature(
    params: &Parameters,
    attributes: &BandwidthVoucher,
    validators: &[Url],
) -> Result<Signature, Error> {
    if validators.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }
    let public_attributes = attributes.get_public_attributes();
    let private_attributes = attributes.get_private_attributes();

    let mut shares = Vec::with_capacity(validators.len());
    let mut validators_partial_vks: Vec<VerificationKey> = Vec::with_capacity(validators.len());

    let mut client = validator_client::ApiClient::new(validators[0].clone());
    let validator_partial_vk = client.get_coconut_verification_key().await?;
    validators_partial_vks.push(validator_partial_vk.key.clone());

    let (pedersen_commitments_openings, blind_sign_request) =
        prepare_blind_sign(params, &private_attributes, &public_attributes)?;

    let first = obtain_partial_credential(
        params,
        attributes,
        &pedersen_commitments_openings,
        &blind_sign_request,
        &client,
        &validator_partial_vk.key,
    )
    .await?;
    shares.push(SignatureShare::new(first, 1));

    for (id, validator_url) in validators.iter().enumerate().skip(1) {
        client.change_validator_api(validator_url.clone());
        let validator_partial_vk = client.get_coconut_verification_key().await?;
        validators_partial_vks.push(validator_partial_vk.key.clone());
        let signature = obtain_partial_credential(
            params,
            attributes,
            &pedersen_commitments_openings,
            &blind_sign_request,
            &client,
            &validator_partial_vk.key,
        )
        .await?;
        let share = SignatureShare::new(signature, (id + 1) as u64);
        shares.push(share)
    }

    let mut attributes = Vec::with_capacity(private_attributes.len() + public_attributes.len());
    attributes.extend_from_slice(&private_attributes);
    attributes.extend_from_slice(&public_attributes);

    let mut indices: Vec<u64> = Vec::with_capacity(validators_partial_vks.len());
    for i in 0..validators_partial_vks.len() {
        indices.push((i + 1) as u64);
    }
    let verification_key =
        aggregate_verification_keys(&validators_partial_vks, Some(indices.as_ref()))?;

    Ok(aggregate_signature_shares(
        params,
        &verification_key,
        &attributes,
        &shares,
    )?)
}

// TODO: better type flow
pub fn prepare_credential_for_spending(
    params: &Parameters,
    public_attributes: Vec<Vec<u8>>,
    serial_number: Attribute,
    binding_number: Attribute,
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
        (public_attributes.len() + PRIVATE_ATTRIBUTES as usize) as u32,
        theta,
        public_attributes,
        signature,
    ))
}
