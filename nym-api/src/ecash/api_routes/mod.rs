// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::api_routes::helpers::build_credentials_response;
use crate::ecash::error::{EcashError, Result};
use crate::ecash::helpers::blind_sign;
use crate::ecash::state::State;
use crate::ecash::storage::CoconutStorageExt;
use nym_api_requests::coconut::models::{
    BatchRedeemTicketsBody, EcashBatchTicketRedemptionResponse, EcashTicketVerificationRejection,
    EcashTicketVerificationResponse, SpentCredentialsResponse, VerifyEcashTicketBody,
};
use nym_api_requests::coconut::models::{
    CredentialsRequestBody, EpochCredentialsResponse, IssuedCredentialResponse,
    IssuedCredentialsResponse,
};
use nym_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, PartialCoinIndicesSignatureResponse,
    PartialExpirationDateSignatureResponse,
};
use nym_api_requests::constants::MIN_BATCH_REDEMPTION_DELAY;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::identify::IdentifyResult;
use nym_credentials::ecash::utils::{cred_exp_date, ecash_today};
use rocket::serde::json::Json;
use rocket::State as RocketState;
use std::collections::HashSet;
use std::ops::Deref;
use time::OffsetDateTime;

pub(crate) mod aggregation;
mod helpers;

#[get("/free-pass-nonce")]
pub async fn get_current_free_pass_nonce() -> Result<()> {
    debug!("Received free pass nonce request");

    Err(EcashError::DisabledFreePass)
}

#[post("/free-pass", data = "<freepass_request_body>")]
pub async fn post_free_pass(
    freepass_request_body: Json<serde_json::Value>,
) -> Result<Json<BlindedSignatureResponse>> {
    debug!("Received free pass sign request");

    let _ = freepass_request_body;
    Err(EcashError::DisabledFreePass)
}

#[post("/blind-sign", data = "<blind_sign_request_body>")]
pub async fn post_blind_sign(
    blind_sign_request_body: Json<BlindSignRequestBody>,
    state: &RocketState<State>,
) -> Result<Json<BlindedSignatureResponse>> {
    debug!("Received blind sign request");
    trace!("body: {:?}", blind_sign_request_body);

    // check if we have the signing key available
    debug!("checking if we actually have coconut keys derived...");
    let maybe_keypair_guard = state.ecash_keypair.get().await;
    let Some(keypair_guard) = maybe_keypair_guard.as_ref() else {
        return Err(EcashError::KeyPairNotDerivedYet);
    };
    let Some(signing_key) = keypair_guard.as_ref() else {
        return Err(EcashError::KeyPairNotDerivedYet);
    };

    // basic check of expiration date validity
    if blind_sign_request_body.expiration_date > cred_exp_date() {
        return Err(EcashError::ExpirationDateTooLate);
    }

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    // check if we already issued a credential for this deposit
    let deposit_id = blind_sign_request_body.deposit_id;
    debug!(
        "checking if we have already issued credential for this deposit (deposit_id: {deposit_id})",
    );
    if let Some(blinded_signature) = state.already_issued(deposit_id).await? {
        return Ok(Json(BlindedSignatureResponse { blinded_signature }));
    }

    //check if account was blacklisted
    let pub_key_bs58 = blind_sign_request_body.ecash_pubkey.to_base58_string();
    let blacklist_response = state
        .client
        .get_blacklisted_account(pub_key_bs58.clone())
        .await?;
    if blacklist_response.account.is_some() {
        return Err(EcashError::BlacklistedAccount);
    }

    // get the deposit details of the claimed id
    debug!("getting deposit details from the chain");
    let deposit = state.get_deposit(deposit_id).await?;

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

fn reject_ticket(
    reason: EcashTicketVerificationRejection,
) -> Result<Json<EcashTicketVerificationResponse>> {
    Ok(Json(EcashTicketVerificationResponse::reject(reason)))
}

// TODO: optimise it; for now it's just dummy split of the original `verify_offline_credential`
// introduce bloomfilter checks without touching storage first, etc.
#[post("/verify-ecash-ticket", data = "<verify_ticket_body>")]
pub async fn verify_ticket(
    // TODO in the future: make it send binary data rather than json
    verify_ticket_body: Json<VerifyEcashTicketBody>,
    state: &RocketState<State>,
) -> Result<Json<EcashTicketVerificationResponse>> {
    let credential_data = &verify_ticket_body.credential;
    let gateway_cosmos_addr = &verify_ticket_body.gateway_cosmos_addr;
    let sn = &credential_data.serial_number_b58();
    let spend_date = credential_data.spend_date;
    let epoch_id = credential_data.epoch_id;

    let verification_key = state.verification_key(epoch_id).await?;

    // SW NOTE: Offline scheme, but we still need some check on that, so that client and gateway can't collude and send expired credentials.
    // Let's allow the current day (obviously), and the day before (for late sender or around midnight)
    let today_date = ecash_today();

    // SAFETY: we're basing this on the current timestamp which, unless you invented a time machine,
    // is not smaller than the minimum time year
    #[allow(clippy::unwrap_used)]
    let yesterday_date = today_date.replace_date(today_date.date().previous_day().unwrap());

    if today_date != spend_date && yesterday_date != spend_date {
        return reject_ticket(EcashTicketVerificationRejection::InvalidSpentDate {
            today: today_date,
            yesterday: yesterday_date,
            received: spend_date,
        });
    }

    // TODO:
    // if state.check_bloomfilter(sn).await {
    //
    // }

    // actual double spend detection with storage
    if let Some(previous_payment) = state
        .get_ticket_data_by_serial_number(&credential_data.encoded_serial_number())
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
                return reject_ticket(EcashTicketVerificationRejection::ReplayedTicket);
            }
            IdentifyResult::DoubleSpendingPublicKeys(pub_key) => {
                //Actual double spending
                log::warn!(
                    "Double spending attempt for key {}",
                    pub_key.to_base58_string()
                );
                // todo!("blacklisting");
                return reject_ticket(EcashTicketVerificationRejection::DoubleSpend);
            }
        }
    }

    // perform actual crypto verification
    if credential_data.verify(&verification_key).is_err() {
        return reject_ticket(EcashTicketVerificationRejection::InvalidTicket);
    }

    //add to bloom filter for fast dup detection
    state.update_bloomfilter(sn).await;

    //store credential
    state
        .store_verified_ticket(credential_data, gateway_cosmos_addr)
        .await?;

    Ok(Json(EcashTicketVerificationResponse { verified: Ok(()) }))
}

