use crate::state::StateParams;
use cosmwasm_std::HumanAddr;
use mixnet_contract::{Gateway, IdentityStringPublicKeyWrapper, MixNode};
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
        mix_identity: IdentityStringPublicKeyWrapper,
    },

    UndelegateFromMixnode {
        mix_identity: IdentityStringPublicKeyWrapper,
    },

    DelegateToGateway {
        gateway_identity: IdentityStringPublicKeyWrapper,
    },

    UndelegateFromGateway {
        gateway_identity: IdentityStringPublicKeyWrapper,
    },

    RewardMixnode {
        identity: IdentityStringPublicKeyWrapper,
        // percentage value in range 0-100
        uptime: u32,
    },

    RewardGateway {
        identity: IdentityStringPublicKeyWrapper,
        // percentage value in range 0-100
        uptime: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<IdentityStringPublicKeyWrapper>,
    },
    GetGateways {
        start_after: Option<IdentityStringPublicKeyWrapper>,
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
        mix_identity: IdentityStringPublicKeyWrapper,
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    GetMixDelegation {
        mix_identity: IdentityStringPublicKeyWrapper,
        address: HumanAddr,
    },
    GetGatewayDelegations {
        gateway_identity: IdentityStringPublicKeyWrapper,
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    GetGatewayDelegation {
        gateway_identity: IdentityStringPublicKeyWrapper,
        address: HumanAddr,
    },
    LayerDistribution {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
