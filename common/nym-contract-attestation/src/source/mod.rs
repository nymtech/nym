// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::AttestationSourceError;
use crate::snapshot::SignedDigestSnapshot;
use crate::snapshot_data::SnapshotData;
use async_trait::async_trait;
use cosmrs::tendermint::block::Height;
use nym_crypto::asymmetric::ed25519;
use serde::de::DeserializeOwned;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

/// A source of nym-api-signed snapshots, so the anchor is independent of any particular
/// transport and can be exercised with a mock. The concrete HTTP transport lives in the
/// consuming client crate.
///
/// One source instance serves one contract. Which contract is fixed when the source is
/// constructed - by the route tree it is pointed at - rather than passed per call, so the
/// anchor stays transport-agnostic and never has to know an HTTP path. A source pointed at
/// the wrong contract fails closed rather than silently anchoring it: `DigestSnapshot` names
/// its contract, and the anchor discards snapshots naming any other before counting quorum.
#[async_trait]
pub trait AttestationSource {
    /// The record type this source's contract holds.
    ///
    /// An associated type rather than a parameter on the trait or the method: a source is
    /// scoped to one contract, so it can serve exactly one record type, and its transport is
    /// typed accordingly. A generic method would oblige every source to produce any record
    /// type on demand, which no real transport can honour.
    type Record: DeserializeOwned + Send;

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

    /// This source's whole record set at `height` - the raw records + node identities a
    /// client recomputes offline against a quorum'd snapshot.
    ///
    /// The record type is [`Self::Record`], so one trait serves every contract without a type
    /// parameter leaking into the anchor - which never calls this and has no use for one.
    async fn snapshot_data(
        &self,
        height: Height,
    ) -> Result<SnapshotData<Self::Record>, AttestationSourceError>;
}
