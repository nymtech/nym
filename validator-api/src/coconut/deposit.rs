// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_bandwidth_contract::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ENCRYPTION_KEY, DEPOSIT_INFO, DEPOSIT_VALUE,
    DEPOSIT_VERIFICATION_KEY,
};
use coconut_interface::BlindSignRequestBody;
use credentials::coconut::bandwidth::BandwidthVoucher;
use crypto::asymmetric::encryption;
use crypto::asymmetric::identity::{self, Signature};
use validator_client::nymd::TxResponse;

use super::error::{CoconutError, Result};

pub async fn extract_encryption_key(
    blind_sign_request_body: &BlindSignRequestBody,
    tx: TxResponse,
) -> Result<encryption::PublicKey> {
    let blind_sign_request = blind_sign_request_body.blind_sign_request();
    let public_attributes = blind_sign_request_body.public_attributes();
    let public_attributes_plain = blind_sign_request_body.public_attributes_plain();

    if !BandwidthVoucher::verify_against_plain(&public_attributes, public_attributes_plain) {
        return Err(CoconutError::InconsistentPublicAttributes);
    }

    let tx_hash_str = blind_sign_request_body.tx_hash();
    let mut message = blind_sign_request.to_bytes();
    message.extend_from_slice(tx_hash_str.as_bytes());

    let signature = Signature::from_base58_string(blind_sign_request_body.signature())?;

    let attributes: &Vec<_> = tx
        .tx_result
        .events
        .iter()
        .find(|event| event.type_str == format!("wasm-{}", DEPOSITED_FUNDS_EVENT_TYPE))
        .ok_or(CoconutError::DepositEventNotFound)?
        .attributes
        .as_ref();

    let deposit_value = attributes
        .iter()
        .find(|tag| tag.key.as_ref() == DEPOSIT_VALUE)
        .ok_or(CoconutError::DepositValueNotFound)?
        .value
        .as_ref();
    let deposit_value_plain = public_attributes_plain.get(0).cloned().unwrap_or_default();
    if deposit_value != deposit_value_plain {
        return Err(CoconutError::DifferentPublicAttributes(
            deposit_value.to_string(),
            deposit_value_plain,
        ));
    }

    let deposit_info = attributes
        .iter()
        .find(|tag| tag.key.as_ref() == DEPOSIT_INFO)
        .ok_or(CoconutError::DepositInfoNotFound)?
        .value
        .as_ref();
    let deposit_info_plain = public_attributes_plain.get(1).cloned().unwrap_or_default();
    if deposit_info != deposit_info_plain {
        return Err(CoconutError::DifferentPublicAttributes(
            deposit_info.to_string(),
            deposit_info_plain,
        ));
    }

    let verification_key = identity::PublicKey::from_base58_string(
        attributes
            .iter()
            .find(|tag| tag.key.as_ref() == DEPOSIT_VERIFICATION_KEY)
            .ok_or(CoconutError::DepositVerifKeyNotFound)?
            .value
            .as_ref(),
    )?;

    let encryption_key = encryption::PublicKey::from_base58_string(
        attributes
            .iter()
            .find(|tag| tag.key.as_ref() == DEPOSIT_ENCRYPTION_KEY)
            .ok_or(CoconutError::DepositEncrKeyNotFound)?
            .value
            .as_ref(),
    )?;

    verification_key.verify(&message, &signature)?;

    Ok(encryption_key)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::coconut::tests::tx_entry_fixture;
    use config::defaults::VOUCHER_INFO;
    use nymcoconut::{prepare_blind_sign, BlindSignRequest, Parameters};
    use rand_07::rngs::OsRng;
    use std::str::FromStr;
    use validator_client::nymd::tx::Hash;
    use validator_client::nymd::{Event, Tag};

    #[tokio::test]
    async fn extract_encryption_key_test() {
        let tx_hash =
            Hash::from_str("6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E")
                .unwrap();
        let mut tx_entry = tx_entry_fixture(&tx_hash.to_string());
        let params = Parameters::new(4).unwrap();
        let mut rng = OsRng;
        let voucher = BandwidthVoucher::new(
            &params,
            "1234".to_string(),
            VOUCHER_INFO.to_string(),
            tx_hash.clone(),
            identity::PrivateKey::from_base58_string(
                identity::KeyPair::new(&mut rng)
                    .private_key()
                    .to_base58_string(),
            )
            .unwrap(),
            encryption::KeyPair::new(&mut rng).private_key().clone(),
        );
        let (_, blind_sign_req) = prepare_blind_sign(
            &params,
            &voucher.get_private_attributes(),
            &voucher.get_public_attributes(),
        )
        .unwrap();
        let signature = "2DHbEZ6pzToGpsAXJrqJi7Wj1pAXeT18283q2YEEyNH5gTymwRozWBdja6SMAVt1dyYmUnM4ZNhsJ4wxZyGh4Z6J".to_string();

        let req = BlindSignRequestBody::new(
            &blind_sign_req,
            tx_hash.to_string(),
            signature.clone(),
            &voucher.get_public_attributes(),
            vec![
                String::from("First wrong plain"),
                String::from("Second wrong plain"),
            ],
            4,
        );
        let err = extract_encryption_key(&req, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::InconsistentPublicAttributes.to_string()
        );

        let req = BlindSignRequestBody::new(
            &blind_sign_req,
            tx_hash.to_string(),
            String::from("Invalid signature"),
            &voucher.get_public_attributes(),
            voucher.get_public_attributes_plain(),
            4,
        );
        let err = extract_encryption_key(&req, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::Ed25519ParseError(
                // this is really just a useless, dummy error value needed to generate the error type
                // and get its string representation
                crypto::asymmetric::identity::Ed25519RecoveryError::MalformedBytes(
                    crypto::asymmetric::identity::SignatureError::new(),
                ),
            )
            .to_string()
        );

        let correct_request = BlindSignRequestBody::new(
            &blind_sign_req,
            tx_hash.to_string(),
            signature.clone(),
            &voucher.get_public_attributes(),
            voucher.get_public_attributes_plain(),
            4,
        );

        tx_entry.tx_result.events.push(Event {
            type_str: format!("wasm-{}", DEPOSITED_FUNDS_EVENT_TYPE),
            attributes: vec![],
        });
        let err = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DepositValueNotFound.to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![Tag {
            key: DEPOSIT_VALUE.parse().unwrap(),
            value: "10".parse().unwrap(),
        }];
        let err = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DifferentPublicAttributes("10".to_string(), "1234".to_string())
                .to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![Tag {
            key: DEPOSIT_VALUE.parse().unwrap(),
            value: "1234".parse().unwrap(),
        }];
        let err = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DepositInfoNotFound.to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            Tag {
                key: DEPOSIT_VALUE.parse().unwrap(),
                value: "1234".parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_INFO.parse().unwrap(),
                value: "bandwidth deposit info".parse().unwrap(),
            },
        ];
        let err = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DifferentPublicAttributes(
                "bandwidth deposit info".to_string(),
                VOUCHER_INFO.to_string(),
            )
            .to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            Tag {
                key: DEPOSIT_VALUE.parse().unwrap(),
                value: "1234".parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_INFO.parse().unwrap(),
                value: VOUCHER_INFO.parse().unwrap(),
            },
        ];
        let err = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DepositVerifKeyNotFound.to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            Tag {
                key: DEPOSIT_VALUE.parse().unwrap(),
                value: "1234".parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_INFO.parse().unwrap(),
                value: VOUCHER_INFO.parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_VERIFICATION_KEY.parse().unwrap(),
                value: "verification key".parse().unwrap(),
            },
        ];
        let err = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::Ed25519ParseError(
                // this is really just a useless, dummy error value needed to generate the error type
                // and get its string representation
                crypto::asymmetric::identity::Ed25519RecoveryError::MalformedBytes(
                    crypto::asymmetric::identity::SignatureError::new(),
                ),
            )
            .to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            Tag {
                key: DEPOSIT_VALUE.parse().unwrap(),
                value: "1234".parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_INFO.parse().unwrap(),
                value: VOUCHER_INFO.parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_VERIFICATION_KEY.parse().unwrap(),
                value: "2eSxwquNJb2nZTEW5p4rbqjHfBaz9UaNhjHHiexPN4He"
                    .parse()
                    .unwrap(),
            },
        ];
        let err = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::DepositEncrKeyNotFound.to_string(),
        );

        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            Tag {
                key: DEPOSIT_VALUE.parse().unwrap(),
                value: "1234".parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_INFO.parse().unwrap(),
                value: VOUCHER_INFO.parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_VERIFICATION_KEY.parse().unwrap(),
                value: "6EJGMdEq7t8Npz54uPkftGsdmj7DKntLVputAnDfVZB2"
                    .parse()
                    .unwrap(),
            },
            Tag {
                key: DEPOSIT_ENCRYPTION_KEY.parse().unwrap(),
                value: "encryption key".parse().unwrap(),
            },
        ];
        let err = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::X25519ParseError(
                // this is really just a useless, dummy error value needed to generate the error type
                // and get its string representation
                crypto::asymmetric::encryption::KeyRecoveryError::InvalidPublicKeyBytes,
            )
            .to_string(),
        );

        let expected_encryption_key = "HxnTpWTkgigSTAysVKLE8pEiUULHdTT1BxFfzfJvQRi6";
        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            Tag {
                key: DEPOSIT_VALUE.parse().unwrap(),
                value: "1234".parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_INFO.parse().unwrap(),
                value: VOUCHER_INFO.parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_VERIFICATION_KEY.parse().unwrap(),
                value: "6EJGMdEq7t8Npz54uPkftGsdmj7DKntLVputAnDfVZB2"
                    .parse()
                    .unwrap(),
            },
            Tag {
                key: DEPOSIT_ENCRYPTION_KEY.parse().unwrap(),
                value: expected_encryption_key.parse().unwrap(),
            },
        ];
        let err = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            CoconutError::SignatureVerificationError(
                crypto::asymmetric::identity::SignatureError::default(),
            )
            .to_string(),
        );

        // hard-coded values, that generate a correct signature
        let blind_sign_req = BlindSignRequest::from_bytes(&[
            176, 113, 19, 237, 218, 252, 113, 20, 225, 238, 59, 88, 217, 45, 233, 178, 65, 28, 242,
            0, 222, 48, 110, 216, 26, 111, 51, 235, 61, 74, 200, 15, 130, 245, 45, 170, 155, 190,
            156, 77, 180, 142, 29, 63, 15, 224, 150, 31, 139, 24, 65, 175, 143, 153, 11, 203, 33,
            16, 152, 22, 221, 203, 99, 233, 208, 142, 161, 194, 46, 227, 177, 96, 119, 30, 175, 69,
            104, 14, 2, 191, 26, 94, 30, 165, 15, 28, 40, 176, 1, 78, 253, 79, 20, 137, 102, 74, 2,
            0, 0, 0, 0, 0, 0, 0, 131, 133, 112, 115, 53, 98, 58, 166, 240, 70, 185, 210, 203, 12,
            114, 66, 180, 38, 139, 12, 187, 45, 250, 201, 68, 102, 159, 172, 218, 124, 151, 23,
            172, 18, 216, 122, 246, 7, 185, 76, 20, 167, 123, 122, 152, 241, 175, 226, 176, 8, 170,
            70, 140, 252, 36, 130, 67, 204, 111, 116, 107, 92, 200, 77, 252, 31, 138, 18, 10, 215,
            165, 243, 95, 199, 193, 61, 200, 187, 22, 198, 109, 213, 145, 71, 171, 132, 174, 68,
            105, 248, 0, 115, 50, 55, 199, 84, 67, 16, 125, 216, 250, 154, 115, 174, 9, 206, 44,
            88, 63, 163, 124, 10, 239, 64, 158, 191, 27, 169, 177, 194, 223, 142, 202, 206, 189,
            122, 123, 91, 171, 15, 40, 192, 148, 75, 174, 24, 116, 229, 127, 170, 110, 183, 151, 2,
            118, 168, 22, 113, 87, 237, 91, 228, 249, 120, 114, 255, 53, 175, 245, 39, 2, 0, 0, 0,
            0, 0, 0, 0, 225, 45, 230, 25, 62, 202, 96, 166, 171, 241, 206, 137, 254, 51, 154, 255,
            122, 130, 107, 54, 5, 206, 207, 120, 193, 214, 64, 10, 111, 195, 86, 55, 201, 36, 10,
            18, 154, 158, 183, 87, 185, 59, 228, 89, 134, 193, 217, 188, 64, 164, 249, 21, 248, 20,
            207, 58, 31, 10, 19, 176, 246, 150, 45, 48, 2, 0, 0, 0, 0, 0, 0, 0, 173, 60, 65, 209,
            100, 114, 138, 186, 158, 150, 109, 230, 111, 86, 101, 72, 194, 237, 173, 195, 139, 175,
            238, 25, 169, 18, 188, 75, 77, 54, 111, 20, 115, 235, 195, 2, 123, 133, 164, 81, 15,
            45, 11, 84, 139, 38, 8, 224, 197, 181, 95, 147, 49, 77, 193, 207, 52, 141, 195, 195,
            66, 137, 17, 32,
        ])
        .unwrap();
        let correct_request = BlindSignRequestBody::new(
            &blind_sign_req,
            "7C41AF8266D91DE55E1C8F4712E6A952A165ED3D8C27C7B00428CBD0DE00A52B".to_string(),
            "gSFgpma5GAVMcsmZwKieqGNHNd3dPzcfa8eT2Qn2LoBccSeyiJdphREbNrkuh5XWxMe2hUsranaYzLro48L9Qhd".to_string(),
            &voucher.get_public_attributes(),
            voucher.get_public_attributes_plain(),
            4,
        );
        tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![
            Tag {
                key: DEPOSIT_VALUE.parse().unwrap(),
                value: "1234".parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_INFO.parse().unwrap(),
                value: VOUCHER_INFO.parse().unwrap(),
            },
            Tag {
                key: DEPOSIT_VERIFICATION_KEY.parse().unwrap(),
                value: "64auwDkWan7R8yH1Mwe9dS4qXgrDBCUNDg3Q4KFnd2P5"
                    .parse()
                    .unwrap(),
            },
            Tag {
                key: DEPOSIT_ENCRYPTION_KEY.parse().unwrap(),
                value: "HxnTpWTkgigSTAysVKLE8pEiUULHdTT1BxFfzfJvQRi6"
                    .parse()
                    .unwrap(),
            },
        ];
        let encryption_key = extract_encryption_key(&correct_request, tx_entry.clone())
            .await
            .unwrap();
        assert_eq!(encryption_key.to_base58_string(), expected_encryption_key);
    }
}
