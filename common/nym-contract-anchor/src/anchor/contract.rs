// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::helpers::get_trusted_digest;
use crate::anchor::{ChainAnchor, TrustAnchor, TrustedDigest};
use crate::error::AnchorError;
use async_trait::async_trait;
use cosmrs::AccountId;
use cosmrs::tendermint::AppHash;
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt};

/// One contract's anchor over a chain anchor: the chain's app hash at a height, and the
/// contract's digest item proven against it with an ICS23 membership proof.
pub struct ContractAnchor<A, C> {
    chain: A,
    client: C,
    contract: AccountId,
    digest_key: Vec<u8>,
}

impl<A, C> ContractAnchor<A, C> {
    /// `digest_key` is the contract-side item key the accumulator is stored under, taken as
    /// a parameter rather than a per-domain constant so one anchor type serves every contract.
    pub fn new(chain: A, client: C, contract: AccountId, digest_key: Vec<u8>) -> Self {
        Self {
            chain,
            client,
            contract,
            digest_key,
        }
    }
}

#[async_trait]
impl<A, C> ChainAnchor for ContractAnchor<A, C>
where
    A: ChainAnchor + Send + Sync,
    C: Send + Sync,
{
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, AnchorError> {
        self.chain.trusted_app_hash(height).await
    }
}

#[async_trait]
impl<A, C> TrustAnchor for ContractAnchor<A, C>
where
    A: ChainAnchor + Send + Sync,
    C: TendermintRpcClientExt + Send + Sync,
{
    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, AnchorError> {
        let app_hash = self.chain.trusted_app_hash(height).await?;

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
