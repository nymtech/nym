// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::bandwidth::freepass::FreePassIssuanceData;
use crate::coconut::bandwidth::issued::IssuedBandwidthCredential;
use crate::coconut::bandwidth::voucher::BandwidthVoucherIssuanceData;
use crate::coconut::bandwidth::{
    bandwidth_credential_params, CredentialSigningData, CredentialType,
};
use crate::coconut::utils::{cred_exp_date_timestamp, freepass_exp_date_timestamp};
use crate::error::Error;
use log::error;
use nym_credentials_interface::{
    aggregate_wallets, constants, generate_keypair_user, issue_verify, setup, withdrawal_request,
    BlindedSignature, ExpirationDateSignature, KeyPairUser, Parameters, PartialWallet,
    VerificationKeyAuth, Wallet,
};
use nym_crypto::asymmetric::{encryption, identity};
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::{Coin, Hash};
use nym_validator_client::signing::AccountData;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub enum BandwidthCredentialIssuanceDataVariant {
    TicketBook(BandwidthVoucherIssuanceData),
    FreePass,
}

impl From<BandwidthVoucherIssuanceData> for BandwidthCredentialIssuanceDataVariant {
    fn from(value: BandwidthVoucherIssuanceData) -> Self {
        BandwidthCredentialIssuanceDataVariant::TicketBook(value)
    }
}

impl BandwidthCredentialIssuanceDataVariant {
    pub fn info(&self) -> CredentialType {
        match self {
            BandwidthCredentialIssuanceDataVariant::TicketBook(..) => CredentialType::TicketBook,
            BandwidthCredentialIssuanceDataVariant::FreePass => CredentialType::FreePass,
        }
    }

    pub fn voucher_data(&self) -> Option<&BandwidthVoucherIssuanceData> {
        match self {
            BandwidthCredentialIssuanceDataVariant::TicketBook(voucher) => Some(voucher),
            _ => None,
        }
    }
}

// all types of bandwidth credentials contain serial number and binding number
#[derive(Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct IssuanceBandwidthCredential {
    /// data specific to given bandwidth credential, for example a value for bandwidth voucher and expiry date for the free pass
    variant_data: BandwidthCredentialIssuanceDataVariant,

    ///ecash keypair related to the credential
    ecash_keypair: KeyPairUser,

    ///expiration_date of that credential
    expiration_date: u64,
}

impl IssuanceBandwidthCredential {
    pub fn default_parameters() -> Parameters {
        setup(constants::NB_TICKETS)
    }

    pub fn new<B: Into<BandwidthCredentialIssuanceDataVariant>>(
        variant_data: B,
        expiration_date: u64,
    ) -> Self {
        let variant_data = variant_data.into();
        let params = bandwidth_credential_params().grp();
        let ecash_keypair = generate_keypair_user(params);

        IssuanceBandwidthCredential {
            variant_data,
            ecash_keypair,
            expiration_date,
        }
    }

    pub fn new_voucher(
        value: impl Into<Coin>,
        deposit_tx_hash: Hash,
        signing_key: identity::PrivateKey,
        unused_ed25519: encryption::PrivateKey,
    ) -> Self {
        Self::new(
            BandwidthVoucherIssuanceData::new(value, deposit_tx_hash, signing_key, unused_ed25519),
            cred_exp_date_timestamp(),
        )
    }

    pub fn new_freepass(timestamp: u64) -> Self {
        let exp_timestamp = if timestamp > freepass_exp_date_timestamp() {
            error!(
                "the provided free pass request has too long expiry, setting it to max possible"
            );
            freepass_exp_date_timestamp()
        } else {
            timestamp
        };
        Self::new(
            BandwidthCredentialIssuanceDataVariant::FreePass,
            exp_timestamp,
        )
    }

    pub fn ecash_pubkey_bs58(&self) -> String {
        use nym_credentials_interface::Base58;

        self.ecash_keypair.public_key().to_bs58()
    }

    pub fn typ(&self) -> CredentialType {
        self.variant_data.info()
    }

    pub fn expiration_date(&self) -> u64 {
        self.expiration_date
    }

    pub fn get_variant_data(&self) -> &BandwidthCredentialIssuanceDataVariant {
        &self.variant_data
    }

    pub fn value(&self) -> u128 {
        if let BandwidthCredentialIssuanceDataVariant::TicketBook(data) = &self.variant_data {
            data.value()
        } else {
            0_u128
        }
    }

    pub fn check_expiration_date(&self) -> bool {
        let old_expiration_date = self.expiration_date;
        let new_expiration_date = match self.get_variant_data() {
            BandwidthCredentialIssuanceDataVariant::TicketBook(_) => cred_exp_date_timestamp(),
            BandwidthCredentialIssuanceDataVariant::FreePass => freepass_exp_date_timestamp(),
        };
        old_expiration_date != new_expiration_date
    }

