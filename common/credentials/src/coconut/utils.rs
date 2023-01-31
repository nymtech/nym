// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_interface::{
    aggregate_signature_shares, aggregate_verification_keys, prove_bandwidth_credential, Attribute,
    BlindedSignature, Credential, Parameters, Signature, SignatureShare, VerificationKey,
};
use crypto::asymmetric::encryption::PublicKey;
use crypto::shared_key::recompute_shared_key;
use crypto::symmetric::stream_cipher;
use nym_api_requests::coconut::BlindSignRequestBody;
use validator_client::client::CoconutApiClient;

use crate::coconut::bandwidth::{BandwidthVoucher, PRIVATE_ATTRIBUTES, PUBLIC_ATTRIBUTES};
use crate::coconut::params::{NymApiCredentialEncryptionAlgorithm, NymApiCredentialHkdfAlgorithm};
use crate::error::Error;

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
    attributes: &BandwidthVoucher,
    client: &validator_client::client::NymApiClient,
    validator_vk: &VerificationKey,
) -> Result<Signature, Error> {
    let public_attributes = attributes.get_public_attributes();
    let public_attributes_plain = attributes.get_public_attributes_plain();
    let private_attributes = attributes.get_private_attributes();
    let blind_sign_request = attributes.blind_sign_request();

    let response = if attributes.use_request() {
        let blind_sign_request_body = BlindSignRequestBody::new(
            blind_sign_request,
            attributes.tx_hash().to_string(),
            attributes.sign(blind_sign_request).to_base58_string(),
            &public_attributes,
            public_attributes_plain,
            (public_attributes.len() + private_attributes.len()) as u32,
        );
        client.blind_sign(&blind_sign_request_body).await?
    } else {
        client
            .partial_bandwidth_credential(&attributes.tx_hash().to_string())
            .await?
    };
    let encrypted_signature = response.encrypted_signature;
    let remote_key = PublicKey::from_bytes(&response.remote_key)?;

    let encryption_key = recompute_shared_key::<
        NymApiCredentialEncryptionAlgorithm,
        NymApiCredentialHkdfAlgorithm,
    >(&remote_key, attributes.encryption_key());
    let zero_iv = stream_cipher::zero_iv::<NymApiCredentialEncryptionAlgorithm>();
    let blinded_signature_bytes = stream_cipher::decrypt::<NymApiCredentialEncryptionAlgorithm>(
        &encryption_key,
        &zero_iv,
        &encrypted_signature,
    );

    let blinded_signature = BlindedSignature::from_bytes(&blinded_signature_bytes)?;

    let unblinded_signature = blinded_signature.unblind(
        params,
        validator_vk,
        &private_attributes,
        &public_attributes,
        &blind_sign_request.get_commitment_hash(),
        attributes.pedersen_commitments_openings(),
    )?;

    Ok(unblinded_signature)
}

pub async fn obtain_aggregate_signature(
    params: &Parameters,
    attributes: &BandwidthVoucher,
    coconut_api_clients: &[CoconutApiClient],
) -> Result<Signature, Error> {
    if coconut_api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }
    let public_attributes = attributes.get_public_attributes();
    let private_attributes = attributes.get_private_attributes();

    let mut shares = Vec::with_capacity(coconut_api_clients.len());
    let validators_partial_vks: Vec<_> = coconut_api_clients
        .iter()
        .map(|api_client| api_client.verification_key.clone())
        .collect();
    let indices: Vec<_> = coconut_api_clients
        .iter()
        .map(|api_client| api_client.node_id)
        .collect();

    for coconut_api_client in coconut_api_clients.iter() {
        let signature = obtain_partial_credential(
            params,
            attributes,
            &coconut_api_client.api_client,
            &coconut_api_client.verification_key,
        )
        .await?;
        let share = SignatureShare::new(signature, coconut_api_client.node_id);
        shares.push(share)
    }

    let mut attributes = Vec::with_capacity(private_attributes.len() + public_attributes.len());
    attributes.extend_from_slice(&private_attributes);
    attributes.extend_from_slice(&public_attributes);

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
#[allow(clippy::too_many_arguments)]
pub fn prepare_credential_for_spending(
    params: &Parameters,
    voucher_value: u64,
    voucher_info: String,
    serial_number: Attribute,
    binding_number: Attribute,
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
        PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES,
        theta,
        voucher_value,
        voucher_info,
        epoch_id,
    ))
}
