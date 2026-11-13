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

    use nym_credentials::ecash::bandwidth::CredentialSpendingData;
    use time::{Date, Duration, Month, OffsetDateTime, Time, UtcOffset};

    // A fixed day keeps the fixture off the wall clock: the ticketbook is
    // valid for the ticketbook validity period starting on it.
    fn fixture_day() -> Date {
        Date::from_calendar_date(2030, Month::January, 15).unwrap()
    }

    fn on_fixture_day(hour: u8) -> OffsetDateTime {
        fixture_day()
            .with_time(Time::from_hms(hour, 0, 0).unwrap())
            .assume_utc()
    }

    #[test]
    fn credential_roundtrip() {
        let with_epoch = CredentialSpendingRequest {
            data: spend_one_ticket(on_fixture_day(12)),
        };

        let bytes = with_epoch.to_bytes();
        let recovered = CredentialSpendingRequest::try_from_bytes(&bytes).unwrap();

        assert_eq!(with_epoch, recovered);
    }

    // The spend date must follow the (skew-corrected) spend time the caller
    // passes, not the local clock: gateways compare it with their own date.
    #[test]
    fn spend_date_follows_spend_time() {
        let next_day = on_fixture_day(12) + Duration::days(1);

        assert_eq!(spend_one_ticket(next_day).spend_date, next_day.date());
    }

    // A spend time given with a local offset still yields its UTC date.
    #[test]
    fn spend_date_is_the_utc_date_of_spend_time() {
        let utc = on_fixture_day(20);
        let local = utc.to_offset(UtcOffset::from_hms(14, 0, 0).unwrap());
        assert_ne!(local.date(), utc.date());

        assert_eq!(spend_one_ticket(local).spend_date, utc.date());
    }

    fn spend_one_ticket(spend_time: OffsetDateTime) -> CredentialSpendingData {
        let keypair = ttp_keygen(1, 1).unwrap().remove(0);

        let mut rng = rand::rng();
        let signing_key = ed25519::PrivateKey::new(&mut rng);

        let validity = nym_compact_ecash::constants::CRED_VALIDITY_PERIOD_DAYS as i64;
        let issuance = IssuanceTicketBook::new_with_expiration(
            42,
            [],
            signing_key,
            TicketType::V1MixnetEntry,
            fixture_day() + Duration::days(validity - 1),
        );
        let expiration_date = issuance.expiration_date();
        let sig_req = issuance.prepare_for_signing();
        let exp_date_sigs = generate_expiration_date_signatures(
            sig_req.expiration_date.ecash_unix_timestamp(),
            &[keypair.secret_key()],
            &[keypair.verification_key()],
            &keypair.verification_key(),
            &[keypair.index.unwrap()],
        )
        .unwrap();
        let blind_sig = issue(
            keypair.secret_key(),
            sig_req.ecash_pub_key,
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
            .aggregate_signature_shares(&keypair.verification_key(), &[partial_wallet], sig_req)
            .unwrap();

        let mut issued = issuance.into_issued_ticketbook(wallet, 1);
        let coin_indices_signatures = generate_coin_indices_signatures(
            nym_credentials_interface::ecash_parameters(),
            &[keypair.secret_key()],
            &[keypair.verification_key()],
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
                spend_time,
                &coin_indices_signatures,
                &exp_date_sigs,
                1,
            )
            .unwrap();
        // The proof must verify against the spend date the ticket carries.
        spending.verify(&keypair.verification_key()).unwrap();
        spending
    }
}
