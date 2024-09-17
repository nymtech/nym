// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::bandwidth::issued::IssuedTicketBook;
use crate::ecash::bandwidth::serialiser::VersionedSerialise;
use crate::ecash::bandwidth::CredentialSigningData;
use crate::ecash::utils::cred_exp_date;
use crate::error::Error;
use nym_api_requests::ecash::BlindSignRequestBody;
use nym_credentials_interface::{
    aggregate_wallets, generate_keypair_user_from_seed, issue_verify, withdrawal_request,
    BlindedSignature, KeyPairUser, PartialWallet, TicketType, VerificationKeyAuth,
    WalletSignatures, WithdrawalRequest,
};
use nym_crypto::asymmetric::identity;
use nym_ecash_contract_common::deposit::DepositId;
use nym_ecash_time::{ecash_default_expiration_date, ecash_today, EcashTime};
use nym_validator_client::nym_api::EpochId;
use serde::{Deserialize, Serialize};
use time::Date;

pub use nym_validator_client::nyxd::{Coin, Hash};

#[derive(Serialize, Deserialize)]
pub struct IssuanceTicketBook {
    /// the id of the associated deposit
    deposit_id: DepositId,

    /// base58 encoded private key ensuring the depositer requested these attributes
    signing_key: identity::PrivateKey,

    /// ecash keypair related to the credential
    ecash_keypair: KeyPairUser,

    /// the type of the ticketbook to be issued
    ticketbook_type: TicketType,

    /// expiration_date of that credential
    expiration_date: Date,
}

impl IssuanceTicketBook {
    pub fn new<M: AsRef<[u8]>>(
        deposit_id: DepositId,
        identifier: M,
        signing_key: identity::PrivateKey,
        ticketbook_type: TicketType,
    ) -> Self {
        //this expiration date will get fed to the ecash library, force midnight to be set
        Self::new_with_expiration(
            deposit_id,
            identifier,
            signing_key,
            ticketbook_type,
            ecash_default_expiration_date(),
        )
    }

    pub fn new_with_expiration<M: AsRef<[u8]>>(
        deposit_id: DepositId,
        identifier: M,
        signing_key: identity::PrivateKey,
        ticketbook_type: TicketType,
        expiration_date: Date,
    ) -> Self {
        let ecash_keypair = generate_keypair_user_from_seed(identifier);
        IssuanceTicketBook {
            deposit_id,
            signing_key,
            ecash_keypair,
            ticketbook_type,
            expiration_date,
        }
    }

    pub fn ecash_pubkey_bs58(&self) -> String {
        use nym_credentials_interface::Base58;

        self.ecash_keypair.public_key().to_bs58()
    }

    pub fn expiration_date(&self) -> Date {
        self.expiration_date
    }

    pub fn ticketbook_type(&self) -> TicketType {
        self.ticketbook_type
    }

    pub fn request_plaintext(request: &WithdrawalRequest, deposit_id: DepositId) -> Vec<u8> {
        let mut message = request.to_bytes();
        message.extend_from_slice(&deposit_id.to_be_bytes());
        message
    }

    fn request_signature(&self, signing_request: &CredentialSigningData) -> identity::Signature {
        let message = Self::request_plaintext(&signing_request.withdrawal_request, self.deposit_id);
        self.signing_key.sign(message)
    }

    pub fn create_blind_sign_request_body(
        &self,
        signing_request: &CredentialSigningData,
    ) -> BlindSignRequestBody {
        let request_signature = self.request_signature(signing_request);

        BlindSignRequestBody::new(
            signing_request.withdrawal_request.clone(),
            self.deposit_id,
            request_signature,
            signing_request.ecash_pub_key.clone(),
            signing_request.expiration_date,
            signing_request.ticketbook_type,
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

    pub fn deposit_id(&self) -> DepositId {
        self.deposit_id
    }

    pub fn identity_key(&self) -> &identity::PrivateKey {
        &self.signing_key
    }

    pub fn check_expiration_date(&self) -> bool {
        self.expiration_date != cred_exp_date().ecash_date()
    }

    pub fn expired(&self) -> bool {
        self.expiration_date < ecash_today().date()
    }

    pub fn prepare_for_signing(&self) -> CredentialSigningData {
        // safety: the creation of the request can only fail if one provided invalid parameters
        // and we created then specific to this type of the credential so the unwrap is fine
        let (withdrawal_request, request_info) = withdrawal_request(
            self.ecash_keypair.secret_key(),
            self.expiration_date.ecash_unix_timestamp(),
            self.ticketbook_type.encode(),
        )
        .unwrap();

        CredentialSigningData {
            withdrawal_request,
            request_info,
            ecash_pub_key: self.ecash_keypair.public_key(),
            expiration_date: self.expiration_date,
            ticketbook_type: self.ticketbook_type,
        }
    }

    pub fn unblind_signature(
        &self,
        validator_vk: &VerificationKeyAuth,
        signing_data: &CredentialSigningData,
        blinded_signature: BlindedSignature,
        signer_index: u64,
    ) -> Result<PartialWallet, Error> {
        let unblinded_signature = issue_verify(
            validator_vk,
            self.ecash_keypair.secret_key(),
            &blinded_signature,
            &signing_data.request_info,
            signer_index,
        )?;

        Ok(unblinded_signature)
    }

    // ideally this would have been generic over credential type, but we really don't need secp256k1 keys for bandwidth vouchers
    pub async fn obtain_partial_ticketbook_credential(
        &self,
        client: &nym_validator_client::client::NymApiClient,
        signer_index: u64,
        validator_vk: &VerificationKeyAuth,
        signing_data: CredentialSigningData,
    ) -> Result<PartialWallet, Error> {
        // We need signing data, because they will be used at the aggregation step

        let request = self.create_blind_sign_request_body(&signing_data);
        let blinded_signature = self.obtain_blinded_credential(client, &request).await?;
        self.unblind_signature(validator_vk, &signing_data, blinded_signature, signer_index)
    }

    pub fn aggregate_signature_shares(
        &self,
        verification_key: &VerificationKeyAuth,
        shares: &[PartialWallet],
        signing_data: CredentialSigningData,
    ) -> Result<WalletSignatures, Error> {
        aggregate_wallets(
            verification_key,
            self.ecash_keypair.secret_key(),
            shares,
            &signing_data.request_info,
        )
        .map_err(Error::SignatureAggregationError)
        .map(|w| w.into_wallet_signatures())
    }

    // also drops self after the conversion
    pub fn into_issued_ticketbook(
        self,
        wallet: WalletSignatures,
        epoch_id: EpochId,
    ) -> IssuedTicketBook {
        self.to_issued_ticketbook(wallet, epoch_id)
    }

    pub fn to_issued_ticketbook(
        &self,
        wallet: WalletSignatures,
        epoch_id: EpochId,
    ) -> IssuedTicketBook {
        IssuedTicketBook::new(
            wallet,
            epoch_id,
            self.ecash_keypair.secret_key().clone(),
            self.ticketbook_type,
            self.expiration_date,
        )
    }
}

impl VersionedSerialise for IssuanceTicketBook {
    const CURRENT_SERIALISATION_REVISION: u8 = 1;

    fn try_unpack(b: &[u8], revision: impl Into<Option<u8>>) -> Result<Self, Error> {
        let revision = revision
            .into()
            .unwrap_or(<Self as VersionedSerialise>::CURRENT_SERIALISATION_REVISION);

        match revision {
            1 => Self::try_unpack_current(b),
            _ => Err(Error::UnknownSerializationRevision { revision }),
        }
    }
}
