// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_interface::{
    elgamal::PublicKey, Attribute, BlindSignRequest, BlindSignRequestBody, BlindedSignature,
    BlindedSignatureResponse, KeyPair, Parameters, VerificationKeyResponse,
};
use config::defaults::VALIDATOR_API_VERSION;
use getset::{CopyGetters, Getters};
use rocket::fairing::AdHoc;
use rocket::serde::json::Json;
use rocket::State;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::AccountId;

mod dealer;
mod issuer;

// temp

// make sure keys are zeroized on drop
struct SecretKey(Vec<()>);
struct VerificationKey(Vec<()>);

pub(crate) struct Config {
    dkg_contract: AccountId,
}

enum IssuerState {
    // has only secure channel keypair
    PreDkg,

    // has node id
    DuringDkg,

    // has actual coconut keys
    PostDkg,
}

pub(crate) struct CredentialIssuer {
    config: Config,

    // by seeing how it looks, we'll probably need some wrapper with an inner field
    partial_secret_key: Arc<RwLock<Option<SecretKey>>>,
    partial_verification_key: Arc<RwLock<Option<VerificationKey>>>,
    // this would need to be stored somewhere
    // secure_channel_keys: secure_channel::KeyPair,
}

impl CredentialIssuer {
    pub(crate) fn new() -> CredentialIssuer {
        todo!()
    }

    pub(crate) fn perform_initial_something() {}
}

// everything below will be refactored and move elsewhere, but for time being I'm not touch them
// as I don't want to deal with broken builds (just yet)

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
                routes![post_blind_sign, get_verification_key],
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
