// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::http::state::ApiState;
use crate::storage::models::BlindedShares;
use futures::{stream, StreamExt};
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    TicketbookAsyncRequest, TicketbookObtainQueryParams, TicketbookRequest,
    TicketbookWalletSharesResponse, WalletShare, WebhookTicketbookWalletShares,
    WebhookTicketbookWalletSharesRequest,
};
use nym_credentials_interface::Base58;
use nym_validator_client::ecash::BlindSignRequestBody;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nym_api::NymApiClientExt;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

// use the same type alias as our contract without importing the whole thing just for this single line
pub type NodeId = u64;

#[instrument(
    skip(state, request_data, request, requested_on),
    fields(
        expiration_date = %request_data.expiration_date,
        ticketbook_type = %request_data.ticketbook_type
    )
)]
pub(crate) async fn try_obtain_wallet_shares(
    state: &ApiState,
    request: Uuid,
    requested_on: OffsetDateTime,
    request_data: TicketbookRequest,
) -> Result<Vec<WalletShare>, CredentialProxyError> {
    // don't proceed if we don't have quorum available as the request will definitely fail
    if !state.quorum_available() {
        return Err(CredentialProxyError::UnavailableSigningQuorum);
    }

    let epoch = state.current_epoch_id().await?;
    let threshold = state.ecash_threshold(epoch).await?;
    let expiration_date = request_data.expiration_date;

    // before we commit to making the deposit, ensure we have required signatures cached and stored
    let _ = state.master_verification_key(Some(epoch)).await?;
    let _ = state.master_coin_index_signatures(Some(epoch)).await?;
    let _ = state
        .master_expiration_date_signatures(epoch, expiration_date)
        .await?;
    let ecash_api_clients = state.ecash_clients(epoch).await?.clone();

    let deposit_data = state
        .get_deposit(request, requested_on, request_data.ecash_pubkey)
        .await?;
    let deposit_id = deposit_data.deposit_id;
    let signature = deposit_data.sign_ticketbook_plaintext(&request_data.withdrawal_request);

    let credential_request = BlindSignRequestBody::new(
        request_data.withdrawal_request.into(),
        deposit_id,
        signature,
        request_data.ecash_pubkey,
        request_data.expiration_date,
        request_data.ticketbook_type,
    );

    let wallet_shares = Arc::new(Mutex::new(HashMap::new()));

    info!("attempting to contract all nym-apis for the partial wallets...");
    stream::iter(ecash_api_clients)
        .for_each_concurrent(None, |client| async {
            // move the client into the block
            let client = client;

            debug!("contacting {client} for blinded partial wallet");
            let res = timeout(
                Duration::from_secs(5),
                client.api_client.blind_sign(&credential_request),
            )
            .await
            .map_err(|_| CredentialProxyError::EcashApiRequestTimeout {
                client_repr: client.to_string(),
            })
            .and_then(|res| res.map_err(Into::into));

            // 1. try to store it
            if let Err(err) = state
                .storage()
                .insert_partial_wallet_share(
                    deposit_id,
                    epoch,
                    expiration_date,
                    client.node_id,
                    &res,
                )
                .await
            {
                error!("failed to persist issued partial share: {err}")
            }

            // 2. add it to the map
            match res {
                Ok(share) => {
                    wallet_shares
                        .lock()
                        .await
                        .insert(client.node_id, share.blinded_signature);
                }
                Err(err) => {
                    error!("failed to obtain partial blinded wallet share from {client}: {err}")
                }
            }
        })
        .await;

    // SAFETY: the futures have completed, so we MUST have the only arc reference
    #[allow(clippy::unwrap_used)]
    let wallet_shares = Arc::into_inner(wallet_shares).unwrap().into_inner();
    let shares = wallet_shares.len();

    if shares < threshold as usize {
        let err = CredentialProxyError::InsufficientNumberOfCredentials {
            available: shares,
            threshold,
        };
        state
            .insert_deposit_usage_error(deposit_id, err.to_string())
            .await;
        return Err(err);
    }

    Ok(wallet_shares
        .into_iter()
        .map(|(node_index, share)| WalletShare {
            node_index,
            bs58_encoded_share: share.to_bs58(),
        })
        .collect())
}

// same as try_obtain_wallet_shares, but writes failures into the db
async fn try_obtain_wallet_shares_async(
    state: &ApiState,
    request: Uuid,
    requested_on: OffsetDateTime,
    request_data: TicketbookRequest,
    device_id: &str,
    credential_id: &str,
) -> Result<Vec<WalletShare>, CredentialProxyError> {
    let shares = match try_obtain_wallet_shares(state, request, requested_on, request_data).await {
        Ok(shares) => shares,
        Err(err) => {
            let obtained = match err {
                CredentialProxyError::InsufficientNumberOfCredentials { available, .. } => {
                    available
                }
                _ => 0,
            };

            // currently there's no retry mechanisms, but, who knows, that might change
            if let Err(err) = state
                .storage()
                .update_pending_async_blinded_shares_error(
                    obtained,
                    device_id,
                    credential_id,
                    &err.to_string(),
                )
                .await
            {
                error!("failed to update database with the error information: {err}")
            }
            return Err(err);
        }
    };

    Ok(shares)
}

