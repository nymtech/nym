// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use crate::ecash::state::EcashState;
use crate::node_status_api::models::AxumResult;
use crate::v2::AxumAppState;
use axum::{Json, Router};
use nym_api_requests::constants::MIN_BATCH_REDEMPTION_DELAY;
use nym_api_requests::ecash::models::{
    BatchRedeemTicketsBody, EcashBatchTicketRedemptionResponse, EcashTicketVerificationRejection,
    EcashTicketVerificationResponse, SpentCredentialsResponse, VerifyEcashTicketBody,
};
use nym_compact_ecash::identify::IdentifyResult;
use nym_ecash_time::EcashTime;
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;
use time::macros::time;
use time::{OffsetDateTime, Time};

pub(crate) fn spending_routes(ecash_state: Arc<EcashState>) -> Router<AxumAppState> {
    Router::new()
        .route(
            "/verify-ecash-ticket",
            axum::routing::post({
                let ecash_state = Arc::clone(&ecash_state);
                |body| verify_ticket(body, ecash_state)
            }),
        )
        .route(
            "/batch-redeem-ecash-tickets",
            axum::routing::post({
                let ecash_state = Arc::clone(&ecash_state);
                |body| batch_redeem_tickets(body, ecash_state)
            }),
        )
        .route(
            "/double-spending-filter-v1",
            axum::routing::get({
                let ecash_state = Arc::clone(&ecash_state);
                || double_spending_filter_v1(ecash_state)
            }),
        )
}

const ONE_AM: Time = time!(1:00);

fn reject_ticket(
    reason: EcashTicketVerificationRejection,
) -> AxumResult<Json<EcashTicketVerificationResponse>> {
    Ok(Json(EcashTicketVerificationResponse::reject(reason)))
}

// TODO: optimise it; for now it's just dummy split of the original `verify_offline_credential`
// introduce bloomfilter checks without touching storage first, etc.
// #[openapi(tag = "Ecash")]
// #[post("/verify-ecash-ticket", data = "<verify_ticket_body>")]
async fn verify_ticket(
    // TODO in the future: make it send binary data rather than json
    Json(verify_ticket_body): Json<VerifyEcashTicketBody>,
    state: Arc<EcashState>,
) -> AxumResult<Json<EcashTicketVerificationResponse>> {
    let credential_data = &verify_ticket_body.credential;
    let gateway_cosmos_addr = &verify_ticket_body.gateway_cosmos_addr;

    // easy check: is there only a single payment attached?
    if credential_data.payment.spend_value != 1 {
        return reject_ticket(EcashTicketVerificationRejection::MultipleTickets);
    }

    let sn = &credential_data.encoded_serial_number();
    let spend_date = credential_data.spend_date;
    let epoch_id = credential_data.epoch_id;

    let now = OffsetDateTime::now_utc();
    let today_ecash = now.ecash_date();

    #[allow(clippy::unwrap_used)]
    let yesterday_ecash = today_ecash.previous_day().unwrap();

    // only accept yesterday date if we're near the day transition, i.e. before 1AM UTC
    if spend_date != today_ecash && now.time() > ONE_AM && spend_date != yesterday_ecash {
        return reject_ticket(EcashTicketVerificationRejection::InvalidSpentDate {
            today: today_ecash,
            yesterday: yesterday_ecash,
            received: spend_date,
        });
    }

    // check the bloomfilter for obvious double-spending so that we wouldn't need to waste time on crypto verification
    // TODO: when blacklisting is implemented, this should get removed
    if state.check_bloomfilter(sn).await {
        return reject_ticket(EcashTicketVerificationRejection::ReplayedTicket);
    }

    // actual double spend detection with storage
    if let Some(previous_payment) = state.get_ticket_data_by_serial_number(sn).await? {
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
                log::error!("UNIMPLEMENTED: blacklisting the double spend key");
                return reject_ticket(EcashTicketVerificationRejection::DoubleSpend);
            }
        }
    }

    let verification_key = state.master_verification_key(Some(epoch_id)).await?;

    // perform actual crypto verification
    if credential_data.verify(&verification_key).is_err() {
        return reject_ticket(EcashTicketVerificationRejection::InvalidTicket);
    }

    // finally get EXCLUSIVE lock on the bloomfilter, check if for the final time and insert the SN
    let was_present = state
        .update_bloomfilter(sn, spend_date, today_ecash)
        .await?;
    if was_present {
        return reject_ticket(EcashTicketVerificationRejection::ReplayedTicket);
    }

    //store credential
    state
        .store_verified_ticket(credential_data, gateway_cosmos_addr)
        .await?;

    Ok(Json(EcashTicketVerificationResponse { verified: Ok(()) }))
}

// // for particular SN returns what gateway has submitted it and whether it has been verified correctly
// async fn credential_status() -> ! {
//     todo!()
// }

// #[openapi(tag = "Ecash")]
// #[post(
//     "/batch-redeem-ecash-tickets",
//     data = "<batch_redeem_credentials_body>"
// )]
async fn batch_redeem_tickets(
    // TODO in the future: make it send binary data rather than json
    Json(batch_redeem_credentials_body): Json<BatchRedeemTicketsBody>,
    state: Arc<EcashState>,
) -> AxumResult<Json<EcashBatchTicketRedemptionResponse>> {
    // 1. see if that gateway has even submitted any tickets
    let Some(provider_info) = state
        .get_ticket_provider(batch_redeem_credentials_body.gateway_cosmos_addr.as_ref())
        .await?
    else {
        return Err(EcashError::NotTicketsProvided.into());
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
            }
            .into());
        }
    }

    // 3. verify the request digest
    if !batch_redeem_credentials_body.verify_digest() {
        return Err(EcashError::MismatchedRequestDigest.into());
    }

    // 4. verify the associated on-chain proposal (whether it's made by correct sender, has valid messages, etc.)
    state
        .validate_redemption_proposal(&batch_redeem_credentials_body)
        .await?;

    let proposal_id = batch_redeem_credentials_body.proposal_id;
    let received = batch_redeem_credentials_body.included_serial_numbers;

    // 5. check if **every** serial number included in the request has been verified by us
    // if we have more than requested, tough luck, they're going to lose them
    let verified = state.get_redeemable_tickets(provider_info).await?;
    let verified_tickets: HashSet<_> = verified.iter().map(|sn| sn.deref()).collect();

    for sn in &received {
        if !verified_tickets.contains(sn.deref()) {
            return Err(EcashError::TicketNotVerified {
                serial_number_bs58: bs58::encode(sn).into_string(),
            }
            .into());
        }
    }

    // TODO: offload it to separate task with work queue and batching (of tx messages) to vote for multiple proposals in the same tx
    state.accept_proposal(proposal_id).await?;
    Ok(Json(EcashBatchTicketRedemptionResponse {
        proposal_accepted: true,
    }))
}

// explicitly mark it as v1 in the URL because the response type WILL change;
// it will probably be compressed bincode or something
// #[openapi(tag = "Ecash")]
// #[get("/double-spending-filter-v1")]
async fn double_spending_filter_v1(
    state: Arc<EcashState>,
) -> AxumResult<Json<SpentCredentialsResponse>> {
    let spent_credentials_export = state.get_bloomfilter_bytes().await;
    Ok(Json(SpentCredentialsResponse::new(
        spent_credentials_export,
    )))
}
