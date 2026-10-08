// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::helpers::get_trusted_digest;
use crate::anchor::{TrustAnchor, TrustedDigest};
use crate::error::AnchorError;
use async_trait::async_trait;
use cosmrs::AccountId;
use cosmrs::tendermint::AppHash;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt};

/// Proven anchor: proves the contract's on-chain digest item via an ICS23 membership
/// proof against the block `app_hash`. Phase 1a takes the `app_hash` from a configured
/// RPC's `header[H+1]`; a light client can replace that source behind the same trait.
pub struct ProvenTrustAnchor<C> {
    client: C,
    contract: AccountId,
    digest_key: Vec<u8>,
}

impl<C> ProvenTrustAnchor<C> {
    /// `digest_key` is the contract-side item key the accumulator is stored under, taken as
    /// a parameter rather than a per-domain constant so one anchor serves every contract.
    pub fn new(client: C, contract: AccountId, digest_key: Vec<u8>) -> Self {
        Self {
            client,
            contract,
            digest_key,
        }
    }
}

#[async_trait]
impl<C> TrustAnchor for ProvenTrustAnchor<C>
where
    C: TendermintRpcClientExt + Send + Sync,
{
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, AnchorError> {
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

    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, AnchorError> {
        // the trusted app_hash for H (from the same anchor the single-entry read uses)
        let app_hash = self.trusted_app_hash(height).await?;

        get_trusted_digest(
            &self.client,
            &self.contract,
            &self.digest_key,
            height,
            app_hash,
        )
        .await
    }
}
