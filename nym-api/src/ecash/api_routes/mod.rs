// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::api_routes::helpers::build_credentials_response;
use crate::ecash::error::{CoconutError, Result};
use crate::ecash::helpers::{accepted_vote_err, blind_sign};
use crate::ecash::state::State;
use crate::ecash::storage::CoconutStorageExt;
use nym_api_requests::coconut::models::SpentCredentialsResponse;
use nym_api_requests::coconut::models::{
    CredentialsRequestBody, EpochCredentialsResponse, IssuedCredentialResponse,
    IssuedCredentialsResponse, VerifyEcashCredentialResponse,
};
use nym_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, PartialCoinIndicesSignatureResponse,
    PartialExpirationDateSignatureResponse, VerifyEcashCredentialBody,
};
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::identify::IdentifyResult;
use nym_credentials::coconut::utils::{cred_exp_date, today};
use nym_ecash_contract_common::spend_credential::check_proposal;
use rocket::serde::json::Json;
use rocket::State as RocketState;
use std::ops::Deref;
use time::Duration;

mod helpers;

#[get("/free-pass-nonce")]
pub async fn get_current_free_pass_nonce() -> Result<()> {
    debug!("Received free pass nonce request");

    Err(CoconutError::DisabledFreePass)
}

#[post("/free-pass", data = "<freepass_request_body>")]
pub async fn post_free_pass(
    freepass_request_body: Json<serde_json::Value>,
) -> Result<Json<BlindedSignatureResponse>> {
    debug!("Received free pass sign request");

    let _ = freepass_request_body;
    Err(CoconutError::DisabledFreePass)
}

#[post("/blind-sign", data = "<blind_sign_request_body>")]
//  Until we have serialization and deserialization traits we'll be using a crutch
pub async fn post_blind_sign(
    blind_sign_request_body: Json<BlindSignRequestBody>,
    state: &RocketState<State>,
) -> Result<Json<BlindedSignatureResponse>> {
    debug!("Received blind sign request");
    trace!("body: {:?}", blind_sign_request_body);

    // check if we already issued a credential for this tx hash
    debug!(
        "checking if we have already issued credential for this deposit (deposit_id: {})",
        blind_sign_request_body.deposit_id
    );
    if let Some(blinded_signature) = state
        .already_issued(blind_sign_request_body.deposit_id)
        .await?
    {
        return Ok(Json(BlindedSignatureResponse { blinded_signature }));
    }

    // check if we have the signing key available
    debug!("checking if we actually have coconut keys derived...");
    let maybe_keypair_guard = state.ecash_keypair.get().await;
    let Some(keypair_guard) = maybe_keypair_guard.as_ref() else {
        return Err(CoconutError::KeyPairNotDerivedYet);
    };
    let Some(signing_key) = keypair_guard.as_ref() else {
        return Err(CoconutError::KeyPairNotDerivedYet);
    };

    //check if account was blacklisted
    let pub_key_bs58 = blind_sign_request_body.ecash_pubkey.to_base58_string();
    let blacklist_response = state
        .client
        .get_blacklisted_account(pub_key_bs58.clone())
        .await?;
    if blacklist_response.account.is_some() {
        return Err(CoconutError::BlacklistedAccount);
    }

    // get the deposit details of the claimed id
    debug!("getting deposit details from the chain");
    let deposit = state
        .get_deposit(blind_sign_request_body.deposit_id)
        .await?;

    //check expiration date validity
    if blind_sign_request_body.expiration_date > cred_exp_date() {
        return Err(CoconutError::ExpirationDateTooLate);
    }

    // check validity of the request
    debug!("fully validating received request");
    state
        .validate_request(&blind_sign_request_body, deposit)
        .await?;

    // produce the partial signature
    debug!("producing the partial credential");
    let blinded_signature = blind_sign(
        blind_sign_request_body.deref(),
        signing_key.keys.secret_key(),
    )?;

    // store the information locally
    debug!("storing the issued credential in the database");
    state
        .store_issued_credential(blind_sign_request_body.into_inner(), &blinded_signature)
        .await?;

    // finally return the credential to the client
    Ok(Json(BlindedSignatureResponse { blinded_signature }))
}

