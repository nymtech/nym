// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Trust anchors: produce a directory digest the caller is willing to trust at a height.

use crate::error::DirectoryClientError;
use async_trait::async_trait;
use cosmrs::tendermint::AppHash;
use nym_lthash::LtHash16;
use nym_validator_client::nyxd::Height;

pub mod attested;
pub mod checkpoint;
mod helpers;
pub mod proven;

#[cfg(feature = "light-client")]
pub mod light_client;

#[cfg(feature = "light-client")]
pub use light_client::{
    LightClientAnchor, nyx_default_options, verify_checkpoint_advances_one_hop,
};

/// A directory digest trusted at a specific height.
pub struct TrustedDigest {
    pub height: Height,
    pub accumulator: LtHash16,
}

/// Produces the chain state a caller is willing to trust at a height. The verification
/// core (fetch all entries at `H`, recompute, compare; or prove a single entry) is
/// independent of which anchor produced that trust, so alternative anchors (a nym-api
/// quorum, a full light client) can replace the proven one without touching the verifier.
#[async_trait]
pub trait DirectoryTrustAnchor {
    /// The block `app_hash` the caller trusts for state committed at `height` - the root
    /// every ICS23 store proof (the digest item and single entries) is checked against.
    /// Proven mode reads it from a configured RPC's `header[H+1]`; a light-client /
    /// attested anchor can replace that source without changing the verify core.
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, DirectoryClientError>;

    /// The directory digest trusted at `height` (in proven mode, an ICS23 membership proof
    /// of the digest item against [`Self::trusted_app_hash`]).
    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, DirectoryClientError>;
}

#[async_trait]
impl<T: DirectoryTrustAnchor + Send + Sync + ?Sized> DirectoryTrustAnchor for Box<T> {
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, DirectoryClientError> {
        (**self).trusted_app_hash(height).await
    }

    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, DirectoryClientError> {
        (**self).trusted_digest(height).await
    }
}
