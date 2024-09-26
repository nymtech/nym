// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::currency::DecCoin;
use crate::error::TypesError;
use crate::pending_events::{PendingEpochEvent, PendingEpochEventData};
use nym_mixnet_contract_common::{IdentityKey, NodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DelegationEventKind.ts"
    )
)]
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, Debug)]
pub enum DelegationEventKind {
    Delegate,
    Undelegate,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DelegationEvent.ts"
    )
)]
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, Debug)]
pub struct DelegationEvent {
    pub kind: DelegationEventKind,
    pub mix_id: NodeId,
    pub address: String,
    pub amount: Option<DecCoin>,
    pub proxy: Option<String>,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/WrappedDelegationEvent.ts"
    )
)]
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, Debug)]
pub struct WrappedDelegationEvent {
    pub event: DelegationEvent,
    pub node_identity: IdentityKey,
}

impl WrappedDelegationEvent {
    pub fn new(event: DelegationEvent, node_identity: IdentityKey) -> Self {
        Self {
            event,
            node_identity,
        }
    }
}

impl DelegationEvent {
    pub fn address_matches(&self, address: &str) -> bool {
        self.address == address
    }
}

pub fn convert_to_delegation_events(epoch_events: Vec<PendingEpochEvent>) -> Vec<DelegationEvent> {
    epoch_events
        .into_iter()
        // filter out all events that are not delegation-related
        .filter_map(|e| e.try_into().ok())
        .collect()
}

impl TryFrom<PendingEpochEvent> for DelegationEvent {
    type Error = TypesError;

    fn try_from(value: PendingEpochEvent) -> Result<Self, Self::Error> {
        value.event.try_into()
    }
}

impl TryFrom<PendingEpochEventData> for DelegationEvent {
    type Error = TypesError;

    fn try_from(value: PendingEpochEventData) -> Result<Self, Self::Error> {
        match value {
            PendingEpochEventData::Delegate {
                owner,
                mix_id,
                amount,
                proxy,
            } => Ok(DelegationEvent {
                kind: DelegationEventKind::Delegate,
                address: owner,
                mix_id,
                proxy,
                amount: Some(amount),
            }),
            PendingEpochEventData::Undelegate {
                owner,
                mix_id,
                proxy,
            } => Ok(DelegationEvent {
                kind: DelegationEventKind::Undelegate,
                address: owner,
                mix_id,
                proxy,
                amount: None,
            }),
            _ => Err(TypesError::NotADelegationEvent),
        }
    }
}