#[post("/verify-online-credential", data = "<verify_credential_body>")]
pub async fn verify_online_credential(
    verify_credential_body: Json<VerifyEcashCredentialBody>,
    state: &RocketState<State>,
) -> Result<Json<VerifyEcashCredentialResponse>> {
    let proposal_id = verify_credential_body.proposal_id;
    let credential_data = &verify_credential_body.credential;
    let payment = &credential_data.payment;
    let today_date = today();

    if today_date != credential_data.spend_date {
        state.refuse_proposal_online(proposal_id).await;
        return Ok(Json(VerifyEcashCredentialResponse::SubmittedTooLate {
            expected_until: today_date,
            actual: credential_data.spend_date,
        }));
    }

    //actual double spend detection with storage
    if let Some(previous_payment) = state
        .get_credential_by_sn(credential_data.serial_number_b58())
        .await?
    {
        match nym_compact_ecash::identify::identify(
            &credential_data.payment,
            &previous_payment.payment,
            credential_data.pay_info,
            previous_payment.pay_info,
        ) {
            IdentifyResult::NotADuplicatePayment => {} //SW NOTE This should never happen, quick message?
            IdentifyResult::DuplicatePayInfo(_) => {
                log::warn!("Identical payInfo");
                state.refuse_proposal_online(proposal_id).await;
                return Ok(Json(VerifyEcashCredentialResponse::AlreadySent));
            }
            IdentifyResult::DoubleSpendingPublicKeys(pub_key) => {
                //Actual double spending
                log::warn!(
                    "Double spending attempt for key {}",
                    pub_key.to_base58_string()
                );
                state.refuse_proposal_online(proposal_id).await;
                state.blacklist(pub_key.to_base58_string()).await;

                return Ok(Json(VerifyEcashCredentialResponse::DoubleSpend));
            }
        }
    }
    //Double spend check with contract
    if let Some(spent_credential) = state
        .client
        .get_spent_credential(payment.serial_number_bs58())
        .await?
        .spend_credential
    {
        if spent_credential.serial_number() == credential_data.serial_number_b58() {
            state.refuse_proposal_online(proposal_id).await;
            return Ok(Json(VerifyEcashCredentialResponse::DoubleSpend));
        }
    }

    let verification_key = state.verification_key(credential_data.epoch_id).await?;

    if credential_data.verify(&verification_key).is_err() {
        state.refuse_proposal_online(proposal_id).await;
        return Ok(Json(VerifyEcashCredentialResponse::Refused));
    }

    let proposal = state.client.get_proposal(proposal_id).await?;

    // Proposal description is the blinded serial number
    if !payment.has_serial_number(&proposal.description)? {
        state.client.vote_proposal(proposal_id, false, None).await?;
        return Ok(Json(VerifyEcashCredentialResponse::InvalidFormat(
            String::from("incorrect blinded serial number in description"),
        )));
    }
    if !check_proposal(proposal.msgs) {
        state.client.vote_proposal(proposal_id, false, None).await?;
        return Ok(Json(VerifyEcashCredentialResponse::InvalidFormat(
            String::from("action is not to spend_credential"),
        )));
    }

    // Vote yes or no on the proposal based on the verification result
    let ret = state.client.vote_proposal(proposal_id, true, None).await;
    accepted_vote_err(ret)?;

    //From here, credential is considered spent

    //add to bloom filter for fast dup detection
    state
        .add_spent_credentials(&credential_data.serial_number_b58())
        .await;

    //store credential
    state
        .store_credential(
            &verify_credential_body.credential,
            &verify_credential_body.gateway_cosmos_addr,
            proposal_id,
        )
        .await?;

    Ok(Json(VerifyEcashCredentialResponse::Accepted))
}

