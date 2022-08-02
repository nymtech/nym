use crate::constants::{INITIAL_GATEWAY_PLEDGE_AMOUNT, INITIAL_MIXNODE_PLEDGE_AMOUNT};
use cosmwasm_std::{coin, Coin};
use mixnet_contract_common::mixnode::MixNodeCostParams;
use mixnet_contract_common::{Gateway, MixNode, Percent};

pub const TEST_COIN_DENOM: &str = "unym";

pub fn mix_node_fixture() -> MixNode {
    MixNode {
        host: "mix.node.org".to_string(),
        mix_port: 1789,
        verloc_port: 1790,
        http_api_port: 8000,
        sphinx_key: "sphinx".to_string(),
        identity_key: "identity".to_string(),
        version: "0.10.0".to_string(),
    }
}

pub fn mix_node_cost_params_fixture() -> MixNodeCostParams {
    MixNodeCostParams {
        profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
        interval_operating_cost: coin(40_000_000, TEST_COIN_DENOM),
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

pub fn good_mixnode_pledge() -> Vec<Coin> {
    vec![Coin {
        denom: TEST_COIN_DENOM.to_string(),
        amount: INITIAL_MIXNODE_PLEDGE_AMOUNT,
    }]
}

pub fn good_gateway_pledge() -> Vec<Coin> {
    vec![Coin {
        denom: TEST_COIN_DENOM.to_string(),
        amount: INITIAL_GATEWAY_PLEDGE_AMOUNT,
    }]
}
