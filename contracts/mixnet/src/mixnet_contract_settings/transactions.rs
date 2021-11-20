use super::storage;
use crate::error::ContractError;
use crate::rewards::helpers::calculate_epoch_reward_rate;
use cosmwasm_std::Decimal;
use cosmwasm_std::DepsMut;
use cosmwasm_std::MessageInfo;
use cosmwasm_std::Response;
use mixnet_contract::ContractSettingsParams;

pub(crate) fn try_update_state_params(
    deps: DepsMut,
    info: MessageInfo,
    params: ContractSettingsParams,
) -> Result<Response, ContractError> {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    let mut state = storage::contract_settings_read(deps.storage).load()?;

    // check if this is executed by the owner, if not reject the transaction
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized);
    }

    if params.mixnode_bond_reward_rate < Decimal::one() {
        return Err(ContractError::DecreasingMixnodeBondReward);
    }

    if params.mixnode_delegation_reward_rate < Decimal::one() {
        return Err(ContractError::DecreasingMixnodeDelegationReward);
    }

    // note: rewarded_set = active_set + idle_set
    // hence rewarded set must always be bigger than (or equal to) the active set
    if params.mixnode_rewarded_set_size < params.mixnode_active_set_size {
        return Err(ContractError::InvalidActiveSetSize);
    }

    // if we're updating epoch length, recalculate rewards for mixnodes
    if state.params.epoch_length != params.epoch_length {
        state.mixnode_epoch_bond_reward =
            calculate_epoch_reward_rate(params.epoch_length, params.mixnode_bond_reward_rate);
        state.mixnode_epoch_delegation_reward =
            calculate_epoch_reward_rate(params.epoch_length, params.mixnode_delegation_reward_rate);
    } else {
        // if mixnode rewards changed, recalculate respective values
        if state.params.mixnode_bond_reward_rate != params.mixnode_bond_reward_rate {
            state.mixnode_epoch_bond_reward =
                calculate_epoch_reward_rate(params.epoch_length, params.mixnode_bond_reward_rate);
        }
        if state.params.mixnode_delegation_reward_rate != params.mixnode_delegation_reward_rate {
            state.mixnode_epoch_delegation_reward = calculate_epoch_reward_rate(
                params.epoch_length,
                params.mixnode_delegation_reward_rate,
            );
        }
    }

    state.params = params;

    storage::contract_settings(deps.storage).save(&state)?;

    Ok(Response::default())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::{
        INITIAL_DEFAULT_EPOCH_LENGTH, INITIAL_GATEWAY_BOND, INITIAL_MIXNODE_BOND,
        INITIAL_MIXNODE_BOND_REWARD_RATE, INITIAL_MIXNODE_DELEGATION_REWARD_RATE,
    };
    use crate::error::ContractError;
    use crate::mixnet_contract_settings::transactions::try_update_state_params;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::Decimal;
    use cosmwasm_std::Response;
    use mixnet_contract::ContractSettingsParams;

    #[test]
    fn updating_state_params() {
        let mut deps = test_helpers::init_contract();

        let new_params = ContractSettingsParams {
            epoch_length: INITIAL_DEFAULT_EPOCH_LENGTH,
            minimum_mixnode_bond: INITIAL_MIXNODE_BOND,
            minimum_gateway_bond: INITIAL_GATEWAY_BOND,
            mixnode_bond_reward_rate: Decimal::percent(INITIAL_MIXNODE_BOND_REWARD_RATE),
            mixnode_delegation_reward_rate: Decimal::percent(
                INITIAL_MIXNODE_DELEGATION_REWARD_RATE,
            ),
            mixnode_rewarded_set_size: 100,
            mixnode_active_set_size: 50,
        };

        // cannot be updated from non-owner account
        let info = mock_info("not-the-creator", &[]);
        let res = try_update_state_params(deps.as_mut(), info, new_params.clone());
        assert_eq!(res, Err(ContractError::Unauthorized));

        // but works fine from the creator account
        let info = mock_info("creator", &[]);
        let res = try_update_state_params(deps.as_mut(), info, new_params.clone());
        assert_eq!(res, Ok(Response::default()));

        // and the state is actually updated
        let current_state = storage::contract_settings_read(deps.as_ref().storage)
            .load()
            .unwrap();
        assert_eq!(current_state.params, new_params);

        // mixnode_epoch_rewards are recalculated if annual reward  is changed
        let current_mix_bond_reward_rate = current_state.mixnode_epoch_bond_reward;
        let current_mix_delegation_reward_rate = current_state.mixnode_epoch_delegation_reward;
        let new_mixnode_bond_reward_rate = Decimal::percent(120);
        let new_mixnode_delegation_reward_rate = Decimal::percent(120);

        // sanity check to make sure we are actually updating the values (in case we changed defaults at some point)
        assert_ne!(new_mixnode_bond_reward_rate, current_mix_bond_reward_rate);
        assert_ne!(
            new_mixnode_delegation_reward_rate,
            current_mix_delegation_reward_rate
        );

        let mut new_params = current_state.params.clone();
        new_params.mixnode_bond_reward_rate = new_mixnode_bond_reward_rate;
        new_params.mixnode_delegation_reward_rate = new_mixnode_delegation_reward_rate;

        let info = mock_info("creator", &[]);
        try_update_state_params(deps.as_mut(), info, new_params.clone()).unwrap();

        let new_state = storage::contract_settings_read(deps.as_ref().storage)
            .load()
            .unwrap();
        let expected_bond =
            calculate_epoch_reward_rate(new_params.epoch_length, new_mixnode_bond_reward_rate);
        let expected_delegation = calculate_epoch_reward_rate(
            new_params.epoch_length,
            new_mixnode_delegation_reward_rate,
        );
        assert_eq!(expected_bond, new_state.mixnode_epoch_bond_reward);
        assert_eq!(
            expected_delegation,
            new_state.mixnode_epoch_delegation_reward
        );

        // mixnode_epoch_rewards is updated on epoch length change
        let new_epoch_length = 42;
        // sanity check to make sure we are actually updating the value (in case we changed defaults at some point)
        assert_ne!(new_epoch_length, current_state.params.epoch_length);
        let mut new_params = current_state.params.clone();
        new_params.epoch_length = new_epoch_length;

        let info = mock_info("creator", &[]);
        try_update_state_params(deps.as_mut(), info, new_params.clone()).unwrap();

        let new_state = storage::contract_settings_read(deps.as_ref().storage)
            .load()
            .unwrap();
        let expected_mixnode_bond =
            calculate_epoch_reward_rate(new_epoch_length, new_params.mixnode_bond_reward_rate);
        let expected_mixnode_delegation = calculate_epoch_reward_rate(
            new_epoch_length,
            new_params.mixnode_delegation_reward_rate,
        );
        assert_eq!(expected_mixnode_bond, new_state.mixnode_epoch_bond_reward);
        assert_eq!(
            expected_mixnode_delegation,
            new_state.mixnode_epoch_delegation_reward
        );

        // error is thrown if rewarded set is smaller than the active set
        let info = mock_info("creator", &[]);
        let mut new_params = current_state.params.clone();
        new_params.mixnode_rewarded_set_size = new_params.mixnode_active_set_size - 1;
        let res = try_update_state_params(deps.as_mut(), info, new_params.clone());
        assert_eq!(Err(ContractError::InvalidActiveSetSize), res)
    }
}
