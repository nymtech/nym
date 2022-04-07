// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod client;
mod deposit;
pub(crate) mod error;
#[cfg(test)]
mod tests;

use crate::coconut::client::Client as LocalClient;
use crate::coconut::deposit::extract_encryption_key;
use crate::coconut::error::{CoconutError, Result};
use crate::ValidatorApiStorage;

use coconut_interface::{
    Attribute, BlindSignRequest, BlindSignRequestBody, BlindedSignature, BlindedSignatureResponse,
    KeyPair, Parameters, VerificationKeyResponse,
};
use config::defaults::VALIDATOR_API_VERSION;
use credentials::coconut::params::{
    ValidatorApiCredentialEncryptionAlgorithm, ValidatorApiCredentialHkdfAlgorithm,
};
use crypto::asymmetric::encryption;
use crypto::shared_key::new_ephemeral_shared_key;
use crypto::symmetric::stream_cipher;

use getset::{CopyGetters, Getters};
use rand_07::rngs::OsRng;
use rocket::fairing::AdHoc;
use rocket::serde::json::Json;
use rocket::State as RocketState;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use validator_client::validator_api::routes::{BANDWIDTH, COCONUT_ROUTES};

pub struct State {
    client: Arc<RwLock<dyn LocalClient + Send + Sync>>,
    key_pair: KeyPair,
    storage: ValidatorApiStorage,
    rng: Arc<Mutex<OsRng>>,
}

impl State {
    pub(crate) fn new<C>(client: C, key_pair: KeyPair, storage: ValidatorApiStorage) -> Self
    where
        C: LocalClient + Send + Sync + 'static,
    {
        let client = Arc::new(RwLock::new(client));
        let rng = Arc::new(Mutex::new(OsRng));
        Self {
            client,
            key_pair,
            storage,
            rng,
        }
    }

    pub async fn signed_before(&self, tx_hash: &str) -> Result<Option<BlindedSignatureResponse>> {
        let ret = self.storage.get_blinded_signature_response(tx_hash).await?;
        if let Some(blinded_signature_reponse) = ret {
            Ok(Some(BlindedSignatureResponse::from_base58_string(
                &blinded_signature_reponse,
            )?))
        } else {
            Ok(None)
        }
    }

    pub async fn encrypt_and_store(
        &self,
        tx_hash: &str,
        remote_key: &encryption::PublicKey,
        signature: &BlindedSignature,
    ) -> Result<BlindedSignatureResponse> {
        let (keypair, shared_key) = {
            let mut rng = *self.rng.lock().await;
            new_ephemeral_shared_key::<
                ValidatorApiCredentialEncryptionAlgorithm,
                ValidatorApiCredentialHkdfAlgorithm,
                _,
            >(&mut rng, remote_key)
        };

        let chunk_data = signature.to_bytes();

        let zero_iv = stream_cipher::zero_iv::<ValidatorApiCredentialEncryptionAlgorithm>();
        let encrypted_data = stream_cipher::encrypt::<ValidatorApiCredentialEncryptionAlgorithm>(
            &shared_key,
            &zero_iv,
            &chunk_data,
        );

        let response =
            BlindedSignatureResponse::new(encrypted_data, keypair.public_key().to_bytes());

        // Atomically insert data, only if there is no signature stored in the meantime
        // This prevents race conditions on storing two signatures for the same deposit transaction
        if self
            .storage
            .insert_blinded_signature_response(tx_hash, &response.to_base58_string())
            .await
            .is_err()
        {
            Ok(self
                .signed_before(tx_hash)
                .await?
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

    pub fn stage<C>(client: C, key_pair: KeyPair, storage: ValidatorApiStorage) -> AdHoc
    where
        C: LocalClient + Send + Sync + 'static,
    {
        let state = State::new(client, key_pair, storage);
        AdHoc::on_ignite("Internal Sign Request Stage", |rocket| async {
            rocket.manage(state).mount(
                // this format! is so ugly...
                format!(
                    "/{}/{}/{}",
                    VALIDATOR_API_VERSION, COCONUT_ROUTES, BANDWIDTH
                ),
                routes![
                    post_blind_sign,
                    get_verification_key,
                    post_partial_bandwidth_credential
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
    if let Some(response) = state
        .signed_before(blind_sign_request_body.tx_hash())
        .await?
    {
        return Ok(Json(response));
    }
    let tx = state
        .client
        .read()
        .await
        .get_tx(blind_sign_request_body.tx_hash())
        .await?;
    let encryption_key = extract_encryption_key(&blind_sign_request_body, tx).await?;
    let internal_request = InternalSignRequest::new(
        *blind_sign_request_body.total_params(),
        blind_sign_request_body.public_attributes(),
        blind_sign_request_body.blind_sign_request().clone(),
    );
    let blinded_signature = blind_sign(internal_request, &state.key_pair);

    let response = state
        .encrypt_and_store(
            blind_sign_request_body.tx_hash(),
            &encryption_key,
            &blinded_signature,
        )
        .await?;

    Ok(Json(response))
}

#[post("/partial-bandwidth-credential", data = "<tx_hash>")]
pub async fn post_partial_bandwidth_credential(
    tx_hash: Json<String>,
    state: &RocketState<State>,
) -> Result<Json<BlindedSignatureResponse>> {
    let v = state
        .signed_before(&tx_hash)
        .await?
        .ok_or(CoconutError::NoSignature)?;
    Ok(Json(v))
}

#[get("/verification-key")]
pub async fn get_verification_key(
    state: &RocketState<State>,
) -> Result<Json<VerificationKeyResponse>> {
    Ok(Json(VerificationKeyResponse::new(
        state.key_pair.verification_key(),
    )))
}
