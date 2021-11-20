use super::storage;
use cosmwasm_std::Deps;
use mixnet_contract::{ContractSettingsParams, RewardingIntervalResponse};

pub(crate) fn query_contract_settings_params(deps: Deps) -> ContractSettingsParams {
    storage::read_contract_settings_params(deps.storage)
}

pub(crate) fn query_rewarding_interval(deps: Deps) -> RewardingIntervalResponse {
    let state = storage::contract_settings_read(deps.storage)
        .load()
        .unwrap();
    RewardingIntervalResponse {
        current_rewarding_interval_starting_block: state.rewarding_interval_starting_block,
        current_rewarding_interval_nonce: state.latest_rewarding_interval_nonce,
        rewarding_in_progress: state.rewarding_in_progress,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mixnet_contract_settings::models::ContractSettings;
    use crate::support::tests::test_helpers;

    use cosmwasm_std::Addr;

    #[test]
    fn query_for_contract_settings_works() {
        let mut deps = test_helpers::init_contract();

        let dummy_state = ContractSettings {
            owner: Addr::unchecked("someowner"),
            rewarding_validator_address: Addr::unchecked("monitor"),
            params: ContractSettingsParams {
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

        storage::contract_settings(deps.as_mut().storage)
            .save(&dummy_state)
            .unwrap();

        assert_eq!(
            dummy_state.params,
            query_contract_settings_params(deps.as_ref())
        )
    }
}
