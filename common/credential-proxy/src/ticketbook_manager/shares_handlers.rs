// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::CredentialProxyError;
use crate::storage::models::MinimalWalletShare;
use crate::ticketbook_manager::TicketbookManager;
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    GlobalDataParams, TicketbookWalletSharesResponse,
};
use nym_validator_client::nym_api::EpochId;
use tracing::{debug, span, Instrument, Level};
use uuid::Uuid;

impl TicketbookManager {
    async fn shares_to_response(
        &self,
        shares: Vec<MinimalWalletShare>,
        params: GlobalDataParams,
    ) -> Result<TicketbookWalletSharesResponse, CredentialProxyError> {
        // in all calls we ensured the shares are non-empty
        #[allow(clippy::unwrap_used)]
        let first = shares.first().unwrap();
        let expiration_date = first.expiration_date;
        let epoch_id = first.epoch_id as EpochId;

        let threshold = self.state.ecash_threshold(epoch_id).await?;
        if shares.len() < threshold as usize {
            return Err(CredentialProxyError::InsufficientNumberOfCredentials {
                available: shares.len(),
                threshold,
            });
        }

        // grab any requested additional data
        let (
            master_verification_key,
            aggregated_expiration_date_signatures,
            aggregated_coin_index_signatures,
        ) = self
            .state
            .global_data(params, epoch_id, expiration_date)
            .await?;

        // finally produce a response
        Ok(TicketbookWalletSharesResponse {
            epoch_id,
            shares: shares.into_iter().map(Into::into).collect(),
            master_verification_key,
            aggregated_coin_index_signatures,
            aggregated_expiration_date_signatures,
        })
    }

    /// Query by id for blinded shares of a bandwidth voucher
    pub async fn query_for_shares_by_id(
        &self,
        uuid: Uuid,
        params: GlobalDataParams,
        share_id: i64,
    ) -> Result<TicketbookWalletSharesResponse, CredentialProxyError> {
        let span = span!(Level::INFO, "query shares by id", uuid = %uuid, share_id = %share_id);
        async move {
            debug!("");

            // TODO: edge case: this will **NOT** work if shares got created in epoch X,
            // but this query happened in epoch X+1
            let shares = self
                .state
                .storage()
                .load_wallet_shares_by_shares_id(share_id)
                .await?;
            if shares.is_empty() {
                debug!("shares not found");

                // check for explicit error
                if let Some(error_message) = self
                    .state
                    .storage()
                    .load_shares_error_by_shares_id(share_id)
                    .await?
                {
                    return Err(CredentialProxyError::ShareByIdLoadError {
                        message: error_message,
                        id: share_id,
                    });
                }

                return Err(CredentialProxyError::SharesByIdNotFound { id: share_id });
            }

            self.shares_to_response(shares, params).await
        }
        .instrument(span)
        .await
    }

    /// Query by id for blinded  wallet shares of a ticketbook
    pub async fn query_for_shares_by_device_id_and_credential_id(
        &self,
        uuid: Uuid,
        params: GlobalDataParams,
        device_id: String,
        credential_id: String,
    ) -> Result<TicketbookWalletSharesResponse, CredentialProxyError> {
        let span = span!(Level::INFO, "query shares by device and credential ids", uuid = %uuid, device_id = %device_id, credential_id = %credential_id);
        async move {
            debug!("");

            // TODO: edge case: this will **NOT** work if shares got created in epoch X,
            // but this query happened in epoch X+1
            let shares = self
                .state
                .storage()
                .load_wallet_shares_by_device_and_credential_id(&device_id, &credential_id)
                .await?;

            if shares.is_empty() {
                debug!("shares not found");

                // check for explicit error
                if let Some(error_message) = self
                    .state
                    .storage()
                    .load_shares_error_by_device_and_credential_id(&device_id, &credential_id)
                    .await?
                {
                    return Err(CredentialProxyError::ShareByDeviceLoadError {
                        message: error_message,
                        device_id,
                        credential_id,
                    });
                }

                return Err(CredentialProxyError::SharesByDeviceNotFound {
                    device_id,
                    credential_id,
                });
            }

            self.shares_to_response(shares, params).await
        }
        .instrument(span)
        .await
    }
}
