// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::InternalSignRequest;
use crate::coconut::deposit::extract_encryption_key;
use crate::coconut::error::{CoconutError, Result};
use crate::coconut::helpers::{accepted_vote_err, blind_sign};
use crate::coconut::state::State;
use nym_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, VerifyCredentialBody, VerifyCredentialResponse,
};
use nym_coconut_bandwidth_contract_common::spend_credential::{
    funds_from_cosmos_msgs, SpendCredentialStatus,
};
use nym_validator_client::nyxd::{Coin, Fee};
use rocket::serde::json::Json;
use rocket::State as RocketState;

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
        *blind_sign_request_body.total_params(),
        blind_sign_request_body.public_attributes(),
        blind_sign_request_body.blind_sign_request().clone(),
    );
    let blinded_signature = if let Some(keypair) = state.key_pair.get().await.as_ref() {
        blind_sign(internal_request, keypair)?
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
    let proposal_id = *verify_credential_body.proposal_id();
    let proposal = state.client.get_proposal(proposal_id).await?;
    // Proposal description is the blinded serial number
    if !verify_credential_body
        .credential()
        .has_blinded_serial_number(&proposal.description)?
    {
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
        .get_spent_credential(verify_credential_body.credential().blinded_serial_number())
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
    let verification_key = state
        .verification_key(*verify_credential_body.credential().epoch_id())
        .await?;
    let mut vote_yes = verify_credential_body
        .credential()
        .verify(&verification_key);

    vote_yes &= Coin::from(proposed_release_funds)
        == Coin::new(
            verify_credential_body.credential().voucher_value() as u128,
            state.mix_denom.clone(),
        );

    // Vote yes or no on the proposal based on the verification result
    let ret = state
        .client
        .vote_proposal(
            proposal_id,
            vote_yes,
            Some(Fee::new_payer_granter_auto(
                None,
                None,
                Some(verify_credential_body.gateway_cosmos_addr().to_owned()),
            )),
        )
        .await;
    accepted_vote_err(ret)?;

    Ok(Json(VerifyCredentialResponse::new(vote_yes)))
}
