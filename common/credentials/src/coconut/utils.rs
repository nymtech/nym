// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::bandwidth::BandwidthVoucher;
use crate::coconut::params::{NymApiCredentialEncryptionAlgorithm, NymApiCredentialHkdfAlgorithm};
use crate::error::Error;
use log::{debug, warn};
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_compact_ecash::scheme::Wallet;
use nym_compact_ecash::setup::GroupParameters;
use nym_compact_ecash::utils::BlindedSignature;
use nym_compact_ecash::{
    aggregate_verification_keys, aggregate_wallets, issue_verify, PartialWallet,
    VerificationKeyAuth,
};
use nym_crypto::asymmetric::encryption::PublicKey;
use nym_crypto::shared_key::recompute_shared_key;
use nym_crypto::symmetric::stream_cipher;
use nym_validator_client::client::CoconutApiClient;

pub async fn obtain_aggregate_verification_key(
    api_clients: &[CoconutApiClient],
) -> Result<VerificationKeyAuth, Error> {
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
    params: &GroupParameters,
    attributes: &BandwidthVoucher,
    client: &nym_validator_client::client::NymApiClient,
    validator_vk: &VerificationKeyAuth,
) -> Result<PartialWallet, Error> {
    //let public_attributes = attributes.get_public_attributes();
    let public_attributes_plain = attributes.get_public_attributes_plain();
    //let private_attributes = attributes.get_private_attributes();
    let withdrawal_request = attributes.withdrawal_request();

    let blind_sign_request_body = BlindSignRequestBody::new(
        withdrawal_request,
        attributes.tx_hash().to_string(),
        attributes.sign(withdrawal_request).to_base58_string(),
        attributes.ecash_keypair().public_key().to_base58_string(),
        // &public_attributes,
        public_attributes_plain.clone(),
        (public_attributes_plain.len()) as u32,
    );
    let response = client.blind_sign(&blind_sign_request_body).await?;
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

    let unblinded_signature = issue_verify(
        params,
        validator_vk,
        &attributes.ecash_keypair().secret_key(),
        &blinded_signature,
        attributes.withdrawal_request_info(),
    )?;

    Ok(unblinded_signature)
}

pub async fn obtain_aggregate_signature(
    params: &GroupParameters,
    attributes: &BandwidthVoucher,
    ecash_api_clients: &[CoconutApiClient],
    threshold: u64,
) -> Result<Wallet, Error> {
    if ecash_api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }
    //let public_attributes = attributes.get_public_attributes();
    //let private_attributes = attributes.get_private_attributes();

    let mut wallets = Vec::with_capacity(ecash_api_clients.len());
    let validators_partial_vks: Vec<_> = ecash_api_clients
        .iter()
        .map(|api_client| api_client.verification_key.clone())
        .collect();
    let indices: Vec<_> = ecash_api_clients
        .iter()
        .map(|api_client| api_client.node_id)
        .collect();
    let verification_key =
        aggregate_verification_keys(&validators_partial_vks, Some(indices.as_ref()))?;

    for coconut_api_client in ecash_api_clients.iter() {
        debug!(
            "attempting to obtain partial credential from {}",
            coconut_api_client.api_client.api_url()
        );

        match obtain_partial_credential(
            params,
            attributes,
            &coconut_api_client.api_client,
            &coconut_api_client.verification_key,
        )
        .await
        {
            Ok(wallet) => wallets.push(wallet),
            Err(err) => {
                warn!(
                    "failed to obtain partial credential from {}: {err}",
                    coconut_api_client.api_client.api_url()
                );
            }
        };
    }
    if wallets.len() < threshold as usize {
        return Err(Error::NotEnoughShares);
    }

    // let mut attributes = Vec::with_capacity(private_attributes.len() + public_attributes.len());
    // attributes.extend_from_slice(&private_attributes);
    // attributes.extend_from_slice(&public_attributes);

    aggregate_wallets(
        params,
        &verification_key,
        &attributes.ecash_keypair().secret_key(),
        &wallets,
        attributes.withdrawal_request_info(),
    )
    .map_err(Error::CompactEcashError)
}

// TODO: better type flow
// #[allow(clippy::too_many_arguments)]
// pub fn prepare_credential_for_spending(
//     params: &nym_coconut_interface::Parameters,
//     voucher_value: u64,
//     voucher_info: String,
//     serial_number: nym_coconut_interface::Attribute,
//     binding_number: nym_coconut_interface::Attribute,
//     epoch_id: u64,
//     signature: &nym_coconut_interface::Signature,
//     verification_key: &nym_coconut_interface::VerificationKey,
// ) -> Result<nym_coconut_interface::Credential, Error> {
//     let theta = nym_coconut_interface::prove_bandwidth_credential(
//         params,
//         verification_key,
//         signature,
//         serial_number,
//         binding_number,
//     )?;

//     Ok(nym_coconut_interface::Credential::new(
//         PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES,
//         theta,
//         voucher_value,
//         voucher_info,
//         epoch_id,
//     ))
// }
