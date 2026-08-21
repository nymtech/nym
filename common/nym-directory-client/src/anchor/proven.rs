// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::helpers::get_trusted_directory_digest;
use crate::anchor::{DirectoryTrustAnchor, TrustedDigest};
use crate::error::DirectoryClientError;
use async_trait::async_trait;
use cosmrs::AccountId;
use cosmrs::tendermint::AppHash;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt};

/// Proven anchor: proves the on-chain `digest_state` item via an ICS23 membership
/// proof against the block `app_hash`. Phase 1a takes the `app_hash` from a configured
/// RPC's `header[H+1]`; a light client can replace that source behind the same trait.
pub struct ProvenTrustAnchor<C> {
    client: C,
    directory_contract: AccountId,
}

impl<C> ProvenTrustAnchor<C> {
    pub fn new(client: C, directory_contract: AccountId) -> Self {
        Self {
            client,
            directory_contract,
        }
    }
}

#[async_trait]
impl<C> DirectoryTrustAnchor for ProvenTrustAnchor<C>
where
    C: TendermintRpcClientExt + Send + Sync,
{
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, DirectoryClientError> {
        // the app_hash committing state at H lives in header[H+1] (CometBFT off-by-one)
        let next: Height = (height.value() as u32 + 1).into();
        Ok(self
            .client
            .header(next)
            .await
            .map_err(NyxdError::from)?
            .header
            .app_hash)
    }

    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, DirectoryClientError> {
        // the trusted app_hash for H (from the same anchor the single-entry read uses)
        let app_hash = self.trusted_app_hash(height).await?;

        get_trusted_directory_digest(&self.client, &self.directory_contract, height, app_hash).await
    }
}
