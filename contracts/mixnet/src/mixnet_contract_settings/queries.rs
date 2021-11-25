use super::storage;
use cosmwasm_std::{Deps, StdResult};
use mixnet_contract::{ContractSettingsParams, RewardingIntervalResponse};

pub(crate) fn query_contract_settings_params(deps: Deps) -> StdResult<ContractSettingsParams> {
    storage::CONTRACT_SETTINGS
        .load(deps.storage)
        .map(|settings| settings.params)
}

pub(crate) fn query_rewarding_interval(deps: Deps) -> StdResult<RewardingIntervalResponse> {
    let state = storage::CONTRACT_SETTINGS.load(deps.storage)?;

    Ok(RewardingIntervalResponse {
        current_rewarding_interval_starting_block: state.rewarding_interval_starting_block,
        current_rewarding_interval_nonce: state.latest_rewarding_interval_nonce,
        rewarding_in_progress: state.rewarding_in_progress,
    })
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
                minimum_mixnode_bond: 123u128.into(),
                minimum_gateway_bond: 456u128.into(),
                mixnode_rewarded_set_size: 1000,
                mixnode_active_set_size: 500,
            },
            rewarding_interval_starting_block: 123,
            latest_rewarding_interval_nonce: 0,
            rewarding_in_progress: false,
        };

        storage::CONTRACT_SETTINGS
            .save(deps.as_mut().storage, &dummy_state)
            .unwrap();

        assert_eq!(
            dummy_state.params,
            query_contract_settings_params(deps.as_ref()).unwrap()
        )
    }
}
