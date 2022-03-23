// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::InternalSignRequest;
use crate::coconut::error::{CoconutError, Result};
use coconut_bandwidth_contract::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ENCRYPTION_KEY, DEPOSIT_INFO, DEPOSIT_VALUE,
    DEPOSIT_VERIFICATION_KEY,
};
use coconut_interface::{BlindSignRequestBody, BlindedSignatureResponse, VerificationKeyResponse};
use config::defaults::VOUCHER_INFO;
use credentials::coconut::bandwidth::BandwidthVoucher;
use nymcoconut::{prepare_blind_sign, ttp_keygen, Base58, KeyPair, Parameters};
use validator_client::nymd::{tx::Hash, DeliverTx, Event, Tag, TxResponse};
use validator_client::validator_api::routes::{
    API_VERSION, COCONUT_BLIND_SIGN, COCONUT_VERIFICATION_KEY,
};

use async_trait::async_trait;
use rocket::http::Status;
use rocket::local::blocking::Client;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

struct DummyClient {
    db: Arc<RwLock<HashMap<String, TxResponse>>>,
}

impl DummyClient {
    pub fn new(db: &Arc<RwLock<HashMap<String, TxResponse>>>) -> Self {
        let db = Arc::clone(db);
        Self { db }
    }
}

#[async_trait]
impl super::client::Client for DummyClient {
    async fn get_tx(&self, tx_hash: &str) -> Result<TxResponse> {
        self.db
            .read()
            .unwrap()
            .get(tx_hash)
            .cloned()
            .ok_or(CoconutError::TxHashParseError)
    }
}

fn tx_entry_fixture(tx_hash: &str) -> TxResponse {
    TxResponse {
        hash: Hash::from_str(tx_hash).unwrap(),
        height: Default::default(),
        index: 0,
        tx_result: DeliverTx {
            code: Default::default(),
            data: Default::default(),
            log: Default::default(),
            info: Default::default(),
            gas_wanted: Default::default(),
            gas_used: Default::default(),
            events: vec![],
            codespace: Default::default(),
        },
        tx: vec![].into(),
        proof: None,
    }
}

fn check_signer_verif_key(key_pair: KeyPair) {
    let verification_key = key_pair.verification_key();

    let mut db_dir = std::env::temp_dir();
    db_dir.push(&verification_key.to_bs58()[..8]);
    let db = sled::open(db_dir).unwrap();
    let nymd_db = Arc::new(RwLock::new(HashMap::new()));
    let nymd_client = DummyClient::new(&nymd_db);

    let rocket = rocket::build().attach(InternalSignRequest::stage(nymd_client, key_pair, db));

    let client = Client::tracked(rocket).expect("valid rocket instance");

    let response = client
        .get(format!("/{}/{}", API_VERSION, COCONUT_VERIFICATION_KEY))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);

    let verification_key_response = response.into_json::<VerificationKeyResponse>().unwrap();
    assert_eq!(verification_key_response.key, verification_key);
}

fn post_and_check_error(
    client: &Client,
    request_body: BlindSignRequestBody,
    expected_error: String,
) {
    let response = client
        .post(format!("/{}/{}", API_VERSION, COCONUT_BLIND_SIGN))
        .json(&request_body)
        .dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(response.into_string().unwrap(), expected_error);
}

#[test]
fn multiple_verification_key() {
    let params = Parameters::new(4).unwrap();
    let num_authorities = 4;

    let key_pairs = ttp_keygen(&params, num_authorities, num_authorities).unwrap();
    for key_pair in key_pairs.into_iter() {
        check_signer_verif_key(key_pair);
    }
}

