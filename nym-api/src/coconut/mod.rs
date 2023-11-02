// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::comm::APICommunicationChannel;
use crate::coconut::client::Client as LocalClient;
use crate::coconut::deposit::extract_encryption_key;
use crate::coconut::error::{CoconutError, Result};
use crate::coconut::helpers::accepted_vote_err;
use crate::support::storage::NymApiStorage;
use getset::{CopyGetters, Getters};
use keypair::KeyPair;
use nym_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, EcashParametersResponse, VerifyCredentialBody,
    VerifyCredentialResponse,
};
use nym_coconut::VerificationKey;
use nym_coconut_bandwidth_contract_common::spend_credential::{
    funds_from_cosmos_msgs, SpendCredentialStatus,
};
use nym_coconut_dkg_common::types::EpochId;

use nym_compact_ecash::error::CompactEcashError;
use nym_compact_ecash::scheme::keygen::{KeyPairAuth, SecretKeyAuth};
use nym_compact_ecash::scheme::withdrawal::WithdrawalRequest;
use nym_compact_ecash::scheme::EcashCredential;
use nym_compact_ecash::setup::{setup, GroupParameters, Parameters};
use nym_compact_ecash::utils::BlindedSignature;
use nym_compact_ecash::{PublicKeyUser, VerificationKeyAuth};
use nym_config::defaults::NYM_API_VERSION;
use nym_credentials::coconut::params::{
    NymApiCredentialEncryptionAlgorithm, NymApiCredentialHkdfAlgorithm,
};
use nym_crypto::asymmetric::encryption;
use nym_crypto::shared_key::new_ephemeral_shared_key;
use nym_crypto::symmetric::stream_cipher;
use nym_validator_client::nym_api::routes::{BANDWIDTH, COCONUT_ROUTES};
use nym_validator_client::nyxd::{AccountId, Coin, Fee};
use rand_07::rngs::OsRng;
use rocket::fairing::AdHoc;
use rocket::serde::json::Json;
use rocket::State as RocketState;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) mod client;
pub(crate) mod comm;
mod deposit;
pub(crate) mod dkg;
pub(crate) mod error;
pub(crate) mod helpers;
pub(crate) mod keypair;
#[cfg(test)]
pub(crate) mod tests;

pub struct State {
    client: Arc<dyn LocalClient + Send + Sync>,
    mix_denom: String,
    ecash_params: Parameters,
    key_pair: KeyPair,
    comm_channel: Arc<dyn APICommunicationChannel + Send + Sync>,
    storage: NymApiStorage,
    rng: Arc<Mutex<OsRng>>,
}

impl State {
    pub(crate) fn new<C, D>(
        client: C,
        mix_denom: String,
        key_pair: KeyPair,
        comm_channel: D,
        storage: NymApiStorage,
    ) -> Self
    where
        C: LocalClient + Send + Sync + 'static,
        D: APICommunicationChannel + Send + Sync + 'static,
    {
        let client = Arc::new(client);
        let comm_channel = Arc::new(comm_channel);
        let rng = Arc::new(Mutex::new(OsRng));
        Self {
            client,
            mix_denom,
            ecash_params: setup(100),
            key_pair,
            comm_channel,
            storage,
            rng,
        }
    }