async fn try_obtain_blinded_ticketbook_async_inner(
    state: &ApiState,
    request: Uuid,
    requested_on: OffsetDateTime,
    request_data: TicketbookAsyncRequest,
    params: TicketbookObtainQueryParams,
    pending: &BlindedShares,
) -> Result<(), CredentialProxyError> {
    let epoch_id = state.current_epoch_id().await?;

    let device_id = &request_data.device_id;
    let credential_id = &request_data.credential_id;
    let secret = request_data.secret.clone();

    // 1. try to obtain global data
    let (
        master_verification_key,
        aggregated_expiration_date_signatures,
        aggregated_coin_index_signatures,
    ) = state
        .global_data(
            params.include_master_verification_key,
            params.include_coin_index_signatures,
            params.include_expiration_date_signatures,
            epoch_id,
            request_data.inner.expiration_date,
        )
        .await?;

    // 2. try to obtain shares (failures are written to the DB)
    let shares = try_obtain_wallet_shares_async(
        state,
        request,
        requested_on,
        request_data.inner,
        device_id,
        credential_id,
    )
    .await?;

    // 3. update the storage, if possible
    // (as long as we can trigger webhook, we should still be good)
    if let Err(err) = state
        .storage()
        .update_pending_async_blinded_shares_issued(shares.len(), device_id, credential_id)
        .await
    {
        error!(uuid = %request, "failed to update db with issued information: {err}")
    }

    // 4. build the webhook request body
    let data = Some(TicketbookWalletSharesResponse {
        epoch_id,
        shares,
        master_verification_key,
        aggregated_coin_index_signatures,
        aggregated_expiration_date_signatures,
    });

    let ticketbook_wallet_shares = WebhookTicketbookWalletShares {
        id: pending.id,
        status: pending.status.to_string(),
        device_id: device_id.clone(),
        credential_id: credential_id.clone(),
        data,
        error_message: None,
        created: pending.created,
        updated: pending.updated,
    };

    let webhook_request = WebhookTicketbookWalletSharesRequest {
        ticketbook_wallet_shares,
        secret,
    };

    // 5. call the webhook
    state
        .zk_nym_web_hook()
        .try_trigger(request, &webhook_request)
        .await;

    Ok(())
}

async fn try_trigger_webhook_request_for_error(
    state: &ApiState,
    request: Uuid,
    request_data: TicketbookAsyncRequest,
    pending: &BlindedShares,
    error_message: String,
) -> Result<(), CredentialProxyError> {
    let device_id = &request_data.device_id;
    let credential_id = &request_data.credential_id;
    let secret = request_data.secret.clone();

    let ticketbook_wallet_shares = WebhookTicketbookWalletShares {
        id: pending.id,
        status: "error".to_string(),
        device_id: device_id.clone(),
        credential_id: credential_id.clone(),
        data: None,
        error_message: Some(error_message),
        created: pending.created,
        updated: pending.updated,
    };

    let webhook_request = WebhookTicketbookWalletSharesRequest {
        ticketbook_wallet_shares,
        secret,
    };

    state
        .zk_nym_web_hook()
        .try_trigger(request, &webhook_request)
        .await;

    Ok(())
}

#[instrument(skip_all, fields(credential_id = %request_data.credential_id, device_id = %request_data.device_id))]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn try_obtain_blinded_ticketbook_async(
    state: ApiState,
    request: Uuid,
    requested_on: OffsetDateTime,
    request_data: TicketbookAsyncRequest,
    params: TicketbookObtainQueryParams,
    pending: BlindedShares,
) {
    let skip_webhook = params.skip_webhook;
    if let Err(err) = try_obtain_blinded_ticketbook_async_inner(
        &state,
        request,
        requested_on,
        request_data.clone(),
        params,
        &pending,
    )
    .await
    {
        if skip_webhook {
            info!(uuid = %request,"the webhook is not going to be called for this request");
            return;
        }

        // post to the webhook to notify of errors on this side
        if let Err(webhook_err) = try_trigger_webhook_request_for_error(
            &state,
            request,
            request_data,
            &pending,
            format!("Failed to get ticketbook: {err}"),
        )
        .await
        {
            error!(uuid = %request, "failed to make webhook request to report error: {webhook_err}")
        }
        error!(uuid = %request, "failed to resolve the blinded ticketbook issuance: {err}")
    } else {
        info!(uuid = %request, "managed to resolve the blinded ticketbook issuance")
    }
}
