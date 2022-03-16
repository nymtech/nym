// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod deposit;
mod error;

use crate::coconut::deposit::extract_encryption_key;
use crate::coconut::error::Result;

use coconut_interface::{
    Attribute, BlindSignRequest, BlindSignRequestBody, BlindedSignature, BlindedSignatureResponse,
    KeyPair, Parameters, VerificationKeyResponse,
};
use config::defaults::VALIDATOR_API_VERSION;
use getset::{CopyGetters, Getters};
use rocket::fairing::AdHoc;
use rocket::serde::json::Json;
use rocket::State as RocketState;

pub struct State {
    key_pair: KeyPair,
    signed_deposits: sled::Db,
}

impl State {
    pub fn new(key_pair: KeyPair, signed_deposits: sled::Db) -> Self {
        Self {
            key_pair,
            signed_deposits,
        }
    }
}

#[derive(Getters, CopyGetters, Debug)]
pub(crate) struct InternalSignRequest {
    // Total number of parameters to generate for
    #[getset(get_copy)]
    total_params: u32,
    #[getset(get)]
    public_attributes: Vec<Attribute>,
    #[getset(get)]
    blind_sign_request: BlindSignRequest,
}

impl InternalSignRequest {
    pub fn new(
        total_params: u32,
        public_attributes: Vec<Attribute>,
        blind_sign_request: BlindSignRequest,
    ) -> InternalSignRequest {
        InternalSignRequest {
            total_params,
            public_attributes,
            blind_sign_request,
        }
    }

    pub fn stage(key_pair: KeyPair, signed_deposits: sled::Db) -> AdHoc {
        let state = State::new(key_pair, signed_deposits);
        AdHoc::on_ignite("Internal Sign Request Stage", |rocket| async {
            rocket.manage(state).mount(
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
        request.blind_sign_request(),
        request.public_attributes(),
    )
    .unwrap()
}

#[post("/blind-sign", data = "<blind_sign_request_body>")]
//  Until we have serialization and deserialization traits we'll be using a crutch
pub async fn post_blind_sign(
    blind_sign_request_body: Json<BlindSignRequestBody>,
    state: &RocketState<State>,
) -> Result<Json<BlindedSignatureResponse>> {
    debug!("{:?}", blind_sign_request_body);
    let _encryption_key = extract_encryption_key(&blind_sign_request_body).await?;
    let internal_request = InternalSignRequest::new(
        *blind_sign_request_body.total_params(),
        blind_sign_request_body.public_attributes(),
        blind_sign_request_body.blind_sign_request().clone(),
    );
    let blinded_signature = blind_sign(internal_request, &state.key_pair);
    Ok(Json(BlindedSignatureResponse::new(blinded_signature)))
}

#[get("/verification-key")]
pub async fn get_verification_key(
    state: &RocketState<State>,
) -> Result<Json<VerificationKeyResponse>> {
    Ok(Json(VerificationKeyResponse::new(
        state.key_pair.verification_key(),
    )))
}
