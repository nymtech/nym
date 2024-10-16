// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiError;
use crate::http::state::ApiState;
use futures::{stream, StreamExt};
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    TicketbookAsyncRequest, TicketbookObtainQueryParams, TicketbookRequest,
    TicketbookWalletSharesResponse, WalletShare,
};
use nym_credentials::IssuanceTicketBook;
use nym_credentials_interface::Base58;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::ecash::BlindSignRequestBody;
use nym_validator_client::nyxd::contract_traits::EcashSigningClient;
use nym_validator_client::nyxd::cosmwasm_client::ToSingletonContractData;
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
) -> Result<Vec<WalletShare>, VpnApiError> {
    let mut rng = OsRng;

    let ed25519_keypair = ed25519::KeyPair::new(&mut rng);

    let epoch = state.current_epoch_id().await?;
    let deposit_amount = state.deposit_amount().await?;
    let threshold = state.ecash_threshold(epoch).await?;
    let expiration_date = request_data.expiration_date;

    // before we commit to making the deposit, ensure we have required signatures cached and stored
    let _ = state.master_verification_key(Some(epoch)).await?;
    let _ = state.master_coin_index_signatures(Some(epoch)).await?;
    let _ = state
        .master_expiration_date_signatures(expiration_date)
        .await?;
    let ecash_api_clients = state.ecash_clients(epoch).await?.clone();

    let chain_write_permit = state.start_chain_tx().await;

    info!("starting the deposit!");
    // TODO: batch those up
    // TODO: batch those up
    let deposit_res = chain_write_permit
        .make_ticketbook_deposit(
            ed25519_keypair.public_key().to_base58_string(),
            deposit_amount.clone(),
            None,
        )
        .await?;

    // explicitly drop it here so other tasks could start using it
    drop(chain_write_permit);

    let deposit_id = deposit_res.parse_singleton_u32_contract_data()?;
    let tx_hash = deposit_res.transaction_hash;
    info!(deposit_id = %deposit_id, tx_hash = %tx_hash, "deposit finished");

    // store the deposit information so if we fail, we could perhaps still reuse it for another issuance
    state
        .storage()
        .insert_deposit_data(
            deposit_id,
            tx_hash,
            requested_on,
            request,
            deposit_amount,
            &request_data.ecash_pubkey,
            &ed25519_keypair,
        )
        .await?;

    let plaintext =
        IssuanceTicketBook::request_plaintext(&request_data.withdrawal_request, deposit_id);
    let signature = ed25519_keypair.private_key().sign(plaintext);

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
            .map_err(|_| VpnApiError::EcashApiRequestTimeout {
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
        return Err(VpnApiError::InsufficientNumberOfCredentials {
            available: shares,
            threshold,
        });
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
) -> Result<Vec<WalletShare>, VpnApiError> {
    let shares = match try_obtain_wallet_shares(state, request, requested_on, request_data).await {
        Ok(shares) => shares,
        Err(err) => {
            let obtained = match err {
                VpnApiError::InsufficientNumberOfCredentials { available, .. } => available,
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
) -> Result<(), VpnApiError> {
    let epoch_id = state.current_epoch_id().await?;

    let device_id = &request_data.device_id;
    let credential_id = &request_data.credential_id;

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

    // 4. build the response
    let response = TicketbookWalletSharesResponse {
        epoch_id,
        shares,
        master_verification_key,
        aggregated_coin_index_signatures,
        aggregated_expiration_date_signatures,
    };

    // 5. call the webhook
    state
        .zk_nym_web_hook()
        .try_trigger(request, &response)
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
) {
    if let Err(err) = try_obtain_blinded_ticketbook_async_inner(
        &state,
        request,
        requested_on,
        request_data,
        params,
    )
    .await
    {
        error!(uuid = %request, "failed to resolve the blinded ticketbook issuance: {err}")
    } else {
        info!(uuid = %request, "managed to resolve the blinded ticketbook issuance")
    }
}
