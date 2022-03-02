// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bandwidth_claim_contract::events::{VOUCHER_ACQUIRED_EVENT_TYPE, VOUCHER_VALUE};
use bip39::Mnemonic;
use coconut_interface::{
    elgamal::PublicKey, Attribute, BlindSignRequest, BlindSignRequestBody, BlindedSignature,
    BlindedSignatureResponse, KeyPair, Parameters, VerificationKeyResponse, VerifyCredentialBody,
    VerifyCredentialResponse,
};
use config::defaults::VALIDATOR_API_VERSION;
use getset::{CopyGetters, Getters};
use rocket::fairing::AdHoc;
use rocket::serde::json::Json;
use rocket::State;
use std::str::FromStr;
use url::Url;
use validator_client::nymd::tx::Hash;
use validator_client::nymd::{AccountId, NymdClient};

#[derive(Getters, CopyGetters, Debug)]
pub(crate) struct InternalSignRequest {
    // Total number of parameters to generate for
    #[getset(get_copy)]
    total_params: u32,
    #[getset(get)]
    public_attributes: Vec<Attribute>,
    #[getset(get)]
    public_key: PublicKey,
    #[getset(get)]
    blind_sign_request: BlindSignRequest,
}

impl InternalSignRequest {
    pub fn new(
        total_params: u32,
        public_attributes: Vec<Attribute>,
        public_key: PublicKey,
        blind_sign_request: BlindSignRequest,
    ) -> InternalSignRequest {
        InternalSignRequest {
            total_params,
            public_attributes,
            public_key,
            blind_sign_request,
        }
    }

    pub fn stage(key_pair: KeyPair) -> AdHoc {
        AdHoc::on_ignite("Internal Sign Request Stage", |rocket| async {
            rocket.manage(key_pair).mount(
                // this format! is so ugly...
                format!("/{}", VALIDATOR_API_VERSION),
                routes![
                    post_blind_sign,
                    get_verification_key,
                    post_verify_credential
                ],
            )
        })
    }
}

fn blind_sign(request: InternalSignRequest, key_pair: &KeyPair) -> BlindedSignature {
    let params = Parameters::new(request.total_params()).unwrap();
    coconut_interface::blind_sign(
        &params,
        &key_pair.secret_key(),
        request.public_key(),
        request.blind_sign_request(),
        request.public_attributes(),
    )
    .unwrap()
}

#[post("/blind-sign", data = "<blind_sign_request_body>")]
//  Until we have serialization and deserialization traits we'll be using a crutch
pub async fn post_blind_sign(
    blind_sign_request_body: Json<BlindSignRequestBody>,
    key_pair: &State<KeyPair>,
) -> Json<BlindedSignatureResponse> {
    debug!("{:?}", blind_sign_request_body);
    let nymd_url = Url::from_str("http://127.0.0.1:26657").unwrap();
    let mnemonic = Mnemonic::from_str(&"have armor behind appear labor choose fire erase arrive slice mother acid second rely exhibit grief soul super record useless antique excite ocean walnut").unwrap();
    let nymd_client = NymdClient::connect_with_mnemonic(
        config::defaults::all::Network::SANDBOX,
        nymd_url.as_ref(),
        None,
        None,
        AccountId::from_str("nymt14hj2tavq8fpesdwxxcu44rty3hh90vhuysqrsr").ok(),
        mnemonic,
        None,
    )
    .expect("Could not create nymd client");
    println!("Looking at tx {}", blind_sign_request_body.0.tx_hash());
    let response = nymd_client
        .get_tx(Hash::from_str(blind_sign_request_body.0.tx_hash()).unwrap())
        .await
        .unwrap();
    println!("Events: {:?}", response.tx_result.events);
    let bandwidth_str = response
        .tx_result
        .events
        .iter()
        .filter(|event| event.type_str == format!("wasm-{}", VOUCHER_ACQUIRED_EVENT_TYPE))
        .map(|event| {
            event
                .attributes
                .iter()
                .filter(|tag| tag.key.as_ref() == VOUCHER_VALUE)
                .last()
                .unwrap()
                .value
                .as_ref()
        })
        .last()
        .unwrap();
    println!("Bandwidth str: {}", bandwidth_str);
    let acuired_bandwidth = Attribute::from(u64::from_str(bandwidth_str).unwrap());
    let requested_bandwidth = blind_sign_request_body.0.public_attributes()[0];
    if acuired_bandwidth != requested_bandwidth {
        panic!(
            "Bandwidth value mismatch: {} vs {}",
            acuired_bandwidth, requested_bandwidth
        );
    }
    let internal_request = InternalSignRequest::new(
        *blind_sign_request_body.total_params(),
        blind_sign_request_body.public_attributes(),
        blind_sign_request_body.public_key().clone(),
        blind_sign_request_body.blind_sign_request().clone(),
    );
    let blinded_signature = blind_sign(internal_request, key_pair);
    Json(BlindedSignatureResponse::new(blinded_signature))
}

#[get("/verification-key")]
pub async fn get_verification_key(key_pair: &State<KeyPair>) -> Json<VerificationKeyResponse> {
    Json(VerificationKeyResponse::new(key_pair.verification_key()))
}

#[post("/verify-credential", data = "<_verify_credential_request_body>")]
pub async fn post_verify_credential(
    _verify_credential_request_body: Json<VerifyCredentialBody>,
    _key_pair: &State<KeyPair>,
) -> Json<VerifyCredentialResponse> {
    Json(VerifyCredentialResponse { response: true })
}