#[post("/verify-offline-credential", data = "<verify_credential_body>")]
pub async fn verify_offline_credential(
    verify_credential_body: Json<VerifyEcashCredentialBody>,
    state: &RocketState<State>,
) -> Result<Json<VerifyEcashCredentialResponse>> {
    let credential_data = &verify_credential_body.credential;
    let proposal_id = verify_credential_body.proposal_id;

    //SW NOTE: Offline scheme, but we still need some check on that, so that client and gateway can't collude and send expired credentials.
    //Let's allow the current day (obviously), and the day before (for late sender or around midnight)
    let today_date = today();
    let yesterday_date = today_date - Duration::DAY;
    if today_date != credential_data.spend_date && yesterday_date != credential_data.spend_date {
        state.refuse_proposal(proposal_id).await;
        return Ok(Json(VerifyEcashCredentialResponse::SubmittedTooLate {
            expected_until: yesterday_date,
            actual: credential_data.spend_date,
        }));
    }

    //actual double spend detection with storage
    if let Some(previous_payment) = state
        .get_credential_by_sn(credential_data.serial_number_b58())
        .await?
    {
        match nym_compact_ecash::identify::identify(
            &credential_data.payment,
            &previous_payment.payment,
            credential_data.pay_info,
            previous_payment.pay_info,
        ) {
            IdentifyResult::NotADuplicatePayment => {} //SW NOTE This should never happen, quick message?
            IdentifyResult::DuplicatePayInfo(_) => {
                log::warn!("Identical payInfo");
                state.refuse_proposal(proposal_id).await;
                return Ok(Json(VerifyEcashCredentialResponse::AlreadySent));
            }
            IdentifyResult::DoubleSpendingPublicKeys(pub_key) => {
                //Actual double spending
                log::warn!(
                    "Double spending attempt for key {}",
                    pub_key.to_base58_string()
                );
                state.refuse_proposal(proposal_id).await;
                state.blacklist(pub_key.to_base58_string()).await;

                return Ok(Json(VerifyEcashCredentialResponse::DoubleSpend));
            }
        }
    }

    let epoch_id = credential_data.epoch_id;
    let verification_key = state.verification_key(epoch_id).await?;

    if credential_data.verify(&verification_key).is_err() {
        state.refuse_proposal(proposal_id).await;
        return Ok(Json(VerifyEcashCredentialResponse::Refused));
    }

    //add to bloom filter for fast dup detection
    state
        .add_spent_credentials(&credential_data.serial_number_b58())
        .await;

    //store credential
    state
        .store_credential(
            &verify_credential_body.credential,
            &verify_credential_body.gateway_cosmos_addr,
            proposal_id,
        )
        .await?;

    state
        .accept_and_execute_proposal(proposal_id, credential_data.serial_number_b58())
        .await?;

    Ok(Json(VerifyEcashCredentialResponse::Accepted))
}

#[get("/spent-credentials-filter")]
pub async fn spent_credentials_filter(
    state: &RocketState<State>,
) -> Result<Json<SpentCredentialsResponse>> {
    let spent_credentials_export = state.export_spent_credentials().await;
    Ok(Json(SpentCredentialsResponse::new(
        spent_credentials_export,
    )))
}

#[get("/epoch-credentials/<epoch>")]
pub async fn epoch_credentials(
    epoch: EpochId,
    state: &RocketState<State>,
) -> Result<Json<EpochCredentialsResponse>> {
    let issued = state.storage.get_epoch_credentials(epoch).await?;

    let response = if let Some(issued) = issued {
        issued.into()
    } else {
        EpochCredentialsResponse {
            epoch_id: epoch,
            first_epoch_credential_id: None,
            total_issued: 0,
        }
    };

    Ok(Json(response))
}

#[get("/issued-credential/<id>")]
pub async fn issued_credential(
    id: i64,
    state: &RocketState<State>,
) -> Result<Json<IssuedCredentialResponse>> {
    let issued = state.storage.get_issued_credential(id).await?;

    let credential = if let Some(issued) = issued {
        Some(issued.try_into()?)
    } else {
        None
    };

    Ok(Json(IssuedCredentialResponse { credential }))
}

#[post("/issued-credentials", data = "<params>")]
pub async fn issued_credentials(
    params: Json<CredentialsRequestBody>,
    state: &RocketState<State>,
) -> Result<Json<IssuedCredentialsResponse>> {
    let params = params.into_inner();

    if params.pagination.is_some() && !params.credential_ids.is_empty() {
        return Err(CoconutError::InvalidQueryArguments);
    }

    let credentials = if let Some(pagination) = params.pagination {
        state
            .storage
            .get_issued_credentials_paged(pagination)
            .await?
    } else {
        state
            .storage
            .get_issued_credentials(params.credential_ids)
            .await?
    };

    build_credentials_response(credentials).map(Json)
}

#[get("/expiration-date-signatures")]
pub async fn expiration_date_signatures(
    state: &RocketState<State>,
) -> Result<Json<PartialExpirationDateSignatureResponse>> {
    let expiration_date_signatures = state.get_exp_date_signatures().await?;

    Ok(Json(PartialExpirationDateSignatureResponse::new(
        &expiration_date_signatures,
    )))
}

#[get("/expiration-date-signatures/<timestamp>")]
pub async fn expiration_date_signatures_timestamp(
    timestamp: u64,
    state: &RocketState<State>,
) -> Result<Json<PartialExpirationDateSignatureResponse>> {
    let expiration_date_signatures = state.get_exp_date_signatures_timestamp(timestamp).await?;
    Ok(Json(PartialExpirationDateSignatureResponse::new(
        &expiration_date_signatures,
    )))
}

#[get("/coin-indices-signatures")]
pub async fn coin_indices_signatures(
    state: &RocketState<State>,
) -> Result<Json<PartialCoinIndicesSignatureResponse>> {
    let coin_indices_signatures = state.get_coin_indices_signatures().await?;
    Ok(Json(PartialCoinIndicesSignatureResponse::new(
        &coin_indices_signatures,
    )))
}
