use crate::state::StateParams;
use cosmwasm_std::HumanAddr;
use mixnet_contract::{Gateway, MixNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    BondMixnode {
        mix_node: MixNode,
    },
    UnbondMixnode {},
    BondGateway {
        gateway: Gateway,
    },
    UnbondGateway {},
    UpdateStateParams(StateParams),

    DelegateToMixnode {
        node_owner: HumanAddr,
    },

    UndelegateFromMixnode {
        node_owner: HumanAddr,
    },

    RewardMixnode {
        owner: HumanAddr,
        // percentage value in range 0-100
        uptime: u32,
    },

    RewardGateway {
        owner: HumanAddr,
        // percentage value in range 0-100
        uptime: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<HumanAddr>,
    },
    GetGateways {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    OwnsMixnode {
        address: HumanAddr,
    },
    OwnsGateway {
        address: HumanAddr,
    },
    StateParams {},
    GetMixDelegations {
        mix_owner: HumanAddr,
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    GetMixDelegation {
        mix_owner: HumanAddr,
        address: HumanAddr,
    },
    GetGatewayDelegations {
        gateway_owner: HumanAddr,
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    GetGatewayDelegation {
        gateway_owner: HumanAddr,
        address: HumanAddr,
    },
    LayerDistribution {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
