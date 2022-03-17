// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod deposit;
mod error;

use crate::coconut::deposit::extract_encryption_key;
use crate::coconut::error::{CoconutError, Result};

use coconut_interface::{
    Attribute, BlindSignRequest, BlindSignRequestBody, BlindedSignature, BlindedSignatureResponse,
    KeyPair, Parameters, VerificationKeyResponse,
};
use config::defaults::VALIDATOR_API_VERSION;
use crypto::asymmetric::encryption;
use crypto::shared_key::new_ephemeral_shared_key;
use crypto::symmetric::stream_cipher;
use crypto::{aes::Aes128, blake3, ctr};

use getset::{CopyGetters, Getters};
use rand_07::rngs::OsRng;
use rocket::fairing::AdHoc;
use rocket::serde::json::Json;
use rocket::State as RocketState;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct State {
    key_pair: KeyPair,
    signed_deposits: sled::Db,
    rng: Arc<Mutex<OsRng>>,
}

impl State {
    pub fn new(key_pair: KeyPair, signed_deposits: sled::Db) -> Self {
        let rng = Arc::new(Mutex::new(OsRng));
        Self {
            key_pair,
            signed_deposits,
            rng,
        }
    }

    pub fn signed_before(&self, tx_hash: &[u8]) -> Result<Option<BlindedSignatureResponse>> {
        Ok(self
            .signed_deposits
            .get(tx_hash)?
            .map(|b| BlindedSignatureResponse::from_bytes(b.to_vec())))
    }

    pub async fn encrypt_and_store(
        &self,
        tx_hash: &[u8],
        remote_key: &encryption::PublicKey,
        signature: &BlindedSignature,
    ) -> Result<BlindedSignatureResponse> {
        let (keypair, shared_key) = {
            let mut rng = *self.rng.lock().await;
            new_ephemeral_shared_key::<ctr::Ctr64LE<Aes128>, blake3::Hasher, _>(
                &mut rng, remote_key,
            )
        };

        let chunk_data = signature.to_bytes();

        let zero_iv = stream_cipher::zero_iv::<ctr::Ctr64LE<Aes128>>();
        let encrypted_data =
            stream_cipher::encrypt::<ctr::Ctr64LE<Aes128>>(&shared_key, &zero_iv, &chunk_data);

        let response =
            BlindedSignatureResponse::new(encrypted_data.clone(), keypair.public_key().to_bytes());

        // Atomically insert data, only if there is no signature stored in the meantime
        // This prevents race conditions on storing two signatures for the same deposit transaction
        if self
            .signed_deposits
            .compare_and_swap(tx_hash, None as Option<&[u8]>, Some(response.to_bytes()))?
            .is_err()
        {
            Ok(self
                .signed_before(tx_hash)?
                .expect("The signature was expected to be there"))
        } else {
            Ok(response)
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
                routes![post_blind_sign, get_verification_key, get_signature],
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
    if let Some(response) = state.signed_before(blind_sign_request_body.tx_hash().as_bytes())? {
        return Ok(Json(response));
    }
    let encryption_key = extract_encryption_key(&blind_sign_request_body).await?;
    let internal_request = InternalSignRequest::new(
        *blind_sign_request_body.total_params(),
        blind_sign_request_body.public_attributes(),
        blind_sign_request_body.blind_sign_request().clone(),
    );
    let blinded_signature = blind_sign(internal_request, &state.key_pair);

    let response = state
        .encrypt_and_store(
            blind_sign_request_body.tx_hash().as_bytes(),
            &encryption_key,
            &blinded_signature,
        )
        .await?;

    Ok(Json(response))
}

#[get("/signature", data = "<tx_hash>")]
pub async fn get_signature(
    tx_hash: String,
    state: &RocketState<State>,
) -> Result<Json<BlindedSignatureResponse>> {
    Ok(Json(
        state
            .signed_before(tx_hash.as_bytes())?
            .ok_or(CoconutError::NoSignature)?,
    ))
}

#[get("/verification-key")]
pub async fn get_verification_key(
    state: &RocketState<State>,
) -> Result<Json<VerificationKeyResponse>> {
    Ok(Json(VerificationKeyResponse::new(
        state.key_pair.verification_key(),
    )))
}
