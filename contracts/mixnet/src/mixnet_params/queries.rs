use super::storage;
use cosmwasm_std::Deps;
use mixnet_contract::{RewardingIntervalResponse, StateParams};

pub(crate) fn query_state_params(deps: Deps) -> StateParams {
    storage::read_state_params(deps.storage)
}

pub(crate) fn query_rewarding_interval(deps: Deps) -> RewardingIntervalResponse {
    let state = storage::config_read(deps.storage).load().unwrap();
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
    use crate::support::tests::test_helpers;

    use cosmwasm_std::Addr;

    #[test]
    fn query_for_contract_state_works() {
        let mut deps = test_helpers::init_contract();

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

        storage::config(deps.as_mut().storage)
            .save(&dummy_state)
            .unwrap();

        assert_eq!(dummy_state.params, query_state_params(deps.as_ref()))
    }
}
