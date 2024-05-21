// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::error::{CoconutError, Result};
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_credentials::coconut::bandwidth::voucher::BandwidthVoucherIssuanceData;
use nym_credentials::coconut::bandwidth::CredentialType;
use nym_crypto::asymmetric::identity;
use nym_ecash_contract_common::events::{
    COSMWASM_DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ENCRYPTION_KEY, DEPOSIT_IDENTITY_KEY,
    DEPOSIT_INFO, DEPOSIT_VALUE,
};
use nym_validator_client::nyxd::helpers::find_tx_attribute;
use nym_validator_client::nyxd::TxResponse;

pub async fn validate_deposit_tx(request: &BlindSignRequestBody, tx: TxResponse) -> Result<()> {
    let deposit_info = find_tx_attribute(&tx, COSMWASM_DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_INFO)
        .ok_or(CoconutError::DepositInfoNotFound)?;

    let x25519_raw = find_tx_attribute(
        &tx,
        COSMWASM_DEPOSITED_FUNDS_EVENT_TYPE,
        DEPOSIT_IDENTITY_KEY,
    )
    .ok_or(CoconutError::DepositVerifKeyNotFound)?;

    // check public attributes against static data
    // (thinking about it attaching that data might be redundant since we have the source of truth on the chain)
    if deposit_info != CredentialType::TicketBook.to_string() {
        return Err(CoconutError::InconsistentDepositInfo {
            request: CredentialType::TicketBook.to_string(),
            on_chain: deposit_info,
        });
    }
    // verify signature
    let x25519 = identity::PublicKey::from_base58_string(x25519_raw)?;
    let plaintext = BandwidthVoucherIssuanceData::request_plaintext(
        &request.inner_sign_request,
        request.tx_hash,
    );
    x25519.verify(plaintext, &request.signature)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::coconut::tests::{tx_entry_fixture, voucher_fixture};
    use nym_compact_ecash::{
        generate_keypair_user, scheme::withdrawal::WithdrawalRequest, setup::GroupParameters,
    };
    use nym_credentials::coconut::bandwidth::CredentialType;
    use nym_ecash_contract_common::events::DEPOSITED_FUNDS_EVENT_TYPE;
    use nym_validator_client::nyxd::{Event, EventAttribute};

    #[tokio::test]
    async fn validate_deposit_tx_test() {
        let voucher = voucher_fixture(None);
        let signing_data = voucher.prepare_for_signing();
        let voucher_data = voucher.get_variant_data().voucher_data().unwrap();
        let correct_request = voucher_data.create_blind_sign_request_body(&signing_data);

        let mut tx_entry = tx_entry_fixture(correct_request.tx_hash);
        let good_deposit_attribute = EventAttribute {
            key: DEPOSIT_VALUE.to_string(),
            value: "random string, it only needs to be there".to_string(),
            index: false,
        };
        let good_info_attribute = EventAttribute {
            key: DEPOSIT_INFO.to_string(),
            value: CredentialType::TicketBook.to_string(),
            index: false,
        };
        let good_identity_key_attribute = EventAttribute {
            key: DEPOSIT_IDENTITY_KEY.to_string(),
            value: "2eSxwquNJb2nZTEW5p4rbqjHfBaz9UaNhjHHiexPN4He".to_string(),
            index: false,
        };
        let good_encryption_key_attribute = EventAttribute {
            key: DEPOSIT_ENCRYPTION_KEY.to_string(),
            value: "2eSxwquNJb2nZTEW5p4rbqjHfBaz9UaNhjHHiexPN4He".to_string(),
            index: false,
        };

        tx_entry.tx_result.events.push(Event {
            kind: format!("wasm-{}", DEPOSITED_FUNDS_EVENT_TYPE),
            attributes: vec![],
        });
        let err = validate_deposit_tx(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DepositValueNotFound.to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            good_info_attribute.clone(),
            good_identity_key_attribute.clone(),
            good_encryption_key_attribute.clone(),
        ];
        let err = validate_deposit_tx(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DepositValueNotFound.to_string()
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            good_deposit_attribute.clone(),
            good_identity_key_attribute.clone(),
            good_encryption_key_attribute.clone(),
        ];
        let err = validate_deposit_tx(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DepositInfoNotFound.to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            good_deposit_attribute.clone(),
            EventAttribute {
                key: DEPOSIT_INFO.parse().unwrap(),
                value: "bandwidth deposit info".parse().unwrap(),
                index: false,
            },
            good_identity_key_attribute.clone(),
            good_encryption_key_attribute.clone(),
        ];
        let err = validate_deposit_tx(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::InconsistentDepositInfo {
                on_chain: "bandwidth deposit info".to_string(),
                request: CredentialType::TicketBook.to_string(),
            }
            .to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            good_deposit_attribute.clone(),
            good_info_attribute.clone(),
            good_encryption_key_attribute.clone(),
        ];
        let err = validate_deposit_tx(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DepositVerifKeyNotFound.to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            good_deposit_attribute.clone(),
            good_info_attribute.clone(),
            EventAttribute {
                key: DEPOSIT_IDENTITY_KEY.parse().unwrap(),
                value: "verification key".parse().unwrap(),
                index: false,
            },
            good_encryption_key_attribute.clone(),
        ];
        let err = validate_deposit_tx(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            CoconutError::Ed25519ParseError(
                nym_crypto::asymmetric::identity::Ed25519RecoveryError::MalformedPublicKeyString { .. }
            )
        ));

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            good_deposit_attribute.clone(),
            good_info_attribute.clone(),
            EventAttribute {
                key: DEPOSIT_IDENTITY_KEY.parse().unwrap(),
                value: "2eSxwquNJb2nZTEW5p4rbqjHfBaz9UaNhjHHiexPN4He"
                    .parse()
                    .unwrap(),
                index: false,
            },
        ];
        let err = validate_deposit_tx(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DepositEncrKeyNotFound.to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            good_deposit_attribute.clone(),
            good_info_attribute.clone(),
            EventAttribute {
                key: DEPOSIT_IDENTITY_KEY.parse().unwrap(),
                value: "6EJGMdEq7t8Npz54uPkftGsdmj7DKntLVputAnDfVZB2"
                    .parse()
                    .unwrap(),
                index: false,
            },
            EventAttribute {
                key: DEPOSIT_ENCRYPTION_KEY.parse().unwrap(),
                value: "encryption key".parse().unwrap(),
                index: false,
            },
        ];
        let err = validate_deposit_tx(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();

        assert!(matches!(err, CoconutError::SignatureVerificationError(..)));

        let expected_encryption_key = "HxnTpWTkgigSTAysVKLE8pEiUULHdTT1BxFfzfJvQRi6";
        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            good_deposit_attribute.clone(),
            good_info_attribute.clone(),
            EventAttribute {
                key: DEPOSIT_IDENTITY_KEY.parse().unwrap(),
                value: "6EJGMdEq7t8Npz54uPkftGsdmj7DKntLVputAnDfVZB2"
                    .parse()
                    .unwrap(),
                index: false,
            },
            EventAttribute {
                key: DEPOSIT_ENCRYPTION_KEY.parse().unwrap(),
                value: expected_encryption_key.parse().unwrap(),
                index: false,
            },
        ];
        let err = validate_deposit_tx(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::SignatureVerificationError(
                nym_crypto::asymmetric::identity::SignatureError::default(),
            )
            .to_string(),
        );

        //hard-coded values, that generate a correct signature
        let blind_sign_req = WithdrawalRequest::try_from(
            [
                176u8, 113, 19, 237, 218, 252, 113, 20, 225, 238, 59, 88, 217, 45, 233, 178, 65,
                28, 242, 0, 222, 48, 110, 216, 26, 111, 51, 235, 61, 74, 200, 15, 130, 245, 45,
                170, 155, 190, 156, 77, 180, 142, 29, 63, 15, 224, 150, 31, 139, 24, 65, 175, 143,
                153, 11, 203, 33, 16, 152, 22, 221, 203, 99, 233, 208, 142, 161, 194, 46, 227, 177,
                96, 119, 30, 175, 69, 104, 14, 2, 191, 26, 94, 30, 165, 15, 28, 40, 176, 1, 78,
                253, 79, 20, 137, 102, 74, 2, 0, 0, 0, 0, 0, 0, 0, 131, 133, 112, 115, 53, 98, 58,
                166, 240, 70, 185, 210, 203, 12, 114, 66, 180, 38, 139, 12, 187, 45, 250, 201, 68,
                102, 159, 172, 218, 124, 151, 23, 172, 18, 216, 122, 246, 7, 185, 76, 20, 167, 123,
                122, 152, 241, 175, 226, 176, 8, 170, 70, 140, 252, 36, 130, 67, 204, 111, 116,
                107, 92, 200, 77, 252, 31, 138, 18, 10, 215, 165, 243, 95, 199, 193, 61, 200, 187,
                22, 198, 109, 213, 145, 71, 171, 132, 174, 68, 105, 248, 0, 115, 50, 55, 199, 84,
                67, 16, 125, 216, 250, 154, 115, 174, 9, 206, 44, 88, 63, 163, 124, 10, 239, 64,
                158, 191, 27, 169, 177, 194, 223, 142, 202, 206, 189, 122, 123, 91, 171, 15, 40,
                192, 148, 75, 174, 24, 116, 229, 127, 170, 110, 183, 151, 2, 118, 168, 22, 113, 87,
                237, 91, 228, 249, 120, 114, 255, 53, 175, 245, 39, 2, 0, 0, 0, 0, 0, 0, 0, 225,
                45, 230, 25, 62, 202, 96, 166, 171, 241, 206, 137, 254, 51, 154, 255, 122, 130,
                107, 54, 5, 206, 207, 120, 193, 214, 64, 10, 111, 195, 86, 55, 201, 36, 10, 18,
                154, 158, 183, 87, 185, 59, 228, 89, 134, 193, 217, 188, 64, 164, 249, 21, 248, 20,
                207, 58, 31, 10, 19, 176, 246, 150, 45, 48, 2, 0, 0, 0, 0, 0, 0, 0, 173, 60, 65,
                209, 100, 114, 138, 186, 158, 150, 109, 230, 111, 86, 101, 72, 194, 237, 173, 195,
                139, 175, 238, 25, 169, 18, 188, 75, 77, 54, 111, 20, 115, 235, 195, 2, 123, 133,
                164, 81, 15, 45, 11, 84, 139, 38, 8, 224, 197, 181, 95, 147, 49, 77, 193, 207, 52,
                141, 195, 195, 66, 137, 17, 32,
            ]
            .as_slice(),
        )
        .unwrap();
        let expiration_date = 1708300800; // Feb, 19th, 2024
        let ecash_keypair = generate_keypair_user(&GroupParameters::new());

        let correct_request = BlindSignRequestBody::new(
            blind_sign_req,
            "7C41AF8266D91DE55E1C8F4712E6A952A165ED3D8C27C7B00428CBD0DE00A52B"
                .parse()
                .unwrap(),
            "3vUCc6MCN5AC2LNgDYjRB1QeErZSN1S8f6K14JHjpUcKWXbjGYFExA8DbwQQBki9gyUqrpBF94Drttb4eMcGQXkp".parse().unwrap(),
            ecash_keypair.public_key(),
            expiration_date,
        );
        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            good_deposit_attribute.clone(),
            good_info_attribute.clone(),
            EventAttribute {
                key: DEPOSIT_IDENTITY_KEY.parse().unwrap(),
                value: "3xoM5GmUSq7YW4YNQrax1fEFLw1GbZozxe6UUoJcrqLG"
                    .parse()
                    .unwrap(),
                index: false,
            },
            EventAttribute {
                key: DEPOSIT_ENCRYPTION_KEY.parse().unwrap(),
                value: "HxnTpWTkgigSTAysVKLE8pEiUULHdTT1BxFfzfJvQRi6"
                    .parse()
                    .unwrap(),
                index: false,
            },
        ];
        let res = validate_deposit_tx(&correct_request, tx_entry.clone()).await;
        assert!(res.is_ok())
    }
}
