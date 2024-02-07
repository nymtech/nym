// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::api_routes::helpers::build_credentials_response;
use crate::coconut::error::{CoconutError, Result};
use crate::coconut::helpers::{accepted_vote_err, blind_sign};
use crate::coconut::state::State;
use crate::coconut::storage::CoconutStorageExt;
use nym_api_requests::coconut::models::{
    CredentialsRequestBody, EpochCredentialsResponse, IssuedCredentialResponse,
    IssuedCredentialsResponse,
};
use nym_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, VerifyCredentialBody, VerifyCredentialResponse,
};
use nym_coconut_bandwidth_contract_common::spend_credential::{
    funds_from_cosmos_msgs, SpendCredentialStatus,
};
use nym_coconut_dkg_common::types::EpochId;
use nym_credentials::coconut::bandwidth::{
    bandwidth_credential_params, IssuanceBandwidthCredential,
};
use nym_validator_client::nyxd::Coin;
use rocket::serde::json::Json;
use rocket::State as RocketState;

mod helpers;

#[post("/blind-sign", data = "<blind_sign_request_body>")]
//  Until we have serialization and deserialization traits we'll be using a crutch
pub async fn post_blind_sign(
    blind_sign_request_body: Json<BlindSignRequestBody>,
    state: &RocketState<State>,
) -> Result<Json<BlindedSignatureResponse>> {
    debug!("Received blind sign request");
    trace!("body: {:?}", blind_sign_request_body);

    // early check: does the request have the expected number of public attributes?
    debug!("performing basic request validation");
    if blind_sign_request_body.public_attributes_plain.len()
        != IssuanceBandwidthCredential::PUBLIC_ATTRIBUTES as usize
    {
        return Err(CoconutError::InconsistentPublicAttributes);
    }

    // check if we already issued a credential for this tx hash
    debug!(
        "checking if we have already issued credential for this tx_hash (hash: {})",
        blind_sign_request_body.tx_hash
    );
    if let Some(blinded_signature) = state
        .already_issued(blind_sign_request_body.tx_hash)
        .await?
    {
        return Ok(Json(BlindedSignatureResponse { blinded_signature }));
    }

    // check if we have the signing key available
    debug!("checking if we actually have coconut keys derived...");
    let maybe_keypair_guard = state.coconut_keypair.get().await;
    let Some(keypair_guard) = maybe_keypair_guard.as_ref() else {
        return Err(CoconutError::KeyPairNotDerivedYet);
    };
    let Some(signing_key) = keypair_guard.as_ref() else {
        return Err(CoconutError::KeyPairNotDerivedYet);
    };

    // get the transaction details of the claimed deposit
    debug!("getting transaction details from the chain");
    let tx = state
        .get_transaction(blind_sign_request_body.tx_hash)
        .await?;

    // check validity of the request
    debug!("fully validating received request");
    state.validate_request(&blind_sign_request_body, tx).await?;

    // produce the partial signature
    debug!("producing the partial credential");
    let blinded_signature = blind_sign(&blind_sign_request_body, signing_key.keys.secret_key())?;

    // store the information locally
    debug!("storing the issued credential in the database");
    state
        .store_issued_credential(blind_sign_request_body.into_inner(), &blinded_signature)
        .await?;

    // finally return the credential to the client
    Ok(Json(BlindedSignatureResponse { blinded_signature }))
}

#[post("/verify-bandwidth-credential", data = "<verify_credential_body>")]
pub async fn verify_bandwidth_credential(
    verify_credential_body: Json<VerifyCredentialBody>,
    state: &RocketState<State>,
) -> Result<Json<VerifyCredentialResponse>> {
    let proposal_id = verify_credential_body.proposal_id;
    let epoch_id = verify_credential_body.epoch_id;
    let credential_data = &verify_credential_body.credential_data;
    let theta = &credential_data.verify_credential_request;

    let voucher_value: u64 = if credential_data.typ.is_voucher() {
        credential_data
            .get_bandwidth_attribute()
            .ok_or(CoconutError::MissingBandwidthValue)?
            .parse()
            .map_err(|source| CoconutError::VoucherValueParsingFailure { source })?
    } else {
        return Err(CoconutError::NotABandwidthVoucher {
            typ: credential_data.typ,
        });
    };

    // TODO: introduce a check to make sure we haven't already voted for this proposal to prevent DDOS

    let proposal = state.client.get_proposal(proposal_id).await?;

    // Proposal description is the blinded serial number
    if !theta.has_blinded_serial_number(&proposal.description)? {
        return Err(CoconutError::IncorrectProposal {
            reason: String::from("incorrect blinded serial number in description"),
        });
    }
    let proposed_release_funds =
        funds_from_cosmos_msgs(proposal.msgs).ok_or(CoconutError::IncorrectProposal {
            reason: String::from("action is not to release funds"),
        })?;
    // Credential has not been spent before, and is on its way of being spent
    let credential_status = state
        .client
        .get_spent_credential(theta.blinded_serial_number_bs58())
        .await?
        .spend_credential
        .ok_or(CoconutError::InvalidCredentialStatus {
            status: String::from("Inexistent"),
        })?
        .status();
    if credential_status != SpendCredentialStatus::InProgress {
        return Err(CoconutError::InvalidCredentialStatus {
            status: format!("{:?}", credential_status),
        });
    }
    let verification_key = state.verification_key(epoch_id).await?;
    let params = bandwidth_credential_params();
    let mut vote_yes = credential_data.verify(params, &verification_key);

    vote_yes &= Coin::from(proposed_release_funds)
        == Coin::new(voucher_value as u128, state.mix_denom.clone());

    // Vote yes or no on the proposal based on the verification result
    let ret = state
        .client
        .vote_proposal(proposal_id, vote_yes, None)
        .await;
    accepted_vote_err(ret)?;

    Ok(Json(VerifyCredentialResponse::new(vote_yes)))
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
