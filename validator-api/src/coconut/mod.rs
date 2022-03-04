// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bandwidth_claim_contract::events::{VOUCHER_ACQUIRED_EVENT_TYPE, VOUCHER_VALUE};
use bip39::Mnemonic;
use coconut_interface::{
    elgamal::PublicKey, Attribute, Base58, BlindSignRequest, BlindSignRequestBody,
    BlindedSignature, BlindedSignatureResponse, Credential, KeyPair, Parameters, VerificationKey,
    VerificationKeyResponse, VerifyCredentialResponse,
};
use config::defaults::VALIDATOR_API_VERSION;
use cw3_flex_multisig::msg::ExecuteMsg;
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
        None,
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

#[post("/verify-credential", data = "<verify_credential_body>")]
pub async fn post_verify_credential(
    verify_credential_body: Json<Credential>,
    key_pair: &State<KeyPair>,
) -> Json<VerifyCredentialResponse> {
    println!(
        "Using verification key: {:?}",
        key_pair.verification_key().to_bs58()
    );
    let aggregated_verification_key = VerificationKey::try_from_bs58("4uTfTzJ1ViDLaWhDkZCHPsM9uv6GqDJ8bfHu6eKuQ5Zzan9KacaNCMuwtHDTpmZyfFHWuqi5cZL5HsDJ6RewGMyG13TTTn8fXdvs4TeuukTP5Kdn7ZpLEZmwra5gFZj3nokqpB6Kk2T88WwDVq5kHgtBikcG6N5fqJWmyb8TNhTjB3WQ87R4x5TbioLPRTRw9w4Ho2zgdGH1X3F99VKGWaYSNXTP22ganxCnd5Yjo3ARbFC21hc4qH7c4Y8EK4X8jML6MJTjbTpFQ5u6evib35knWf4rwm5Rtuoh8SgixmV8J5dovsJ4FbH9oB2PuWUf7hPThfY9ipqoefoFiMtGwT8wvNkB9zmGJqNDHUohoaZniBYSge3XYx8P53D8y1gkZVwTdL9TxRNpV3SoyLvXBWZL8Vv4tqEByhycKWYhgrmLDf5w8VS9riSqgJC2eqTDgNVxZrm8XZj2wArShFixsqiJHnhDzcMkUYx2vnEYdfE6FHYHncaoq58i32J9TaWM9sgvAnubcRPLofU8F45aR682tBYtEn3uNzxYEhgjuTmmiKuUifV79FBco3td8FTbwxz6yKxoWk3yJhPBo3fPXQoZFxDfB6CE5yp4ma1D7qdzYV1kJFcK7cCwqRZg6AveybdW9cDPMyPPzG2CqFSJMZvKKTB").unwrap();
    let response = verify_credential_body
        .0
        .verify(&aggregated_verification_key);
    if !response {
        return Json(VerifyCredentialResponse { response });
    }
    let mnemonic = if std::env::var("ROCKET_PORT") == Ok("8081".to_string()) {
        "have armor behind appear labor choose fire erase arrive slice mother acid second rely exhibit grief soul super record useless antique excite ocean walnut"
    } else if std::env::var("ROCKET_PORT") == Ok("8082".to_string()) {
        "inner luggage start square fabric ritual cereal engine winner tiny exile frozen end cherry loan humble laundry desk blur vicious word amount remove praise"
    } else {
        "hat pulse impulse prosper name rose auction grape stone leader book provide discover exchange drift story parent barely novel giggle deposit dizzy recipe where"
    };
    let mnemonic = Mnemonic::from_str(mnemonic).unwrap();
    let nymd_url = Url::from_str("http://127.0.0.1:26657").unwrap();
    let nymd_client = NymdClient::connect_with_mnemonic(
        config::defaults::all::Network::SANDBOX,
        nymd_url.as_ref(),
        None,
        None,
        None,
        mnemonic,
        None,
    )
    .expect("Could not create nymd client");
    let req = ExecuteMsg::Vote {
        proposal_id: *verify_credential_body.0.proposal_id(),
        vote: cw3::Vote::Yes,
    };
    nymd_client
        .execute(
            &AccountId::from_str("nymt1qwlgtx52gsdu7dtp0cekka5zehdl0uj3vqx3jd").unwrap(),
            &req,
            Default::default(),
            "",
            vec![],
        )
        .await
        .unwrap();
    println!("Sending response: {}", response);
    Json(VerifyCredentialResponse { response })
}
