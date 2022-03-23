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
    use nymcoconut::{prepare_blind_sign, Parameters};
    use validator_client::nymd::{Event, Tag};

    #[tokio::test]
    async fn extract_encryption_key_errors() {
        let tx_hash =
            "6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E".to_string();
        let mut tx_entry = tx_entry_fixture(&tx_hash);
        let params = Parameters::new(4).unwrap();
        let voucher = BandwidthVoucher::new(
            &params,
            "1234",
            VOUCHER_INFO,
            tx_hash.clone(),
            "Signing key".to_string(),
            "Encryption key".to_string(),
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
            tx_hash.clone(),
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
            tx_hash.clone(),
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
            tx_hash.clone(),
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
            CoconutError::DifferentPublicAttributes(10.to_string(), 1234.to_string()).to_string(),
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
                value: "6EJGMdEq7t8Npz54uPkftGsdmj7DKntLVputAnDfVZB2"
                    .parse()
                    .unwrap(),
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
    }
}
