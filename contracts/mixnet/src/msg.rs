use crate::state::StateParams;
use cosmwasm_std::Addr;
use mixnet_contract::{Gateway, IdentityKey, MixNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
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
        mix_identity: IdentityKey,
    },

    UndelegateFromMixnode {
        mix_identity: IdentityKey,
    },

    DelegateToGateway {
        gateway_identity: IdentityKey,
    },

    UndelegateFromGateway {
        gateway_identity: IdentityKey,
    },

    RewardMixnode {
        identity: IdentityKey,
        // percentage value in range 0-100
        uptime: u32,
    },

    RewardGateway {
        identity: IdentityKey,
        // percentage value in range 0-100
        uptime: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<IdentityKey>,
    },
    GetGateways {
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    OwnsMixnode {
        address: Addr,
    },
    OwnsGateway {
        address: Addr,
    },
    StateParams {},
    GetMixDelegations {
        mix_identity: IdentityKey,
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    GetMixDelegation {
        mix_identity: IdentityKey,
        address: Addr,
    },
    GetGatewayDelegations {
        gateway_identity: IdentityKey,
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    GetGatewayDelegation {
        gateway_identity: IdentityKey,
        address: Addr,
    },
    LayerDistribution {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
