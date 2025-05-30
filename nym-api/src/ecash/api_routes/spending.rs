// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use crate::ecash::state::EcashState;
use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::{Json, Router};
use nym_api_requests::constants::MIN_BATCH_REDEMPTION_DELAY;
use nym_api_requests::ecash::models::{
    BatchRedeemTicketsBody, EcashBatchTicketRedemptionResponse, EcashTicketVerificationRejection,
    EcashTicketVerificationResponse, SpentCredentialsResponse, VerifyEcashTicketBody,
};
use nym_compact_ecash::identify::IdentifyResult;
use nym_ecash_time::EcashTime;
use nym_http_api_common::{FormattedResponse, Output, OutputParams};
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;
use time::macros::time;
use time::{OffsetDateTime, Time};
use tracing::{error, warn};

#[allow(deprecated)]
pub(crate) fn spending_routes() -> Router<AppState> {
    Router::new()
        .route("/verify-ecash-ticket", axum::routing::post(verify_ticket))
        .route(
            "/batch-redeem-ecash-tickets",
            axum::routing::post(batch_redeem_tickets),
        )
        .route(
            "/double-spending-filter-v1",
            axum::routing::get(double_spending_filter_v1),
        )
}

const ONE_AM: Time = time!(1:00);

fn reject_ticket(
    output: Output,
    reason: EcashTicketVerificationRejection,
) -> AxumResult<FormattedResponse<EcashTicketVerificationResponse>> {
    Ok(output.to_response(EcashTicketVerificationResponse::reject(reason)))
}

// TODO: optimise it; for now it's just dummy split of the original `verify_offline_credential`
// introduce bloomfilter checks without touching storage first, etc.
#[utoipa::path(
    tag = "Ecash",
    post,
    request_body = VerifyEcashTicketBody,
    path = "/v1/ecash/verify-ecash-ticket",
    responses(
        (status = 200, content(
            (EcashTicketVerificationResponse = "application/json"),
            (EcashTicketVerificationResponse = "application/yaml"),
            (EcashTicketVerificationResponse = "application/bincode")
        )),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the current epoch"),
    )
)]
async fn verify_ticket(
    Query(output): Query<OutputParams>,
    State(state): State<Arc<EcashState>>,
    // TODO in the future: make it send binary data rather than json
    Json(verify_ticket_body): Json<VerifyEcashTicketBody>,
) -> AxumResult<FormattedResponse<EcashTicketVerificationResponse>> {
    state.ensure_signer().await?;
    let output = output.output.unwrap_or_default();

    let credential_data = &verify_ticket_body.credential;
    let gateway_cosmos_addr = &verify_ticket_body.gateway_cosmos_addr;

    // easy check: is there only a single payment attached?
    if credential_data.payment.spend_value != 1 {
        return reject_ticket(output, EcashTicketVerificationRejection::MultipleTickets);
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
        return reject_ticket(
            output,
            EcashTicketVerificationRejection::InvalidSpentDate {
                today: today_ecash,
                yesterday: yesterday_ecash,
                received: spend_date,
            },
        );
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
                warn!("Identical payInfo");
                return reject_ticket(output, EcashTicketVerificationRejection::ReplayedTicket);
            }
            IdentifyResult::DoubleSpendingPublicKeys(pub_key) => {
                //Actual double spending
                warn!(
                    "Double spending attempt for key {}",
                    pub_key.to_base58_string()
                );
                error!("UNIMPLEMENTED: blacklisting the double spend key");
                return reject_ticket(output, EcashTicketVerificationRejection::DoubleSpend);
            }
        }
    }

    let verification_key = state.master_verification_key(Some(epoch_id)).await?;

    // perform actual crypto verification
    if credential_data.verify(&verification_key).is_err() {
        return reject_ticket(output, EcashTicketVerificationRejection::InvalidTicket);
    }

    // store credential and check whether it wasn't already there (due to a parallel request)
    let was_inserted = state
        .store_verified_ticket(credential_data, gateway_cosmos_addr)
        .await?;
    if !was_inserted {
        return reject_ticket(output, EcashTicketVerificationRejection::ReplayedTicket);
    }

    Ok(output.to_response(EcashTicketVerificationResponse { verified: Ok(()) }))
}

#[utoipa::path(
    tag = "Ecash",
    post,
    request_body = BatchRedeemTicketsBody,
    path = "/v1/ecash/batch-redeem-ecash-tickets",
    responses(
        (status = 200, content(
            (EcashBatchTicketRedemptionResponse = "application/json"),
            (EcashBatchTicketRedemptionResponse = "application/yaml"),
            (EcashBatchTicketRedemptionResponse = "application/bincode")
        )),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the current epoch"),
    ),
    params(OutputParams)
)]
async fn batch_redeem_tickets(
    Query(output): Query<OutputParams>,
    State(state): State<Arc<EcashState>>,
    // TODO in the future: make it send binary data rather than json
    Json(batch_redeem_credentials_body): Json<BatchRedeemTicketsBody>,
) -> AxumResult<FormattedResponse<EcashBatchTicketRedemptionResponse>> {
    state.ensure_signer().await?;
    let output = output.output.unwrap_or_default();

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
    let verified = state.get_redeemable_tickets(&provider_info).await?;
    let verified_tickets: HashSet<_> = verified.iter().map(|sn| sn.deref()).collect();

    for sn in &received {
        if !verified_tickets.contains(sn.deref()) {
            return Err(EcashError::TicketNotVerified {
                serial_number_bs58: bs58::encode(sn).into_string(),
            }
            .into());
        }
    }

    // 6. vote on the proposal
    // TODO: offload it to separate task with work queue and batching (of tx messages) to vote for multiple proposals in the same tx
    // similarly to what we do inside the credential proxy
    state.accept_proposal(proposal_id).await?;

    // 7. update the time of the last verification for this provider
    state.update_last_batch_verification(&provider_info).await?;

    Ok(output.to_response(EcashBatchTicketRedemptionResponse {
        proposal_accepted: true,
    }))
}

// explicitly mark it as v1 in the URL because the response type WILL change;
// it will probably be compressed bincode or something
#[utoipa::path(
    tag = "Ecash",
    get,
    path = "/v1/ecash/double-spending-filter-v1",
    responses(
        (status = 500, body = String, description = "bloomfilters got disabled"),
    )
)]
#[deprecated]
async fn double_spending_filter_v1() -> AxumResult<FormattedResponse<SpentCredentialsResponse>> {
    AxumResult::Err(AxumErrorResponse::new(
        "permanently restricted",
        StatusCode::GONE,
    ))
}
