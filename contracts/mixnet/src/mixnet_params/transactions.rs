use crate::error::ContractError;
use crate::helpers::calculate_epoch_reward_rate;
use crate::storage::config_read;
use cosmwasm_std::Decimal;
use cosmwasm_std::DepsMut;
use cosmwasm_std::MessageInfo;
use cosmwasm_std::Response;
use mixnet_contract::StateParams;

pub(crate) fn try_update_state_params(
    deps: DepsMut,
    info: MessageInfo,
    params: StateParams,
) -> Result<Response, ContractError> {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    let mut state = config_read(deps.storage).load()?;

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

    crate::storage::config(deps.storage).save(&state)?;

    Ok(Response::default())
}
