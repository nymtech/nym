// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::InternalSignRequest;
use crate::coconut::error::CoconutError;
use coconut_interface::{BlindSignRequestBody, BlindedSignatureResponse, VerificationKeyResponse};
use config::defaults::VOUCHER_INFO;
use credentials::coconut::bandwidth::BandwidthVoucher;
use nymcoconut::{prepare_blind_sign, ttp_keygen, Base58, KeyPair, Parameters};
use rocket::http::Status;
use rocket::local::blocking::Client;
use validator_client::validator_api::routes::{
    API_VERSION, COCONUT_BLIND_SIGN, COCONUT_VERIFICATION_KEY,
};

fn check_signer_verif_key(key_pair: KeyPair) {
    let verification_key = key_pair.verification_key();

    let mut db_dir = std::env::temp_dir();
    db_dir.push(&verification_key.to_bs58()[..8]);
    let db = sled::open(db_dir).unwrap();

    let rocket = rocket::build().attach(InternalSignRequest::stage(key_pair, db));

    let client = Client::tracked(rocket).expect("valid rocket instance");

    let response = client
        .get(format!("/{}/{}", API_VERSION, COCONUT_VERIFICATION_KEY))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);

    let verification_key_response = response.into_json::<VerificationKeyResponse>().unwrap();
    assert_eq!(verification_key_response.key, verification_key);
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
    let signature = String::from(
        "4KqGroRpbHC2UTuUWg6XdVcu7jDpz22SLNTpXpVC4y9JVmtEvyCGnwDqYwbVBLqrkVxgEm9dfMPCi5u3NSGK3ouz",
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

    let rocket = rocket::build().attach(InternalSignRequest::stage(key_pair, db.clone()));
    let client = Client::tracked(rocket).expect("valid rocket instance");

    let request_body = BlindSignRequestBody::new(
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
    let response = client
        .post(format!("/{}/{}", API_VERSION, COCONUT_BLIND_SIGN))
        .json(&request_body)
        .dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().unwrap(),
        CoconutError::InconsistentPublicAttributes.to_string()
    );

    let request_body = BlindSignRequestBody::new(
        &blind_sign_req,
        tx_hash.clone(),
        String::from("Invalid signature"),
        &voucher.get_public_attributes(),
        voucher.get_public_attributes_plain(),
        4,
    );

    let response = client
        .post(format!("/{}/{}", API_VERSION, COCONUT_BLIND_SIGN))
        .json(&request_body)
        .dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().unwrap(),
        CoconutError::Ed25519ParseError(
            // this is really just a useless, dummy error value needed to generate the error type
            // and get its string representation
            crypto::asymmetric::identity::Ed25519RecoveryError::MalformedBytes(
                crypto::asymmetric::identity::SignatureError::new()
            )
        )
        .to_string()
    );

    let request_body = BlindSignRequestBody::new(
        &blind_sign_req,
        String::from("Wrong tx hash"),
        signature.clone(),
        &voucher.get_public_attributes(),
        voucher.get_public_attributes_plain(),
        4,
    );

    let response = client
        .post(format!("/{}/{}", API_VERSION, COCONUT_BLIND_SIGN))
        .json(&request_body)
        .dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().unwrap(),
        CoconutError::TxHashParseError.to_string()
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
