// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::AttestationSourceError;
use crate::snapshot::SignedDigestSnapshot;
use async_trait::async_trait;
use cosmrs::tendermint::block::Height;
use nym_crypto::asymmetric::ed25519;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

/// A source of nym-api-signed directory snapshots, so the anchor is independent of any
/// particular transport and can be exercised with a mock. The concrete HTTP transport
/// lives in the consuming client crate.
#[async_trait]
pub trait AttestationSource {
    /// This source's ed25519 identity key.
    fn identity(&self) -> ed25519::PublicKey;

    /// This source's latest signed snapshot.
    async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, AttestationSourceError>;

    /// This source's signed snapshot at a specific height, if still within its
    /// retained window.
    async fn snapshot_at(
        &self,
        height: Height,
    ) -> Result<SignedDigestSnapshot, AttestationSourceError>;
}
