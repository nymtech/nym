// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use nym_api_requests::coconut::FreePassRequest;
use nym_credentials_interface::{BlindedSignature, CredentialSigningData};
use nym_validator_client::signing::AccountData;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct FreePassIssuedData {}

pub struct FreePassIssuanceData {}

impl FreePassIssuanceData {
    pub async fn obtain_free_pass_nonce(
        client: &nym_validator_client::client::NymApiClient,
    ) -> Result<[u8; 16], Error> {
        let server_response = client.free_pass_nonce().await?;
        Ok(server_response.current_nonce)
    }

    pub fn create_free_pass_request(
        signing_request: &CredentialSigningData,
        account_data: &AccountData,
        issuer_nonce: [u8; 16],
    ) -> Result<FreePassRequest, Error> {
        let nonce_signature = account_data
            .private_key()
            .sign(&issuer_nonce)
            .map_err(|_| Error::Secp256k1SignFailure)?;

        Ok(FreePassRequest {
            cosmos_pubkey: account_data.public_key(),
            inner_sign_request: signing_request.withdrawal_request.clone(),
            used_nonce: issuer_nonce,
            nonce_signature,
            ecash_pubkey: signing_request.ecash_pub_key.clone(),
            expiration_date: signing_request.expiration_date,
        })
    }

    pub async fn obtain_blinded_credential(
        client: &nym_validator_client::client::NymApiClient,
        request: &FreePassRequest,
    ) -> Result<BlindedSignature, Error> {
        let server_response = client.issue_free_pass_credential(request).await?;
        Ok(server_response.blinded_signature)
    }

    pub async fn request_blinded_credential(
        signing_request: &CredentialSigningData,
        account_data: &AccountData,
        client: &nym_validator_client::client::NymApiClient,
    ) -> Result<BlindedSignature, Error> {
        let signing_nonce = Self::obtain_free_pass_nonce(client).await?;
        let request = Self::create_free_pass_request(signing_request, account_data, signing_nonce)?;
        Self::obtain_blinded_credential(client, &request).await
    }
}
