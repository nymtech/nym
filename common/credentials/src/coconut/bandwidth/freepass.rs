// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::utils::scalar_serde_helper;
use crate::error::Error;
use nym_api_requests::coconut::FreePassRequest;
use nym_credentials_interface::{
    hash_to_scalar, Attribute, BlindedSignature, CredentialSigningData, PublicAttribute,
};
use nym_validator_client::signing::AccountData;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, Time};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const MAX_FREE_PASS_VALIDITY: Duration = Duration::WEEK; // 1 week

#[derive(Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct FreePassIssuedData {
    /// the plain validity value of this credential expressed as unix timestamp
    #[zeroize(skip)]
    expiry_date: OffsetDateTime,
}

impl<'a> From<&'a FreePassIssuanceData> for FreePassIssuedData {
    fn from(value: &'a FreePassIssuanceData) -> Self {
        FreePassIssuedData {
            expiry_date: value.expiry_date,
        }
    }
}

impl FreePassIssuedData {
    pub fn expiry_date_plain(&self) -> String {
        self.expiry_date.unix_timestamp().to_string()
    }
}

#[derive(Zeroize, Serialize, Deserialize)]
pub struct FreePassIssuanceData {
    /// the plain validity value of this credential expressed as unix timestamp
    #[zeroize(skip)]
    expiry_date: OffsetDateTime,

    // the expiry date, as unix timestamp, hashed into a scalar
    #[serde(with = "scalar_serde_helper")]
    expiry_date_prehashed: PublicAttribute,
}

impl FreePassIssuanceData {
    pub fn new(expiry_date: Option<OffsetDateTime>) -> Self {
        // ideally we should have implemented a proper error handling here, sure.
        // but given it's meant to only be used by nym, imo it's fine to just panic here in case of invalid arguments
        let expiry_date = if let Some(provided) = expiry_date {
            if provided - OffsetDateTime::now_utc() > MAX_FREE_PASS_VALIDITY {
                panic!("the provided expiry date is bigger than the maximum value of {MAX_FREE_PASS_VALIDITY}");
            }

            provided
        } else {
            Self::default_expiry_date()
        };

        let expiry_date_prehashed = hash_to_scalar(expiry_date.unix_timestamp().to_string());

        FreePassIssuanceData {
            expiry_date,
            expiry_date_prehashed,
        }
    }

    pub fn default_expiry_date() -> OffsetDateTime {
        // set it to furthest midnight in the future such as it's no more than a week away,
        // i.e. if it's currently for example 9:43 on 2nd March 2024, it will set it to 0:00 on 9th March 2024
        (OffsetDateTime::now_utc() + MAX_FREE_PASS_VALIDITY).replace_time(Time::MIDNIGHT)
    }

    pub fn expiry_date_attribute(&self) -> &Attribute {
        &self.expiry_date_prehashed
    }

    pub fn expiry_date_plain(&self) -> String {
        self.expiry_date.unix_timestamp().to_string()
    }

    pub async fn obtain_free_pass_nonce(
        &self,
        client: &nym_validator_client::client::NymApiClient,
    ) -> Result<u32, Error> {
        let server_response = client.free_pass_nonce().await?;
        Ok(server_response.current_nonce)
    }

    pub fn create_free_pass_request(
        &self,
        signing_request: &CredentialSigningData,
        account_data: &AccountData,
        issuer_nonce: u32,
    ) -> Result<FreePassRequest, Error> {
        let plaintext = issuer_nonce.to_be_bytes();
        let nonce_signature = account_data
            .private_key()
            .sign(&plaintext)
            .map_err(|_| Error::Secp256k1SignFailure)?;

        Ok(FreePassRequest {
            cosmos_pubkey: account_data.public_key(),
            inner_sign_request: signing_request.blind_sign_request.clone(),
            used_nonce: issuer_nonce,
            nonce_signature,
            public_attributes_plain: signing_request.public_attributes_plain.clone(),
        })
    }

    pub async fn obtain_blinded_credential(
        &self,
        client: &nym_validator_client::client::NymApiClient,
        request: &FreePassRequest,
    ) -> Result<BlindedSignature, Error> {
        let server_response = client.issue_free_pass_credential(request).await?;
        Ok(server_response.blinded_signature)
    }

    pub async fn request_blinded_credential(
        &self,
        signing_request: &CredentialSigningData,
        account_data: &AccountData,
        client: &nym_validator_client::client::NymApiClient,
    ) -> Result<BlindedSignature, Error> {
        let signing_nonce = self.obtain_free_pass_nonce(client).await?;
        let request =
            self.create_free_pass_request(signing_request, account_data, signing_nonce)?;
        self.obtain_blinded_credential(client, &request).await
    }
}
