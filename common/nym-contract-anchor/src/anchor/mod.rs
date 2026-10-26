// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Trust anchors: the chain state a caller is willing to trust at a height, and a contract's
//! digest within it.

use crate::error::AnchorError;
use async_trait::async_trait;
use cosmrs::tendermint::AppHash;
use nym_lthash::LtHash16;
use nym_validator_client::nyxd::Height;
use std::sync::Arc;

pub mod attested;
pub mod checkpoint;
pub mod contract;
pub mod helpers;
pub mod proven;

#[cfg(feature = "light-client")]
pub mod light_client;

#[cfg(feature = "light-client")]
pub use light_client::{
    LightClientAnchor, nyx_default_options, verify_checkpoint_advances_one_hop,
};

/// A contract digest trusted at a specific height.
pub struct TrustedDigest {
    pub height: Height,
    pub accumulator: LtHash16,
}

/// Trust in the chain itself, independent of any contract: one instance can serve every
/// contract a caller verifies.
#[async_trait]
pub trait ChainAnchor {
    /// The block `app_hash` the caller trusts for state committed at `height` - the root
    /// every ICS23 store proof (the digest item and single entries) is checked against.
    /// Proven mode reads it from a configured RPC's `header[H+1]`; a light-client /
    /// attested anchor can replace that source without changing the verify core.
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, AnchorError>;
}

/// Produces the state of one contract a caller is willing to trust at a height. The verification
/// core (fetch all entries at `H`, recompute, compare; or prove a single entry) is
/// independent of which anchor produced that trust: a `ContractAnchor` over any chain anchor,
/// or an attested nym-api quorum.
#[async_trait]
pub trait TrustAnchor: ChainAnchor {
    /// The contract digest trusted at `height` (in proven mode, an ICS23 membership proof
    /// of the digest item against [`ChainAnchor::trusted_app_hash`]).
    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, AnchorError>;
}

#[async_trait]
impl<T: ChainAnchor + Send + Sync + ?Sized> ChainAnchor for Box<T> {
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, AnchorError> {
        (**self).trusted_app_hash(height).await
    }
}

#[async_trait]
impl<T: TrustAnchor + Send + Sync + ?Sized> TrustAnchor for Box<T> {
    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, AnchorError> {
        (**self).trusted_digest(height).await
    }
}

#[async_trait]
impl<T: ChainAnchor + Send + Sync + ?Sized> ChainAnchor for Arc<T> {
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, AnchorError> {
        (**self).trusted_app_hash(height).await
    }
}
