use std::time::Duration;

use crate::contract::INITIAL_MIXNODE_PLEDGE;
use crate::mixnodes::storage as mixnodes_storage;
use crate::{mixnodes::storage::StoredMixnodeBond, support::tests};
use config::defaults::DENOM;
use cosmwasm_std::{coin, Addr, Coin, Uint128};
use mixnet_contract_common::reward_params::NodeRewardParams;
use mixnet_contract_common::{Gateway, GatewayBond, Interval, Layer, MixNode};
use time::OffsetDateTime;

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
        None,
        None,
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

pub fn node_reward_params_fixture(uptime: u128) -> NodeRewardParams {
    NodeRewardParams::new(0, uptime, true)
}

pub fn epoch_fixture() -> Interval {
    Interval::new(1, OffsetDateTime::now_utc(), Duration::from_secs(3600))
}
