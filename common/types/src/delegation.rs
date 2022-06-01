use crate::currency::{DecCoin, RegisteredCoins};
use crate::error::TypesError;
use mixnet_contract_common::mixnode::DelegationEvent as ContractDelegationEvent;
use mixnet_contract_common::mixnode::PendingUndelegate as ContractPendingUndelegate;
use mixnet_contract_common::Delegation as MixnetContractDelegation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Delegation.ts")
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct Delegation {
    pub owner: String,
    pub node_identity: String,
    pub amount: DecCoin,
    pub block_height: u64,
    pub proxy: Option<String>, // proxy address used to delegate the funds on behalf of another address
}

impl Delegation {
    pub fn from_mixnet_contract(
        delegation: MixnetContractDelegation,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        Ok(Delegation {
            owner: delegation.owner.to_string(),
            node_identity: delegation.node_identity,
            amount: reg.attempt_convert_to_display_dec_coin(delegation.amount.into())?,
            block_height: delegation.block_height,
            proxy: delegation.proxy.map(|d| d.to_string()),
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/DelegationRecord.ts")
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct DelegationRecord {
    pub amount: DecCoin,
    pub block_height: u64,
    pub delegated_on_iso_datetime: String,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/DelegationWithEverything.ts")
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct DelegationWithEverything {
    pub owner: String,
    pub node_identity: String,
    pub amount: DecCoin,
    pub total_delegation: Option<DecCoin>,
    pub pledge_amount: Option<DecCoin>,
    pub block_height: u64,
    pub delegated_on_iso_datetime: String,
    pub profit_margin_percent: Option<u8>,
    pub avg_uptime_percent: Option<u8>,
    pub stake_saturation: Option<f32>,
    pub proxy: Option<String>,
    pub accumulated_rewards: Option<DecCoin>,
    pub pending_events: Vec<DelegationEvent>,
    pub history: Vec<DelegationRecord>,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/DelegationResult.ts")
)]
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct DelegationResult {
    source_address: String,
    target_address: String,
    amount: Option<DecCoin>,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/DelegationEventKind.ts")
)]
#[derive(Clone, Deserialize, Serialize, PartialEq, JsonSchema, Debug)]
pub enum DelegationEventKind {
    Delegate,
    Undelegate,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/DelegationEvent.ts")
)]
#[derive(Clone, Deserialize, Serialize, PartialEq, JsonSchema, Debug)]
pub struct DelegationEvent {
    pub kind: DelegationEventKind,
    pub node_identity: String,
    pub address: String,
    pub amount: Option<DecCoin>,
    pub block_height: u64,
}

impl DelegationEvent {
    pub fn from_mixnet_contract(
        event: ContractDelegationEvent,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        Ok(match event {
            ContractDelegationEvent::Delegate(delegation) => DelegationEvent {
                kind: DelegationEventKind::Delegate,
                block_height: delegation.block_height,
                address: delegation.owner.into_string(),
                node_identity: delegation.node_identity,
                amount: Some(reg.attempt_convert_to_display_dec_coin(delegation.amount.into())?),
            },
            ContractDelegationEvent::Undelegate(pending_undelegate) => DelegationEvent {
                kind: DelegationEventKind::Undelegate,
                block_height: pending_undelegate.block_height(),
                address: pending_undelegate.delegate().into_string(),
                node_identity: pending_undelegate.mix_identity(),
                amount: None,
            },
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/PendingUndelegate.ts")
)]
#[derive(Deserialize, Serialize, PartialEq, JsonSchema, Clone, Debug)]
pub struct PendingUndelegate {
    mix_identity: String,
    delegate: String,
    proxy: Option<String>,
    block_height: u64,
}

impl From<ContractPendingUndelegate> for PendingUndelegate {
    fn from(pending_undelegate: ContractPendingUndelegate) -> Self {
        PendingUndelegate {
            mix_identity: pending_undelegate.mix_identity(),
            delegate: pending_undelegate.delegate().to_string(),
            proxy: pending_undelegate.proxy().map(|p| p.to_string()),
            block_height: pending_undelegate.block_height(),
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/DelegationSummaryResponse.ts")
)]
#[derive(Deserialize, Serialize)]
pub struct DelegationsSummaryResponse {
    pub delegations: Vec<DelegationWithEverything>,
    pub total_delegations: DecCoin,
    pub total_rewards: DecCoin,
}
