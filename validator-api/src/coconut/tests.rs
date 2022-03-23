// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::InternalSignRequest;
use crate::coconut::error::{CoconutError, Result};
use coconut_interface::{BlindSignRequestBody, BlindedSignatureResponse, VerificationKeyResponse};
use config::defaults::VOUCHER_INFO;
use credentials::coconut::bandwidth::BandwidthVoucher;
use crypto::shared_key::recompute_shared_key;
use crypto::symmetric::stream_cipher;
use crypto::{aes::Aes128, blake3, ctr};
use nymcoconut::{prepare_blind_sign, ttp_keygen, Base58, BlindedSignature, KeyPair, Parameters};
use validator_client::nymd::{tx::Hash, DeliverTx, TxResponse};
use validator_client::validator_api::routes::{
    API_VERSION, COCONUT_BLIND_SIGN, COCONUT_VERIFICATION_KEY,
};

use crate::coconut::State;
use async_trait::async_trait;
use rand_07::rngs::OsRng;
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

pub fn tx_entry_fixture(tx_hash: &str) -> TxResponse {
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
fn signed_before() {
    let tx_hash = String::from("6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E");
    let tx_entry = tx_entry_fixture(&tx_hash);
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

#[tokio::test]
async fn state_functions() {
    let nymd_db = Arc::new(RwLock::new(HashMap::new()));
    let nymd_client = DummyClient::new(&nymd_db);
    let params = Parameters::new(4).unwrap();
    let key_pair = ttp_keygen(&params, 1, 1).unwrap().remove(0);
    let mut db_dir = std::env::temp_dir();
    db_dir.push(&key_pair.verification_key().to_bs58()[..8]);
    let db = sled::open(db_dir).unwrap();
    let state = State::new(nymd_client, key_pair, db.clone());

    let tx_hash = String::from("6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E");
    assert!(state.signed_before(tx_hash.as_bytes()).unwrap().is_none());

    let encrypted_signature = vec![1, 2, 3, 4];
    let remote_key = [42; 32];
    let expected_response = BlindedSignatureResponse::new(encrypted_signature, remote_key);
    db.insert(tx_hash.as_bytes(), expected_response.to_bytes())
        .unwrap();
    assert_eq!(
        state
            .signed_before(tx_hash.as_bytes())
            .unwrap()
            .unwrap()
            .to_bytes(),
        expected_response.to_bytes()
    );

    let encryption_keypair = crypto::asymmetric::encryption::KeyPair::new(&mut OsRng);
    let blinded_signature = BlindedSignature::from_bytes(&[
        183, 217, 166, 113, 40, 123, 74, 25, 72, 31, 136, 19, 125, 95, 217, 228, 96, 113, 25, 240,
        12, 102, 125, 11, 174, 20, 216, 82, 192, 71, 27, 194, 48, 20, 17, 95, 243, 179, 82, 21, 57,
        143, 101, 19, 22, 186, 147, 13, 147, 238, 39, 119, 15, 36, 251, 131, 250, 38, 185, 113,
        187, 40, 227, 107, 134, 190, 123, 183, 126, 176, 226, 173, 147, 137, 17, 175, 13, 115, 78,
        222, 119, 93, 146, 116, 229, 0, 152, 51, 232, 2, 102, 204, 147, 202, 254, 243,
    ])
    .unwrap();
    // Check that the new payload is not stored if there was already something signed for tx_hash
    assert_eq!(
        state
            .encrypt_and_store(
                tx_hash.as_bytes(),
                encryption_keypair.public_key(),
                &blinded_signature,
            )
            .await
            .unwrap()
            .to_bytes(),
        expected_response.to_bytes()
    );

    // And use a new hash to store a new signature
    let tx_hash = String::from("97D64C38D6601B1F0FD3A82E20D252685CB7A210AFB0261018590659AB82B0BF");
    let response = state
        .encrypt_and_store(
            tx_hash.as_bytes(),
            encryption_keypair.public_key(),
            &blinded_signature,
        )
        .await
        .unwrap();
    let remote_key =
        crypto::asymmetric::encryption::PublicKey::from_bytes(&response.remote_key).unwrap();

    let encryption_key = recompute_shared_key::<ctr::Ctr64LE<Aes128>, blake3::Hasher>(
        &remote_key,
        encryption_keypair.private_key(),
    );
    let zero_iv = stream_cipher::zero_iv::<ctr::Ctr64LE<Aes128>>();
    let blinded_signature_bytes = stream_cipher::decrypt::<ctr::Ctr64LE<Aes128>>(
        &encryption_key,
        &zero_iv,
        &response.encrypted_signature,
    );

    let received_blinded_signature =
        BlindedSignature::from_bytes(&blinded_signature_bytes).unwrap();
    assert_eq!(
        blinded_signature.to_bytes(),
        received_blinded_signature.to_bytes()
    );

    // Check that the same value for tx_hash is returned

    let other_signature = BlindedSignature::from_bytes(&[
        183, 217, 166, 113, 40, 123, 74, 25, 72, 31, 136, 19, 125, 95, 217, 228, 96, 113, 25, 240,
        12, 102, 125, 11, 174, 20, 216, 82, 192, 71, 27, 194, 48, 20, 17, 95, 243, 179, 82, 21, 57,
        143, 101, 19, 22, 186, 147, 13, 131, 236, 38, 138, 192, 235, 169, 142, 176, 118, 153, 238,
        141, 91, 94, 139, 168, 214, 17, 250, 96, 206, 139, 89, 139, 87, 31, 8, 106, 171, 8, 140,
        201, 158, 18, 152, 24, 98, 153, 170, 141, 35, 190, 200, 19, 148, 71, 197,
    ])
    .unwrap();
    assert_eq!(
        state
            .encrypt_and_store(
                tx_hash.as_bytes(),
                encryption_keypair.public_key(),
                &other_signature,
            )
            .await
            .unwrap()
            .to_bytes(),
        response.to_bytes()
    );
}