    pub async fn signed_before(&self, tx_hash: &str) -> Result<Option<BlindedSignatureResponse>> {
        let ret = self.storage.get_blinded_signature_response(tx_hash).await?;
        if let Some(blinded_signature_reponse) = ret {
            Ok(Some(BlindedSignatureResponse::from_base58_string(
                blinded_signature_reponse,
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
                NymApiCredentialEncryptionAlgorithm,
                NymApiCredentialHkdfAlgorithm,
                _,
            >(&mut rng, remote_key)
        };

        let chunk_data = signature.to_bytes();

        let zero_iv = stream_cipher::zero_iv::<NymApiCredentialEncryptionAlgorithm>();
        let encrypted_data = stream_cipher::encrypt::<NymApiCredentialEncryptionAlgorithm>(
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

    pub async fn verification_key(&self, epoch_id: EpochId) -> Result<VerificationKeyAuth> {
        self.comm_channel
            .aggregated_verification_key(epoch_id)
            .await
    }

    pub async fn store_credential(
        &self,
        credential: &EcashCredential,
        gateway_addr: &AccountId,
    ) -> Result<()> {
        self.storage
            .insert_credential(credential, gateway_addr)
            .await
            .map_err(|err| err.into())
    }
}

#[derive(Getters, CopyGetters, Debug)]
pub(crate) struct InternalSignRequest {
    #[getset(get)]
    withdrawal_request: WithdrawalRequest,
    #[getset(get)]
    ecash_pubkey: PublicKeyUser,
}

impl InternalSignRequest {
    pub fn new(
        withdrawal_request: WithdrawalRequest,
        ecash_pubkey: PublicKeyUser,
    ) -> InternalSignRequest {
        InternalSignRequest {
            withdrawal_request,
            ecash_pubkey,
        }
    }

    pub fn stage<C, D>(
        client: C,
        mix_denom: String,
        key_pair: KeyPair,
        comm_channel: D,
        storage: NymApiStorage,
    ) -> AdHoc
    where
        C: LocalClient + Send + Sync + 'static,
        D: APICommunicationChannel + Send + Sync + 'static,
    {
        let state = State::new(client, mix_denom, key_pair, comm_channel, storage);
        AdHoc::on_ignite("Internal Sign Request Stage", |rocket| async {
            rocket.manage(state).mount(
                // this format! is so ugly...
                format!("/{}/{}/{}", NYM_API_VERSION, COCONUT_ROUTES, BANDWIDTH),
                routes![
                    post_blind_sign,
                    verify_bandwidth_credential,
                    ecash_parameters
                ],
            )
        })
    }
}

fn blind_sign(request: InternalSignRequest, key_pair: KeyPairAuth) -> Result<BlindedSignature> {
    let params = GroupParameters::new()?;
    Ok(nym_compact_ecash::scheme::withdrawal::issue_wallet(
        &params,
        key_pair.secret_key(),
        request.ecash_pubkey().clone(),
        request.withdrawal_request(),
    )?)
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
        .get_tx(blind_sign_request_body.tx_hash())
        .await?;
    let encryption_key = extract_encryption_key(&blind_sign_request_body, tx).await?;
    let internal_request = InternalSignRequest::new(
        blind_sign_request_body.withdrawal_request().clone(),
        PublicKeyUser::from_base58_string(blind_sign_request_body.ecash_pubkey())?,
    );
    let blinded_signature = if let Some(keypair) = state.key_pair.get().await.as_ref() {
        let secret_key_auth = SecretKeyAuth::from_bytes(&keypair.secret_key().to_bytes())?;
        let verification_key_auth =
            VerificationKeyAuth::from_bytes(&keypair.verification_key().to_bytes())?;
        let index = keypair.index;

        let keypair_auth = KeyPairAuth::new(secret_key_auth, verification_key_auth, index);
        blind_sign(internal_request, keypair_auth)?
    } else {
        return Err(CoconutError::KeyPairNotDerivedYet);
    };

    let response = state
        .encrypt_and_store(
            blind_sign_request_body.tx_hash(),
            &encryption_key,
            &blinded_signature,
        )
        .await?;

    Ok(Json(response))
}

#[post("/verify-bandwidth-credential", data = "<verify_credential_body>")]
pub async fn verify_bandwidth_credential(
    verify_credential_body: Json<VerifyCredentialBody>,
    state: &RocketState<State>,
) -> Result<Json<VerifyCredentialResponse>> {
    let verification_key = state
        .verification_key(*verify_credential_body.credential().epoch_id())
        .await?;

    if let Err(_) = verify_credential_body.credential().payment().spend_verify(
        verify_credential_body.credential().params(),
        &verification_key,
        verify_credential_body.credential().pay_info(),
    ) {
        return Err(CoconutError::CompactEcashInternalError(
            CompactEcashError::PaymentVerification,
        ));
    }

    //store credential
    state
        .store_credential(
            verify_credential_body.credential(),
            verify_credential_body.gateway_cosmos_addr(),
        )
        .await?;

    Ok(Json(VerifyCredentialResponse::new(true)))
}

#[get("/ecash-parameters")]
pub async fn ecash_parameters(state: &RocketState<State>) -> Result<Json<EcashParametersResponse>> {
    Ok(Json(EcashParametersResponse::new(&state.ecash_params)))
}

// #[post("/verify-bandwidth-credential", data = "<verify_credential_body>")]
// pub async fn verify_bandwidth_credential(
//     verify_credential_body: Json<VerifyCredentialBody>,
//     state: &RocketState<State>,
// ) -> Result<Json<VerifyCredentialResponse>> {
//     let proposal_id = *verify_credential_body.proposal_id();
//     let proposal = state.client.get_proposal(proposal_id).await?;
//     // Proposal description is the blinded serial number
//     if !verify_credential_body
//         .credential()
//         .has_blinded_serial_number(&proposal.description)?
//     {
//         return Err(CoconutError::IncorrectProposal {
//             reason: String::from("incorrect blinded serial number in description"),
//         });
//     }
//     let proposed_release_funds =
//         funds_from_cosmos_msgs(proposal.msgs).ok_or(CoconutError::IncorrectProposal {
//             reason: String::from("action is not to release funds"),
//         })?;
//     // Credential has not been spent before, and is on its way of being spent
//     let credential_status = state
//         .client
//         .get_spent_credential(verify_credential_body.credential().blinded_serial_number())
//         .await?
//         .spend_credential
//         .ok_or(CoconutError::InvalidCredentialStatus {
//             status: String::from("Inexistent"),
//         })?
//         .status();
//     if credential_status != SpendCredentialStatus::InProgress {
//         return Err(CoconutError::InvalidCredentialStatus {
//             status: format!("{:?}", credential_status),
//         });
//     }
//     let verification_key = state
//         .verification_key(*verify_credential_body.credential().epoch_id())
//         .await?;

//     let verification_key_converted =
//         VerificationKey::try_from(verification_key.to_bytes().as_slice())
//             .expect("converstion should not fail"); //SW : TEMPORARY workaround for type conversion

//     let mut vote_yes = verify_credential_body
//         .credential()
//         .verify(&verification_key_converted);

//     vote_yes &= Coin::from(proposed_release_funds)
//         == Coin::new(
//             verify_credential_body.credential().voucher_value() as u128,
//             state.mix_denom.clone(),
//         );

//     // Vote yes or no on the proposal based on the verification result
//     let ret = state
//         .client
//         .vote_proposal(
//             proposal_id,
//             vote_yes,
//             Some(Fee::new_payer_granter_auto(
//                 None,
//                 None,
//                 Some(verify_credential_body.gateway_cosmos_addr().to_owned()),
//             )),
//         )
//         .await;
//     accepted_vote_err(ret)?;

//     Ok(Json(VerifyCredentialResponse::new(vote_yes)))
// }
