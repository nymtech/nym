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
    UnbondMixnode {
        mix_identity: String,
    },
    BondGateway {
        gateway: Gateway,
    },
    UnbondGateway {
        gateway_identity: String,
    },
    UpdateStateParams(StateParams),

    DelegateToMixnode {
        mix_identity: String,
    },

    UndelegateFromMixnode {
        mix_identity: String,
    },

    DelegateToGateway {
        gateway_identity: String,
    },

    UndelegateFromGateway {
        gateway_identity: String,
    },

    RewardMixnode {
        identity: String,
        // percentage value in range 0-100
        uptime: u32,
    },

    RewardGateway {
        identity: String,
        // percentage value in range 0-100
        uptime: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<String>,
    },
    GetGateways {
        start_after: Option<String>,
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
        mix_identity: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetMixDelegation {
        mix_identity: String,
        address: HumanAddr,
    },
    GetGatewayDelegations {
        gateway_identity: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetGatewayDelegation {
        gateway_identity: String,
        address: HumanAddr,
    },
    LayerDistribution {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