    pub fn prepare_for_signing(&self) -> CredentialSigningData {
        let params = bandwidth_credential_params();

        // safety: the creation of the request can only fail if one provided invalid parameters
        // and we created then specific to this type of the credential so the unwrap is fine
        let (withdrawal_request, request_info) = withdrawal_request(
            params.grp(),
            &self.ecash_keypair.secret_key(),
            self.expiration_date,
        )
        .unwrap();

        CredentialSigningData {
            withdrawal_request,
            request_info,
            ecash_pub_key: self.ecash_keypair.public_key(),
            typ: self.typ(),
            expiration_date: self.expiration_date,
        }
    }

    pub fn unblind_signature(
        &self,
        validator_vk: &VerificationKeyAuth,
        signing_data: &CredentialSigningData,
        blinded_signature: BlindedSignature,
        signer_index: u64,
    ) -> Result<PartialWallet, Error> {
        let params = bandwidth_credential_params().grp();
        let unblinded_signature = issue_verify(
            params,
            validator_vk,
            &self.ecash_keypair.secret_key(),
            &blinded_signature,
            &signing_data.request_info,
            signer_index,
        )?;

        Ok(unblinded_signature)
    }

    pub async fn obtain_partial_freepass_credential(
        &self,
        client: &nym_validator_client::client::NymApiClient,
        signer_index: u64,
        account_data: &AccountData,
        validator_vk: &VerificationKeyAuth,
        signing_data: CredentialSigningData,
    ) -> Result<PartialWallet, Error> {
        // We need signing data, because they will be use at the aggregation step

        let blinded_signature = match &self.variant_data {
            BandwidthCredentialIssuanceDataVariant::FreePass => {
                FreePassIssuanceData::request_blinded_credential(
                    &signing_data,
                    account_data,
                    client,
                )
                .await?
            }
            _ => return Err(Error::NotAFreePass),
        };
        self.unblind_signature(validator_vk, &signing_data, blinded_signature, signer_index)
    }

    // ideally this would have been generic over credential type, but we really don't need secp256k1 keys for bandwidth vouchers
    pub async fn obtain_partial_bandwidth_voucher_credential(
        &self,
        client: &nym_validator_client::client::NymApiClient,
        signer_index: u64,
        validator_vk: &VerificationKeyAuth,
        signing_data: CredentialSigningData,
    ) -> Result<PartialWallet, Error> {
        // We need signing data, because they will be use at the aggregation step

        let blinded_signature = match &self.variant_data {
            BandwidthCredentialIssuanceDataVariant::TicketBook(voucher) => {
                // TODO: the request can be re-used between different apis
                let request = voucher.create_blind_sign_request_body(&signing_data);
                voucher.obtain_blinded_credential(client, &request).await?
            }
            _ => return Err(Error::NotABandwdithVoucher),
        };
        self.unblind_signature(validator_vk, &signing_data, blinded_signature, signer_index)
    }

    pub fn aggregate_signature_shares(
        &self,
        verification_key: &VerificationKeyAuth,
        shares: &[PartialWallet],
        signing_data: CredentialSigningData,
    ) -> Result<Wallet, Error> {
        let params = bandwidth_credential_params().grp();
        aggregate_wallets(
            params,
            verification_key,
            &self.ecash_keypair.secret_key(),
            shares,
            &signing_data.request_info,
        )
        .map_err(Error::SignatureAggregationError)
    }

    // also drops self after the conversion
    pub fn into_issued_credential(
        self,
        wallet: Wallet,
        exp_date_signatures: Vec<ExpirationDateSignature>,
        epoch_id: EpochId,
    ) -> IssuedBandwidthCredential {
        self.to_issued_credential(wallet, exp_date_signatures, epoch_id)
    }

    pub fn to_issued_credential(
        &self,
        wallet: Wallet,
        exp_date_signatures: Vec<ExpirationDateSignature>,
        epoch_id: EpochId,
    ) -> IssuedBandwidthCredential {
        IssuedBandwidthCredential::new(
            wallet,
            (&self.variant_data).into(),
            epoch_id,
            self.ecash_keypair.secret_key(),
            exp_date_signatures,
            self.expiration_date,
        )
    }

    // TODO: is that actually needed?
    pub fn to_recovery_bytes(&self) -> Vec<u8> {
        use bincode::Options;
        // safety: our data format is stable and thus the serialization should not fail
        make_recovery_bincode_serializer().serialize(self).unwrap()
    }

    // TODO: is that actually needed?
    // idea: make it consistent with the issued credential and its vX serde
    pub fn try_from_recovered_bytes(bytes: &[u8]) -> Result<Self, Error> {
        use bincode::Options;
        make_recovery_bincode_serializer()
            .deserialize(bytes)
            .map_err(|source| Error::RecoveryCredentialDeserializationFailure { source })
    }
}

fn make_recovery_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    fn assert_zeroize<T: Zeroize>() {}

    #[test]
    fn credential_is_zeroized() {
        assert_zeroize::<IssuanceBandwidthCredential>();
        assert_zeroize_on_drop::<IssuanceBandwidthCredential>();
    }
}
