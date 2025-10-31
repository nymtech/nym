// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::CredentialProxyError;
use crate::nym_api_helpers::ensure_sane_expiration_date;
use crate::ticketbook_manager::TicketbookManager;
use nym_compact_ecash::Base58;
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    CurrentEpochResponse, DepositResponse, GlobalDataParams, MasterVerificationKeyResponse,
    ObtainTicketBookSharesAsyncResponse, PartialVerificationKey, PartialVerificationKeysResponse,
    TicketbookAsyncRequest, TicketbookObtainParams, TicketbookRequest,
    TicketbookWalletSharesAsyncResponse, TicketbookWalletSharesResponse,
};
use time::OffsetDateTime;
use tracing::{Instrument, Level, error, info, span, warn};
use uuid::Uuid;

impl TicketbookManager {
    pub async fn obtain_ticketbook_shares(
        &self,
        uuid: Uuid,
        request: TicketbookRequest,
        params: GlobalDataParams,
    ) -> Result<TicketbookWalletSharesResponse, CredentialProxyError> {
        let requested_on = OffsetDateTime::now_utc();
        let span = span!(Level::INFO, "obtain ticketboook", uuid = %uuid);

        async move {
            info!("");

            self.state.ensure_credentials_issuable().await?;
            let epoch_id = self.state.current_epoch_id().await?;
            ensure_sane_expiration_date(request.expiration_date)?;

            // if additional data was requested, grab them first in case there are any cache/network issues
            let (
                master_verification_key,
                aggregated_expiration_date_signatures,
                aggregated_coin_index_signatures,
            ) = self
                .state
                .global_data(params, epoch_id, request.expiration_date)
                .await?;

            let shares = self
                .try_obtain_wallet_shares(uuid, requested_on, request)
                .await
                .inspect_err(|err| warn!("shares request failure: {err}"))?;

            info!("request was successful!");
            Ok(TicketbookWalletSharesResponse {
                epoch_id,
                shares,
                master_verification_key,
                aggregated_coin_index_signatures,
                aggregated_expiration_date_signatures,
            })
        }
        .instrument(span)
        .await
    }

    pub async fn obtain_ticketbook_shares_async(
        &self,
        uuid: Uuid,
        request: TicketbookAsyncRequest,
        params: TicketbookObtainParams,
    ) -> Result<ObtainTicketBookSharesAsyncResponse, CredentialProxyError> {
        let requested_on = OffsetDateTime::now_utc();
        let span = span!(Level::INFO, "[async] obtain ticketboook", uuid = %uuid);
        async move {
            info!("");

            // 1. perform basic validation
            self.state.ensure_credentials_issuable().await?;

            ensure_sane_expiration_date(request.inner.expiration_date)?;

            // 2. store the request to retrieve the id
            let pending = self
                .state
                .storage()
                .insert_new_pending_async_shares_request(
                    uuid,
                    &request.device_id,
                    &request.credential_id,
                )
                .await
                .inspect_err(|err| error!("failed to insert new pending async shares: {err}"))?;

            let id = pending.id;

            // 3. try to spawn a new task attempting to resolve the request
            let this = self.clone();
            if self
                .try_spawn_in_background(async move {
                    this.try_obtain_blinded_ticketbook_async(
                        uuid,
                        requested_on,
                        request,
                        params,
                        pending,
                    )
                    .await
                })
                .is_none()
            {
                warn!("could not start async ticketbook issuance due to shutdown in progress");
                return Err(CredentialProxyError::ShutdownInProgress);
            }

            // 4. in the meantime, return the id to the user
            Ok(TicketbookWalletSharesAsyncResponse { id, uuid }.into())
        }
        .instrument(span)
        .await
    }

    pub async fn current_deposit(&self) -> Result<DepositResponse, CredentialProxyError> {
        let current_deposit = self.state.deposit_amount().await?;
        Ok(DepositResponse {
            current_deposit_amount: current_deposit.amount,
            current_deposit_denom: current_deposit.denom,
        })
    }

    pub async fn partial_verification_keys(
        &self,
    ) -> Result<PartialVerificationKeysResponse, CredentialProxyError> {
        self.state.ensure_credentials_issuable().await?;

        let epoch_id = self.state.current_epoch_id().await?;
        let signers = self.state.ecash_clients(epoch_id).await?;
        Ok(PartialVerificationKeysResponse {
            epoch_id,
            keys: signers
                .iter()
                .map(|signer| PartialVerificationKey {
                    node_index: signer.node_id,
                    bs58_encoded_key: signer.verification_key.to_bs58(),
                })
                .collect(),
        })
    }

    pub async fn master_verification_key(
        &self,
    ) -> Result<MasterVerificationKeyResponse, CredentialProxyError> {
        self.state.ensure_credentials_issuable().await?;

        let epoch_id = self.state.current_epoch_id().await?;
        let key = self.state.master_verification_key(Some(epoch_id)).await?;
        Ok(MasterVerificationKeyResponse {
            epoch_id,
            bs58_encoded_key: key.to_bs58(),
        })
    }

    pub async fn current_epoch(&self) -> Result<CurrentEpochResponse, CredentialProxyError> {
        self.state.ensure_credentials_issuable().await?;
        let epoch_id = self.state.current_epoch_id().await?;
        Ok(CurrentEpochResponse { epoch_id })
    }
}
