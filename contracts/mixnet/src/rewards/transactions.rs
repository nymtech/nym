use crate::error::ContractError;
use crate::helpers::scale_reward_by_uptime;
use crate::storage;
use crate::storage::{
    config_read, decr_reward_pool, increase_mix_delegated_stakes, increase_mix_delegated_stakes_v2,
    mixnodes, mixnodes_read, rewarded_mixnodes, rewarded_mixnodes_read,
};
use crate::transactions::{MAX_REWARDING_DURATION_IN_BLOCKS, MINIMUM_BLOCK_AGE_FOR_REWARDING};
use cosmwasm_std::{attr, DepsMut, Env, MessageInfo, Response, Uint128};
use mixnet_contract::mixnode::NodeRewardParams;
use mixnet_contract::IdentityKey;

// Note: this function is designed to work with only a single validator entity distributing rewards
// The main purpose of this function is to update `latest_rewarding_interval_nonce` which
// will trigger a different seed selection for the pseudorandom generation of the "demanded" set of mixnodes.
pub(crate) fn try_begin_mixnode_rewarding(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let mut state = config_read(deps.storage).load()?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    // check whether sufficient number of blocks already elapsed since the previous rewarding happened
    // (this implies the validator responsible for rewarding in the previous interval did not call
    // `try_finish_mixnode_rewarding` - perhaps they crashed or something. Regardless of the reason
    // it shouldn't prevent anyone from distributing rewards in the following interval)
    // Do note, however, that calling `try_finish_mixnode_rewarding` is crucial as otherwise the
    // "demanded" set won't get updated on the validator API side
    if state.rewarding_in_progress
        && state.rewarding_interval_starting_block + MAX_REWARDING_DURATION_IN_BLOCKS
            > env.block.height
    {
        return Err(ContractError::RewardingInProgress);
    }

    // make sure the validator is in sync with the contract state
    if rewarding_interval_nonce != state.latest_rewarding_interval_nonce + 1 {
        return Err(ContractError::InvalidRewardingIntervalNonce {
            received: rewarding_interval_nonce,
            expected: state.latest_rewarding_interval_nonce + 1,
        });
    }

    state.rewarding_interval_starting_block = env.block.height;
    state.latest_rewarding_interval_nonce = rewarding_interval_nonce;
    state.rewarding_in_progress = true;

    storage::config(deps.storage).save(&state)?;

    let mut response = Response::new();
    response.add_attribute(
        "rewarding interval nonce",
        rewarding_interval_nonce.to_string(),
    );
    Ok(response)
}

// Note: if any changes are made to this function or anything it is calling down the stack,
// for example delegation reward distribution, the gas limits must be retested and both
// validator-api/src/rewarding/bonding_mixnodes::{MIXNODE_REWARD_OP_BASE_GAS_LIMIT, PER_MIXNODE_DELEGATION_GAS_INCREASE}
// must be updated appropriately.
pub(crate) fn try_reward_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
    uptime: u32,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let state = config_read(deps.storage).load()?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    if !state.rewarding_in_progress {
        return Err(ContractError::RewardingNotInProgress);
    }

    // make sure the transaction is sent for the correct rewarding interval
    if rewarding_interval_nonce != state.latest_rewarding_interval_nonce {
        return Err(ContractError::InvalidRewardingIntervalNonce {
            received: rewarding_interval_nonce,
            expected: state.latest_rewarding_interval_nonce,
        });
    }

    // check if the mixnode hasn't been rewarded in this rewarding interval already
    if rewarded_mixnodes_read(deps.storage, rewarding_interval_nonce)
        .may_load(mix_identity.as_bytes())?
        .is_some()
    {
        return Err(ContractError::MixnodeAlreadyRewarded {
            identity: mix_identity,
        });
    }

    // optimisation for uptime being 0. No rewards will be given so just terminate here
    if uptime == 0 {
        rewarded_mixnodes(deps.storage, rewarding_interval_nonce)
            .save(mix_identity.as_bytes(), &Default::default())?;
        return Ok(Response {
            submessages: vec![],
            messages: vec![],
            attributes: vec![
                attr("bond increase", Uint128(0)),
                attr("total delegation increase", Uint128(0)),
            ],
            data: None,
        });
    }

    // check if the bond even exists
    let mut current_bond = match mixnodes_read(deps.storage).load(mix_identity.as_bytes()) {
        Ok(bond) => bond,
        Err(_) => {
            return Ok(Response {
                attributes: vec![attr("result", "bond not found")],
                ..Default::default()
            });
        }
    };

    let mut node_reward = Uint128(0);
    let mut total_delegation_reward = Uint128(0);

    // update current bond with the reward given to the node and the delegators
    // if it has been bonded for long enough
    if current_bond.block_height + MINIMUM_BLOCK_AGE_FOR_REWARDING <= env.block.height {
        let bond_reward_rate = state.mixnode_epoch_bond_reward;
        let delegation_reward_rate = state.mixnode_epoch_delegation_reward;
        let bond_scaled_reward_rate = scale_reward_by_uptime(bond_reward_rate, uptime)?;
        let delegation_scaled_reward_rate = scale_reward_by_uptime(delegation_reward_rate, uptime)?;

        total_delegation_reward = increase_mix_delegated_stakes(
            deps.storage,
            &mix_identity,
            delegation_scaled_reward_rate,
            env.block.height,
        )?;

        node_reward = current_bond.bond_amount.amount * bond_scaled_reward_rate;
        current_bond.bond_amount.amount += node_reward;
        current_bond.total_delegation.amount += total_delegation_reward;
        mixnodes(deps.storage).save(mix_identity.as_bytes(), &current_bond)?;
    }

    rewarded_mixnodes(deps.storage, rewarding_interval_nonce)
        .save(mix_identity.as_bytes(), &Default::default())?;

    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("bond increase", node_reward),
            attr("total delegation increase", total_delegation_reward),
        ],
        data: None,
    })
}

