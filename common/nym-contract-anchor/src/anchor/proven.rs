// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::ChainAnchor;
use crate::error::AnchorError;
use async_trait::async_trait;
use cosmrs::tendermint::AppHash;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt};

/// Proven chain anchor: takes the block `app_hash` from a configured, trusted RPC's
/// `header[H+1]`; a light client can replace that source behind the same trait.
pub struct ProvenChainAnchor<C> {
    client: C,
}

impl<C> ProvenChainAnchor<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }
}

#[async_trait]
impl<C> ChainAnchor for ProvenChainAnchor<C>
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
}
