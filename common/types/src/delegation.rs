use crate::currency::DecCoin;
use crate::error::TypesError;
use log::error;
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
        display_amount: DecCoin,
    ) -> Self {
        Delegation {
            owner: delegation.owner.to_string(),
            node_identity: delegation.node_identity,
            amount: display_amount,
            block_height: delegation.block_height,
            proxy: delegation.proxy.map(|d| d.to_string()),
        }
    }
}

impl TryFrom<MixnetContractDelegation> for Delegation {
    type Error = TypesError;

    fn try_from(delegation: MixnetContractDelegation) -> Result<Self, Self::Error> {
        todo!()
        // let MixnetContractDelegation {
        //     owner,
        //     node_identity,
        //     amount,
        //     block_height,
        //     proxy,
        // } = value;
        //
        // let amount: MajorCurrencyAmount = amount.into();
        //
        // Ok(Delegation {
        //     owner: owner.into_string(),
        //     node_identity,
        //     amount,
        //     block_height,
        //     proxy: proxy.map(|p| p.into_string()),
        // })
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

impl DelegationResult {
    pub fn new(
        source_address: &str,
        target_address: &str,
        amount: Option<DecCoin>,
    ) -> DelegationResult {
        DelegationResult {
            source_address: source_address.to_string(),
            target_address: target_address.to_string(),
            amount,
        }
    }
}

impl TryFrom<MixnetContractDelegation> for DelegationResult {
    type Error = TypesError;

    fn try_from(delegation: MixnetContractDelegation) -> Result<Self, Self::Error> {
        todo!()
        // let amount: DecCoin = delegation.amount.clone().into();
        // Ok(DelegationResult {
        //     source_address: delegation.owner().to_string(),
        //     target_address: delegation.node_identity(),
        //     amount: Some(amount),
        // })
    }
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

impl TryFrom<ContractDelegationEvent> for DelegationEvent {
    type Error = TypesError;

    fn try_from(event: ContractDelegationEvent) -> Result<Self, Self::Error> {
        todo!()
        //
        // match event {
        //     ContractDelegationEvent::Delegate(delegation) => {
        //         let amount: MajorCurrencyAmount = delegation.amount.into();
        //         Ok(DelegationEvent {
        //             kind: DelegationEventKind::Delegate,
        //             block_height: delegation.block_height,
        //             address: delegation.owner.into_string(),
        //             node_identity: delegation.node_identity,
        //             amount: Some(amount),
        //         })
        //     }
        //     ContractDelegationEvent::Undelegate(pending_undelegate) => Ok(DelegationEvent {
        //         kind: DelegationEventKind::Undelegate,
        //         block_height: pending_undelegate.block_height(),
        //         address: pending_undelegate.delegate().into_string(),
        //         node_identity: pending_undelegate.mix_identity(),
        //         amount: None,
        //     }),
        // }
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

pub fn from_contract_delegation_events(
    events: Vec<ContractDelegationEvent>,
) -> Result<Vec<DelegationEvent>, TypesError> {
    let (events, errors): (Vec<_>, Vec<_>) = events
        .into_iter()
        .map(|delegation_event| delegation_event.try_into())
        .partition(Result::is_ok);

    if errors.is_empty() {
        let events = events
            .into_iter()
            .filter_map(|e| e.ok())
            .collect::<Vec<DelegationEvent>>();
        return Ok(events);
    }
    let errors = errors
        .into_iter()
        .filter_map(|e| e.err())
        .collect::<Vec<TypesError>>();

    error!("Failed to convert delegations: {:?}", errors);
    Err(TypesError::DelegationsInvalid)
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
