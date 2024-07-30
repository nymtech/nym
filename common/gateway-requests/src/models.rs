// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::ecash::bandwidth::CredentialSpendingData;
use nym_credentials_interface::{Base58, Bytable, CompactEcashError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CredentialSpendingRequest {
    /// The cryptographic material required for spending the underlying credential.
    pub data: CredentialSpendingData,
}

impl CredentialSpendingRequest {
    pub fn new(data: CredentialSpendingData) -> Self {
        CredentialSpendingRequest { data }
    }

    // pub fn matches_serial_number(
    //     &self,
    //     serial_number_bs58: &str,
    // ) -> Result<bool, CompactEcashError> {
    //     self.data.payment.has_serial_number(serial_number_bs58)
    // }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.to_bytes()
    }

    pub fn try_from_bytes(raw: &[u8]) -> Result<Self, CompactEcashError> {
        Ok(CredentialSpendingRequest {
            data: CredentialSpendingData::try_from_bytes(raw)?,
        })
    }

    pub fn encoded_serial_number(&self) -> Vec<u8> {
        self.data.encoded_serial_number()
    }

    pub fn serial_number_bs58(&self) -> String {
        self.data.serial_number_b58()
    }
}

impl Bytable for CredentialSpendingRequest {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CompactEcashError> {
        Self::try_from_bytes(slice)
    }
}

impl Base58 for CredentialSpendingRequest {}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_compact_ecash::{
        issue,
        tests::helpers::{generate_coin_indices_signatures, generate_expiration_date_signatures},
        ttp_keygen, PayInfo,
    };
    use nym_credentials::ecash::utils::EcashTime;
    use nym_credentials::IssuanceTicketBook;
    use nym_credentials_interface::TicketType;
    use nym_crypto::asymmetric::ed25519;
    use rand::rngs::OsRng;

    #[test]
    fn credential_roundtrip() {
        // make valid request
        let keypair = ttp_keygen(1, 1).unwrap().remove(0);

        let mut rng = OsRng;
        let signing_key = ed25519::PrivateKey::new(&mut rng);

        let issuance = IssuanceTicketBook::new(42, [], signing_key, TicketType::V1MixnetEntry);
        let expiration_date = issuance.expiration_date();
        let sig_req = issuance.prepare_for_signing();
        let exp_date_sigs = generate_expiration_date_signatures(
            sig_req.expiration_date.ecash_unix_timestamp(),
            &[keypair.secret_key()],
            &vec![keypair.verification_key()],
            &keypair.verification_key(),
            &[keypair.index.unwrap()],
        )
        .unwrap();
        let blind_sig = issue(
            keypair.secret_key(),
            sig_req.ecash_pub_key.clone(),
            &sig_req.withdrawal_request,
            expiration_date.ecash_unix_timestamp(),
            issuance.ticketbook_type().encode(),
        )
        .unwrap();

        let partial_wallet = issuance
            .unblind_signature(
                &keypair.verification_key(),
                &sig_req,
                blind_sig,
                keypair.index.unwrap(),
            )
            .unwrap();

        let wallet = issuance
            .aggregate_signature_shares(&keypair.verification_key(), &vec![partial_wallet], sig_req)
            .unwrap();

        let mut issued = issuance.into_issued_ticketbook(wallet, 1);
        let coin_indices_signatures = generate_coin_indices_signatures(
            nym_credentials_interface::ecash_parameters(),
            &[keypair.secret_key()],
            &vec![keypair.verification_key()],
            &keypair.verification_key(),
            &[keypair.index.unwrap()],
        )
        .unwrap();
        let pay_info = PayInfo {
            pay_info_bytes: [6u8; 72],
        };
        let spending = issued
            .prepare_for_spending(
                &keypair.verification_key(),
                pay_info,
                &coin_indices_signatures,
                &exp_date_sigs,
                1,
            )
            .unwrap();

        let with_epoch = CredentialSpendingRequest { data: spending };

        let bytes = with_epoch.to_bytes();
        let recovered = CredentialSpendingRequest::try_from_bytes(&bytes).unwrap();

        assert_eq!(with_epoch, recovered);
    }
}
