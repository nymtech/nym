use crate::constants::INITIAL_PLEDGE_AMOUNT;
use cosmwasm_std::{coin, Coin};
use mixnet_contract_common::mixnode::NodeCostParams;
use mixnet_contract_common::reward_params::ActiveSetUpdate;
use mixnet_contract_common::{
    Gateway, MixNode, Percent, DEFAULT_INTERVAL_OPERATING_COST_AMOUNT,
    DEFAULT_PROFIT_MARGIN_PERCENT,
};

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

pub fn node_cost_params_fixture() -> NodeCostParams {
    NodeCostParams {
        profit_margin_percent: Percent::from_percentage_value(DEFAULT_PROFIT_MARGIN_PERCENT)
            .unwrap(),
        interval_operating_cost: coin(DEFAULT_INTERVAL_OPERATING_COST_AMOUNT, TEST_COIN_DENOM),
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

pub fn good_node_plegge() -> Vec<Coin> {
    vec![Coin {
        denom: TEST_COIN_DENOM.to_string(),
        amount: INITIAL_PLEDGE_AMOUNT,
    }]
}

pub fn good_mixnode_pledge() -> Vec<Coin> {
    good_node_plegge()
}

pub fn good_gateway_pledge() -> Vec<Coin> {
    good_node_plegge()
}

pub fn active_set_update_fixture() -> ActiveSetUpdate {
    ActiveSetUpdate {
        entry_gateways: 30,
        exit_gateways: 30,
        mixnodes: 30,
    }
}
