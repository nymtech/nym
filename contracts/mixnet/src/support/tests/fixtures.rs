use crate::constants::{INTERVAL_REWARD_PERCENT, SYBIL_RESISTANCE_PERCENT};
use crate::contract::{INITIAL_MIXNODE_PLEDGE, INITIAL_REWARD_POOL};
use crate::mixnodes::storage as mixnodes_storage;
use crate::{mixnodes::storage::StoredMixnodeBond, support::tests};
use config::defaults::{DENOM, TOTAL_SUPPLY};
use cosmwasm_std::{coin, Addr, Coin, Uint128};
use mixnet_contract_common::reward_params::{IntervalRewardParams, NodeRewardParams, RewardParams};
use mixnet_contract_common::{Gateway, GatewayBond, Layer, MixNode};

pub fn mix_node_fixture() -> MixNode {
    MixNode {
        host: "mix.node.org".to_string(),
        mix_port: 1789,
        verloc_port: 1790,
        http_api_port: 8000,
        sphinx_key: "sphinx".to_string(),
        identity_key: "identity".to_string(),
        version: "0.10.0".to_string(),
        profit_margin_percent: 10,
    }
}

pub fn gateway_fixture() -> Gateway {
    Gateway {
        host: "1.1.1.1".to_string(),
        mix_port: 1789,
        clients_port: 9000,
        location: "Sweden".to_string(),
        sphinx_key: "sphinx".to_string(),
        identity_key: "identity".to_string(),
        version: "0.10.0".to_string(),
    }
}

pub fn gateway_bond_fixture(owner: &str) -> GatewayBond {
    let gateway = Gateway {
        identity_key: format!("id-{}", owner),
        ..tests::fixtures::gateway_fixture()
    };
    GatewayBond::new(
        coin(50, DENOM),
        Addr::unchecked(owner),
        12_345,
        gateway,
        None,
    )
}

pub(crate) fn stored_mixnode_bond_fixture(owner: &str) -> mixnodes_storage::StoredMixnodeBond {
    StoredMixnodeBond::new(
        coin(50, DENOM),
        Addr::unchecked(owner),
        Layer::One,
        12_345,
        MixNode {
            identity_key: format!("id-{}", owner),
            ..super::fixtures::mix_node_fixture()
        },
        None,
        Uint128::zero(),
        None
    )
}

pub fn good_mixnode_pledge() -> Vec<Coin> {
    vec![Coin {
        denom: DENOM.to_string(),
        amount: INITIAL_MIXNODE_PLEDGE,
    }]
}

pub fn good_gateway_pledge() -> Vec<Coin> {
    vec![Coin {
        denom: DENOM.to_string(),
        amount: INITIAL_MIXNODE_PLEDGE,
    }]
}

// when exact values are irrelevant and what matters is the action of rewarding
pub fn rewarding_params_fixture(uptime: u128) -> RewardParams {
    let interval_reward_params = IntervalRewardParams::new(
        (INITIAL_REWARD_POOL / 100 / 720) * INTERVAL_REWARD_PERCENT as u128,
        50 as u128,
        25 as u128,
        TOTAL_SUPPLY - INITIAL_REWARD_POOL,
        SYBIL_RESISTANCE_PERCENT,
        10,
    );

    let node_reward_params = NodeRewardParams::new(0, uptime, true);

    RewardParams::new(interval_reward_params, node_reward_params)
}