pub(crate) fn try_reward_mixnode_v2(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
    params: NodeRewardParams,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let state = config_read(deps.storage).load()?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    if !state.rewarding_in_progress {
        return Err(ContractError::RewardingNotInProgress);
    }

    // make sure the transaction is sent for the correct rewarding interval
    if rewarding_interval_nonce != state.latest_rewarding_interval_nonce {
        return Err(ContractError::InvalidRewardingIntervalNonce {
            received: rewarding_interval_nonce,
            expected: state.latest_rewarding_interval_nonce,
        });
    }

    // check if the mixnode hasn't been rewarded in this rewarding interval already
    if rewarded_mixnodes_read(deps.storage, rewarding_interval_nonce)
        .may_load(mix_identity.as_bytes())?
        .is_some()
    {
        return Err(ContractError::MixnodeAlreadyRewarded {
            identity: mix_identity,
        });
    }

    // check if the bond even exists
    let mut current_bond = match mixnodes_read(deps.storage).load(mix_identity.as_bytes()) {
        Ok(bond) => bond,
        Err(_) => {
            return Ok(Response {
                attributes: vec![attr("result", "bond not found")],
                ..Default::default()
            });
        }
    };

    let mut reward_params = params;

    reward_params.set_reward_blockstamp(env.block.height);

    let reward_result = current_bond.reward(&reward_params);

    // Omitting the price per packet function now, it follows that base operator reward is the node_reward
    let operator_reward = current_bond.operator_reward(&reward_params);

    let total_delegation_reward =
        increase_mix_delegated_stakes_v2(deps.storage, &current_bond, &reward_params)?;

    // update current bond with the reward given to the node and the delegators
    // if it has been bonded for long enough
    if current_bond.block_height + MINIMUM_BLOCK_AGE_FOR_REWARDING
        <= reward_params.reward_blockstamp()
    {
        current_bond.bond_amount.amount += Uint128(operator_reward);
        current_bond.total_delegation.amount += total_delegation_reward;
        mixnodes(deps.storage).save(mix_identity.as_bytes(), &current_bond)?;
        decr_reward_pool(Uint128(operator_reward), deps.storage)?;
        decr_reward_pool(total_delegation_reward, deps.storage)?;
    }

    rewarded_mixnodes(deps.storage, rewarding_interval_nonce)
        .save(mix_identity.as_bytes(), &Default::default())?;

    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("bond increase", reward_result.reward()),
            attr("total delegation increase", total_delegation_reward),
        ],
        data: None,
    })
}
pub(crate) fn try_finish_mixnode_rewarding(
    deps: DepsMut,
    info: MessageInfo,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let mut state = config_read(deps.storage).load()?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    if !state.rewarding_in_progress {
        return Err(ContractError::RewardingNotInProgress);
    }

    // make sure the validator is in sync with the contract state
    if rewarding_interval_nonce != state.latest_rewarding_interval_nonce {
        return Err(ContractError::InvalidRewardingIntervalNonce {
            received: rewarding_interval_nonce,
            expected: state.latest_rewarding_interval_nonce,
        });
    }

    state.rewarding_in_progress = false;
    storage::config(deps.storage).save(&state)?;

    Ok(Response::new())
}
