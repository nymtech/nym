// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::bandwidth::CredentialSigningData;
use crate::coconut::utils::scalar_serde_helper;
use crate::error::Error;
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_credentials_interface::{
    hash_to_scalar, Attribute, BlindSignRequest, BlindedSignature, CredentialType, PublicAttribute,
};
use nym_crypto::asymmetric::{encryption, identity};
use nym_validator_client::nyxd::{Coin, Hash};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct BandwidthVoucherIssuedData {
    /// the plain value (e.g., bandwidth) encoded in this voucher
    // note: for legacy reasons we're only using the value of the coin and ignoring the denom
    #[zeroize(skip)]
    value: Coin,
}

impl<'a> From<&'a BandwidthVoucherIssuanceData> for BandwidthVoucherIssuedData {
    fn from(value: &'a BandwidthVoucherIssuanceData) -> Self {
        BandwidthVoucherIssuedData {
            value: value.value.clone(),
        }
    }
}

impl BandwidthVoucherIssuedData {
    pub fn value(&self) -> &Coin {
        &self.value
    }

    pub fn value_plain(&self) -> String {
        self.value.amount.to_string()
    }
}

#[derive(Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct BandwidthVoucherIssuanceData {
    /// the plain value (e.g., bandwidth) encoded in this voucher
    // note: for legacy reasons we're only using the value of the coin and ignoring the denom
    #[zeroize(skip)]
    value: Coin,

    // note: as mentioned above, we're only hashing the value of the coin!
    #[serde(with = "scalar_serde_helper")]
    value_prehashed: PublicAttribute,

    /// the hash of the deposit transaction
    #[zeroize(skip)]
    deposit_tx_hash: Hash,

    /// base58 encoded private key ensuring the depositer requested these attributes
    signing_key: identity::PrivateKey,

    /// base58 encoded private key ensuring only this client receives the signature share
    unused_ed25519: encryption::PrivateKey,
}

impl BandwidthVoucherIssuanceData {
    pub fn new(
        value: impl Into<Coin>,
        deposit_tx_hash: Hash,
        signing_key: identity::PrivateKey,
        unused_ed25519: encryption::PrivateKey,
    ) -> Self {
        let value = value.into();
        let value_prehashed = hash_to_scalar(value.amount.to_string());

        BandwidthVoucherIssuanceData {
            value,
            value_prehashed,
            deposit_tx_hash,
            signing_key,
            unused_ed25519,
        }
    }

    pub fn request_plaintext(request: &BlindSignRequest, tx_hash: Hash) -> Vec<u8> {
        let mut message = request.to_bytes();
        message.extend_from_slice(tx_hash.as_bytes());
        message
    }

    fn request_signature(&self, signing_request: &CredentialSigningData) -> identity::Signature {
        let message =
            Self::request_plaintext(&signing_request.blind_sign_request, self.deposit_tx_hash);
        self.signing_key.sign(message)
    }

    pub fn create_blind_sign_request_body(
        &self,
        signing_request: &CredentialSigningData,
    ) -> BlindSignRequestBody {
        let request_signature = self.request_signature(signing_request);

        BlindSignRequestBody::new(
            signing_request.blind_sign_request.clone(),
            self.deposit_tx_hash,
            request_signature,
            signing_request.public_attributes_plain.clone(),
        )
    }

    pub async fn obtain_blinded_credential(
        &self,
        client: &nym_validator_client::client::NymApiClient,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignature, Error> {
        let server_response = client.blind_sign(request_body).await?;
        Ok(server_response.blinded_signature)
    }

    pub fn value_plain(&self) -> String {
        self.value.amount.to_string()
    }

    pub fn value_attribute(&self) -> &Attribute {
        &self.value_prehashed
    }

    pub fn typ() -> CredentialType {
        CredentialType::Voucher
    }

    pub fn tx_hash(&self) -> Hash {
        self.deposit_tx_hash
    }

    pub fn identity_key(&self) -> &identity::PrivateKey {
        &self.signing_key
    }

    pub fn encryption_key(&self) -> &encryption::PrivateKey {
        &self.unused_ed25519
    }
}
