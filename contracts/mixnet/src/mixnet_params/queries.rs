use crate::storage::{
    circulating_supply, config_read, read_layer_distribution, read_state_params, reward_pool_value,
};

use cosmwasm_std::{Deps, Uint128};
use mixnet_contract::{LayerDistribution, RewardingIntervalResponse, StateParams};

pub(crate) fn query_state_params(deps: Deps) -> StateParams {
    read_state_params(deps.storage)
}

pub(crate) fn query_rewarding_interval(deps: Deps) -> RewardingIntervalResponse {
    let state = config_read(deps.storage).load().unwrap();
    RewardingIntervalResponse {
        current_rewarding_interval_starting_block: state.rewarding_interval_starting_block,
        current_rewarding_interval_nonce: state.latest_rewarding_interval_nonce,
        rewarding_in_progress: state.rewarding_in_progress,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mixnet_params::state::State;
    use crate::storage::{config, gateways, mix_delegations, mixnodes};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{
        good_gateway_bond, good_mixnode_bond, raw_delegation_fixture,
    };
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{Addr, Storage};
    use mixnet_contract::{Gateway, MixNode, RawDelegationData};

    #[test]
    fn query_for_contract_state_works() {
        let mut deps = helpers::init_contract();

        let dummy_state = State {
            owner: Addr::unchecked("someowner"),
            rewarding_validator_address: Addr::unchecked("monitor"),
            params: StateParams {
                epoch_length: 1,
                minimum_mixnode_bond: 123u128.into(),
                minimum_gateway_bond: 456u128.into(),
                mixnode_bond_reward_rate: "1.23".parse().unwrap(),
                mixnode_delegation_reward_rate: "7.89".parse().unwrap(),
                mixnode_rewarded_set_size: 1000,
                mixnode_active_set_size: 500,
            },
            rewarding_interval_starting_block: 123,
            latest_rewarding_interval_nonce: 0,
            rewarding_in_progress: false,
            mixnode_epoch_bond_reward: "1.23".parse().unwrap(),
            mixnode_epoch_delegation_reward: "7.89".parse().unwrap(),
        };

        config(deps.as_mut().storage).save(&dummy_state).unwrap();

        assert_eq!(dummy_state.params, query_state_params(deps.as_ref()))
    }
}