#[test]
fn blind_sign() {
    let tx_hash = String::from("6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E");
    let mut tx_entry = tx_entry_fixture(&tx_hash);
    let signature = String::from(
        "2DHbEZ6pzToGpsAXJrqJi7Wj1pAXeT18283q2YEEyNH5gTymwRozWBdja6SMAVt1dyYmUnM4ZNhsJ4wxZyGh4Z6J",
    );
    let signing_key = String::from("Signing key");
    let encryption_key = String::from("Encryption key");

    let params = Parameters::new(4).unwrap();
    let voucher = BandwidthVoucher::new(
        &params,
        "1234",
        VOUCHER_INFO,
        tx_hash.clone(),
        signing_key,
        encryption_key,
    );
    let (_, blind_sign_req) = prepare_blind_sign(
        &params,
        &voucher.get_private_attributes(),
        &voucher.get_public_attributes(),
    )
    .unwrap();

    let key_pair = ttp_keygen(&params, 1, 1).unwrap().remove(0);
    let mut db_dir = std::env::temp_dir();
    db_dir.push(&key_pair.verification_key().to_bs58()[..8]);
    let db = sled::open(db_dir).unwrap();
    let nymd_db = Arc::new(RwLock::new(HashMap::new()));
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    let nymd_client = DummyClient::new(&nymd_db);

    let rocket = rocket::build().attach(InternalSignRequest::stage(
        nymd_client,
        key_pair,
        db.clone(),
    ));
    let client = Client::tracked(rocket).expect("valid rocket instance");

    post_and_check_error(
        &client,
        BlindSignRequestBody::new(
            &blind_sign_req,
            tx_hash.clone(),
            signature.clone(),
            &voucher.get_public_attributes(),
            vec![
                String::from("First wrong plain"),
                String::from("Second wrong plain"),
            ],
            4,
        ),
        CoconutError::InconsistentPublicAttributes.to_string(),
    );
    post_and_check_error(
        &client,
        BlindSignRequestBody::new(
            &blind_sign_req,
            tx_hash.clone(),
            String::from("Invalid signature"),
            &voucher.get_public_attributes(),
            voucher.get_public_attributes_plain(),
            4,
        ),
        CoconutError::Ed25519ParseError(
            // this is really just a useless, dummy error value needed to generate the error type
            // and get its string representation
            crypto::asymmetric::identity::Ed25519RecoveryError::MalformedBytes(
                crypto::asymmetric::identity::SignatureError::new(),
            ),
        )
        .to_string(),
    );

    let correct_request = BlindSignRequestBody::new(
        &blind_sign_req,
        tx_hash.clone(),
        signature.clone(),
        &voucher.get_public_attributes(),
        voucher.get_public_attributes_plain(),
        4,
    );
    post_and_check_error(
        &client,
        correct_request.clone(),
        CoconutError::DepositEventNotFound.to_string(),
    );

    tx_entry.tx_result.events.push(Event {
        type_str: format!("wasm-{}", DEPOSITED_FUNDS_EVENT_TYPE),
        attributes: vec![],
    });
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    post_and_check_error(
        &client,
        correct_request.clone(),
        CoconutError::DepositValueNotFound.to_string(),
    );

    tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![Tag {
        key: DEPOSIT_VALUE.parse().unwrap(),
        value: "10".parse().unwrap(),
    }];
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    post_and_check_error(
        &client,
        correct_request.clone(),
        CoconutError::DifferentPublicAttributes(10.to_string(), 1234.to_string()).to_string(),
    );

    tx_entry.tx_result.events.get_mut(0).unwrap().attributes = vec![Tag {
        key: DEPOSIT_VALUE.parse().unwrap(),
        value: "1234".parse().unwrap(),
    }];
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    post_and_check_error(
        &client,
        correct_request.clone(),
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
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    post_and_check_error(
        &client,
        correct_request.clone(),
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
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    post_and_check_error(
        &client,
        correct_request.clone(),
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
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    post_and_check_error(
        &client,
        correct_request.clone(),
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
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    post_and_check_error(
        &client,
        correct_request.clone(),
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
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    post_and_check_error(
        &client,
        correct_request.clone(),
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
    nymd_db
        .write()
        .unwrap()
        .insert(tx_hash.clone(), tx_entry.clone());
    post_and_check_error(
        &client,
        correct_request.clone(),
        CoconutError::SignatureVerificationError(
            crypto::asymmetric::identity::SignatureError::default(),
        )
        .to_string(),
    );

    let request_body = BlindSignRequestBody::new(
        &blind_sign_req,
        tx_hash.clone(),
        signature.clone(),
        &voucher.get_public_attributes(),
        voucher.get_public_attributes_plain(),
        4,
    );

    let encrypted_signature = vec![1, 2, 3, 4];
    let remote_key = [42; 32];
    let expected_response = BlindedSignatureResponse::new(encrypted_signature, remote_key);
    db.insert(tx_hash.as_bytes(), expected_response.to_bytes())
        .unwrap();

    let response = client
        .post(format!("/{}/{}", API_VERSION, COCONUT_BLIND_SIGN))
        .json(&request_body)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let blinded_signature_response = response.into_json::<BlindedSignatureResponse>().unwrap();
    assert_eq!(
        blinded_signature_response.to_bytes(),
        expected_response.to_bytes()
    );
}
