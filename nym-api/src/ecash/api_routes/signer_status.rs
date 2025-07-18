// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::state::EcashState;
use crate::node_status_api::models::ApiResult;
use axum::extract::{Query, State};
use nym_api_requests::ecash::models::{EcashSignerStatusResponse, EcashSignerStatusResponseBody};
use nym_api_requests::signable::SignableMessageBody;
use nym_http_api_common::{FormattedResponse, OutputParams};
use std::sync::Arc;
use time::OffsetDateTime;

#[utoipa::path(
    tag = "Ecash",
    get,
    path = "/signer-status",
    context_path = "/v1/ecash",
    responses(
         (status = 200, content(
            (EcashSignerStatusResponse = "application/json"),
            (EcashSignerStatusResponse = "application/yaml"),
            (EcashSignerStatusResponse = "application/bincode")
        )),
    ),
    params(OutputParams)
)]
pub(crate) async fn signer_status(
    Query(params): Query<OutputParams>,
    State(state): State<Arc<EcashState>>,
) -> ApiResult<FormattedResponse<EcashSignerStatusResponse>> {
    let output = params.get_output();

    let dkg_ecash_epoch_id = state.current_dkg_epoch().await?;

    Ok(output.to_response(
        EcashSignerStatusResponseBody {
            current_time: OffsetDateTime::now_utc(),
            dkg_ecash_epoch_id,
            signer_disabled: state.local.explicitly_disabled,
            is_ecash_signer: state.is_dkg_signer(dkg_ecash_epoch_id).await?,
            has_signing_keys: state.ecash_signing_key().await.is_ok(),
        }
        .sign(state.local.identity_keypair.private_key()),
    ))
}