// // for particular SN returns what gateway has submitted it and whether it has been verified correctly
// pub async fn credential_status() -> ! {
//     todo!()
// }

#[post(
    "/batch-redeem-ecash-tickets",
    data = "<batch_redeem_credentials_body>"
)]
pub async fn batch_redeem_tickets(
    // TODO in the future: make it send binary data rather than json
    batch_redeem_credentials_body: Json<BatchRedeemTicketsBody>,
    state: &RocketState<State>,
) -> Result<Json<EcashBatchTicketRedemptionResponse>> {
    // 1. see if that gateway has even submitted any tickets
    let Some(provider_info) = state
        .get_ticket_provider(batch_redeem_credentials_body.gateway_cosmos_addr.as_ref())
        .await?
    else {
        return Err(EcashError::NotTicketsProvided);
    };

    // 2. check if the gateway is not trying to spam the redemption requests
    // (we have to protect our poor chain)
    if let Some(last_redemption) = provider_info.last_batch_verification {
        let now = OffsetDateTime::now_utc();
        let next_allowed = last_redemption + MIN_BATCH_REDEMPTION_DELAY;

        if next_allowed > now {
            return Err(EcashError::TooFrequentRedemption {
                last_redemption,
                next_allowed,
            });
        }
    }

    // 3. verify the request digest
    if !batch_redeem_credentials_body.verify_digest() {
        return Err(EcashError::MismatchedRequestDigest);
    }

    // 4. verify the associated on-chain proposal (whether it's made by correct sender, has valid messages, etc.)
    state
        .validate_redemption_proposal(&batch_redeem_credentials_body)
        .await?;

    let proposal_id = batch_redeem_credentials_body.proposal_id;
    let received = batch_redeem_credentials_body
        .into_inner()
        .included_serial_numbers;

    // 5. check if **every** serial number included in the request has been verified by us
    // if we have more than requested, tough luck, they're going to lose them
    let verified = state.get_redeemable_tickets(provider_info).await?;
    let verified_tickets: HashSet<_> = verified.iter().map(|sn| sn.deref()).collect();

    for sn in &received {
        if !verified_tickets.contains(sn.deref()) {
            return Err(EcashError::TicketNotVerified {
                serial_number_bs58: bs58::encode(sn).into_string(),
            });
        }
    }

    // TODO: offload it to separate task with work queue and batching (of tx messages) to vote for multiple proposals in the same tx
    state.accept_proposal(proposal_id).await?;
    Ok(Json(EcashBatchTicketRedemptionResponse {
        proposal_accepted: true,
    }))
}

#[get("/spent-credentials-filter")]
pub async fn spent_credentials_filter(
    state: &RocketState<State>,
) -> Result<Json<SpentCredentialsResponse>> {
    let spent_credentials_export = state.export_bloomfilter().await;
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
        return Err(EcashError::InvalidQueryArguments);
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
    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

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
    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let expiration_date_signatures = state.get_exp_date_signatures_timestamp(timestamp).await?;
    Ok(Json(PartialExpirationDateSignatureResponse::new(
        &expiration_date_signatures,
    )))
}

#[get("/coin-indices-signatures")]
pub async fn coin_indices_signatures(
    state: &RocketState<State>,
) -> Result<Json<PartialCoinIndicesSignatureResponse>> {
    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let coin_indices_signatures = state.get_coin_indices_signatures().await?;
    Ok(Json(PartialCoinIndicesSignatureResponse::new(
        &coin_indices_signatures,
    )))
}
