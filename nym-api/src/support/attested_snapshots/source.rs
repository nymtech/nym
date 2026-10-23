// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519;
use nym_directory_client::error::VerificationError;
use nym_lthash::LtHash16;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::AccountId;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeMap;
use tendermint::block::Height;
use tendermint::hash::AppHash;

/// What differs between the attested contracts: the record set verified at a height, and what
/// the source keeps beside it.
#[async_trait]
pub(crate) trait AttestedSnapshotSource: Send + Sync + 'static {
    type Record: Clone + Serialize + DeserializeOwned + Send + Sync + 'static;
    type Auxiliary: Clone + Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Names the contract in logs and in the refresher's name.
    const NAME: &'static str;

    fn contract_address(&self) -> &AccountId;

    /// The verified record set at `height`. `previous` is the auxiliary value of the newest
    /// retained snapshot below `height`, if any.
    async fn verified_set(
        &self,
        height: Height,
        previous: Option<&Self::Auxiliary>,
    ) -> Result<VerifiedSet<Self>, VerificationError>;
}

/// A source's output at one height, for the provider to sign and retain.
pub(crate) struct VerifiedSet<S: AttestedSnapshotSource + ?Sized> {
    pub(crate) app_hash: AppHash,
    pub(crate) accumulator: LtHash16,
    pub(crate) records: Vec<S::Record>,
    pub(crate) node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    pub(crate) auxiliary: S::Auxiliary,
}
