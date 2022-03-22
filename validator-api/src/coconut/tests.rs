// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::InternalSignRequest;
use coconut_interface::VerificationKeyResponse;
use nymcoconut::{ttp_keygen, Base58, KeyPair, Parameters};
use rocket::http::Status;
use rocket::local::blocking::Client;
use validator_client::validator_api::routes::{API_VERSION, COCONUT_VERIFICATION_KEY};

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
fn get_verification_key() {
    let params = Parameters::new(4).unwrap();
    let num_authorities = 4;

    let key_pairs = ttp_keygen(&params, num_authorities, num_authorities).unwrap();
    for key_pair in key_pairs.into_iter() {
        check_signer_verif_key(key_pair);
    }
}
