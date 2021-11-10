// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use crate::error::ContractError;
use crate::helpers::{calculate_epoch_reward_rate, scale_reward_by_uptime, Delegations};
use crate::queries;
use crate::storage::*;
use config::defaults::DENOM;
use cosmwasm_std::{
    attr, coins, BankMsg, Coin, Decimal, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cosmwasm_storage::ReadonlyBucket;
use mixnet_contract::mixnode::NodeRewardParams;
use mixnet_contract::{
    Gateway, GatewayBond, IdentityKey, Layer, MixNode, MixNodeBond, RawDelegationData, StateParams,
};

pub(crate) const OLD_DELEGATIONS_CHUNK_SIZE: usize = 500;

// approximately 1 day (assuming 5s per block)
pub(crate) const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 17280;

// approximately 30min (assuming 5s per block)
pub(crate) const MAX_REWARDING_DURATION_IN_BLOCKS: u64 = 360;

fn total_delegations(delegations_bucket: ReadonlyBucket<RawDelegationData>) -> StdResult<Coin> {
    Ok(Coin::new(
        Delegations::new(delegations_bucket)
            .fold(0, |acc, x| acc + x.delegation_data.amount.u128()),
        DENOM,
    ))
}

fn validate_mixnode_bond(bond: &[Coin], minimum_bond: Uint128) -> Result<(), ContractError> {
    // check if anything was put as bond
    if bond.is_empty() {
        return Err(ContractError::NoBondFound);
    }

    if bond.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if bond[0].denom != DENOM {
        return Err(ContractError::WrongDenom {});
    }

    // check that we have at least MIXNODE_BOND coins in our bond
    if bond[0].amount < minimum_bond {
        return Err(ContractError::InsufficientMixNodeBond {
            received: bond[0].amount.into(),
            minimum: minimum_bond.into(),
        });
    }

    Ok(())
}

pub(crate) fn try_add_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_node: MixNode,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_bytes();

    // if the client has an active bonded gateway, don't allow mixnode bonding
    if gateways_owners_read(deps.storage)
        .may_load(sender_bytes)?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsGateway);
    }

    let mut was_present = false;
    // if the client has an active mixnode with a different identity, don't allow bonding
    if let Some(existing_node) = mixnodes_owners_read(deps.storage).may_load(sender_bytes)? {
        if existing_node != mix_node.identity_key {
            return Err(ContractError::AlreadyOwnsMixnode);
        }
        was_present = true
    }

    // check if somebody else has already bonded a mixnode with this identity
    if let Some(existing_bond) =
        mixnodes_read(deps.storage).may_load(mix_node.identity_key.as_bytes())?
    {
        if existing_bond.owner != info.sender {
            return Err(ContractError::DuplicateMixnode {
                owner: existing_bond.owner,
            });
        }
    }

    let minimum_bond = read_state_params(deps.storage).minimum_mixnode_bond;
    validate_mixnode_bond(&info.funds, minimum_bond)?;

    let layer_distribution = queries::query_layer_distribution(deps.as_ref());
    let layer = layer_distribution.choose_with_fewest();

    let mut bond = MixNodeBond::new(
        info.funds[0].clone(),
        info.sender.clone(),
        layer,
        env.block.height,
        mix_node,
        None,
    );

    // this might potentially require more gas if a significant number of delegations was there
    let delegations_bucket = mix_delegations_read(deps.storage, &bond.mix_node.identity_key);
    let existing_delegation = total_delegations(delegations_bucket)?;
    bond.total_delegation = existing_delegation;

    let identity = bond.identity();

    mixnodes(deps.storage).save(identity.as_bytes(), &bond)?;
    mixnodes_owners(deps.storage).save(sender_bytes, identity)?;
    increment_layer_count(deps.storage, bond.layer)?;

    let attributes = vec![attr("overwritten", was_present)];
    Ok(Response {
        submessages: Vec::new(),
        messages: Vec::new(),
        attributes,
        data: None,
    })
}

pub(crate) fn try_remove_mixnode(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_bytes();

    // try to find the identity of the sender's node
    let mix_identity = match mixnodes_owners_read(deps.storage).may_load(sender_bytes)? {
        Some(identity) => identity,
        None => return Err(ContractError::NoAssociatedMixNodeBond { owner: info.sender }),
    };

    // get the bond, since we found associated identity, the node MUST exist
    let mixnode_bond = mixnodes_read(deps.storage).load(mix_identity.as_bytes())?;

    // send bonded funds back to the bond owner
    let messages = vec![BankMsg::Send {
        to_address: info.sender.as_str().to_owned(),
        amount: vec![mixnode_bond.bond_amount()],
    }
    .into()];

    // remove the bond from the list of bonded mixnodes
    mixnodes(deps.storage).remove(mix_identity.as_bytes());
    // remove the node ownership
    mixnodes_owners(deps.storage).remove(sender_bytes);
    // decrement layer count
    decrement_layer_count(deps.storage, mixnode_bond.layer)?;

    // log our actions
    let attributes = vec![attr("action", "unbond"), attr("mixnode_bond", mixnode_bond)];

    Ok(Response {
        submessages: Vec::new(),
        messages,
        attributes,
        data: None,
    })
}

fn validate_gateway_bond(bond: &[Coin], minimum_bond: Uint128) -> Result<(), ContractError> {
    // check if anything was put as bond
    if bond.is_empty() {
        return Err(ContractError::NoBondFound);
    }

    if bond.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if bond[0].denom != DENOM {
        return Err(ContractError::WrongDenom {});
    }

    // check that we have at least 100 coins in our bond
    if bond[0].amount < minimum_bond {
        return Err(ContractError::InsufficientGatewayBond {
            received: bond[0].amount.into(),
            minimum: minimum_bond.into(),
        });
    }

    Ok(())
}

pub(crate) fn try_add_gateway(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    gateway: Gateway,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_bytes();

    // if the client has an active bonded mixnode, don't allow gateway bonding
    if mixnodes_owners_read(deps.storage)
        .may_load(sender_bytes)?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsMixnode);
    }

    let mut was_present = false;
    // if the client has an active gateway with a different identity, don't allow bonding
    if let Some(existing_node) = gateways_owners_read(deps.storage).may_load(sender_bytes)? {
        if existing_node != gateway.identity_key {
            return Err(ContractError::AlreadyOwnsGateway);
        }
        was_present = true
    }

    // check if somebody else has already bonded a gateway with this identity
    if let Some(existing_bond) =
        gateways_read(deps.storage).may_load(gateway.identity_key.as_bytes())?
    {
        if existing_bond.owner != info.sender {
            return Err(ContractError::DuplicateGateway {
                owner: existing_bond.owner,
            });
        }
    }

    let minimum_bond = read_state_params(deps.storage).minimum_gateway_bond;
    validate_gateway_bond(&info.funds, minimum_bond)?;

    let bond = GatewayBond::new(
        info.funds[0].clone(),
        info.sender.clone(),
        env.block.height,
        gateway,
    );

    let identity = bond.identity();
    gateways(deps.storage).save(identity.as_bytes(), &bond)?;
    gateways_owners(deps.storage).save(sender_bytes, identity)?;
    increment_layer_count(deps.storage, Layer::Gateway)?;

    let attributes = vec![attr("overwritten", was_present)];
    Ok(Response {
        submessages: Vec::new(),
        messages: Vec::new(),
        attributes,
        data: None,
    })
}

pub(crate) fn try_remove_gateway(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_str().as_bytes();

    // try to find the identity of the sender's node
    let gateway_identity = match gateways_owners_read(deps.storage).may_load(sender_bytes)? {
        Some(identity) => identity,
        None => return Err(ContractError::NoAssociatedGatewayBond { owner: info.sender }),
    };

    // get the bond, since we found associated identity, the node MUST exist
    let gateway_bond = gateways_read(deps.storage).load(gateway_identity.as_bytes())?;

    // send bonded funds back to the bond owner
    let messages = vec![BankMsg::Send {
        to_address: info.sender.as_str().to_owned(),
        amount: vec![gateway_bond.bond_amount()],
    }
    .into()];

    // remove the bond from the list of bonded gateways
    gateways(deps.storage).remove(gateway_identity.as_bytes());
    // remove the node ownership
    gateways_owners(deps.storage).remove(sender_bytes);
    // decrement layer count
    decrement_layer_count(deps.storage, Layer::Gateway)?;

    // log our actions
    let attributes = vec![
        attr("action", "unbond"),
        attr("address", info.sender),
        attr("gateway_bond", gateway_bond),
    ];

    Ok(Response {
        submessages: Vec::new(),
        messages,
        attributes,
        data: None,
    })
}

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

    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

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

    config(deps.storage).save(&state)?;

    let mut response = Response::new();
    response.add_attribute(
        "rewarding interval nonce",
        rewarding_interval_nonce.to_string(),
    );
    Ok(response)
}

// Note: if any changes are made to this function or anything it is calling down the stack,
// for example delegation reward distribution, the gas limits must be retested and both
// validator-api/src/rewarding/mod.rs::{MIXNODE_REWARD_OP_BASE_GAS_LIMIT, PER_MIXNODE_DELEGATION_GAS_INCREASE}
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
    config(deps.storage).save(&state)?;

    Ok(Response::new())
}

fn validate_delegation_stake(delegation: &[Coin]) -> Result<(), ContractError> {
    // check if anything was put as delegation
    if delegation.is_empty() {
        return Err(ContractError::EmptyDelegation);
    }

    if delegation.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if delegation[0].denom != DENOM {
        return Err(ContractError::WrongDenom {});
    }

    // check that we have provided a non-zero amount in the delegation
    if delegation[0].amount.is_zero() {
        return Err(ContractError::EmptyDelegation);
    }

    Ok(())
}

pub(crate) fn try_delegate_to_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    // check if the delegation contains any funds of the appropriate denomination
    validate_delegation_stake(&info.funds)?;

    // check if the target node actually exists
    let mut current_bond = match mixnodes_read(deps.storage).load(mix_identity.as_bytes()) {
        Ok(bond) => bond,
        Err(_) => {
            return Err(ContractError::MixNodeBondNotFound {
                identity: mix_identity,
            });
        }
    };

    let amount = info.funds[0].amount;

    // update total_delegation of this node
    current_bond.total_delegation.amount += info.funds[0].amount;
    mixnodes(deps.storage).save(mix_identity.as_bytes(), &current_bond)?;

    let mut delegation_bucket = mix_delegations(deps.storage, &mix_identity);
    let sender_bytes = info.sender.as_bytes();

    // write the delegation
    let new_amount = match delegation_bucket.may_load(sender_bytes)? {
        Some(existing_delegation) => existing_delegation.amount + amount,
        None => amount,
    };
    // the block height is reset, if it existed
    let new_delegation = RawDelegationData::new(new_amount, env.block.height);
    delegation_bucket.save(sender_bytes, &new_delegation)?;

    reverse_mix_delegations(deps.storage, &info.sender).save(mix_identity.as_bytes(), &())?;

    Ok(Response::default())
}

pub(crate) fn try_remove_delegation_from_mixnode(
    deps: DepsMut,
    info: MessageInfo,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    let mut delegation_bucket = mix_delegations(deps.storage, &mix_identity);
    let sender_bytes = info.sender.as_bytes();
    match delegation_bucket.may_load(sender_bytes)? {
        Some(delegation) => {
            // remove delegation from the buckets
            delegation_bucket.remove(sender_bytes);
            reverse_mix_delegations(deps.storage, &info.sender).remove(mix_identity.as_bytes());

            // send delegated funds back to the delegation owner
            let messages = vec![BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: coins(delegation.amount.u128(), DENOM),
            }
            .into()];

            // update total_delegation of this node
            let mut mixnodes_bucket = mixnodes(deps.storage);
            // in some rare cases the mixnode bond might no longer exist as the node unbonded
            // before delegation was removed. that is fine
            if let Some(mut existing_bond) = mixnodes_bucket.may_load(mix_identity.as_bytes())? {
                // we should NEVER underflow here, if we do, it means we have some serious error in our logic
                existing_bond.total_delegation.amount = existing_bond
                    .total_delegation
                    .amount
                    .checked_sub(delegation.amount)
                    .unwrap();
                mixnodes_bucket.save(mix_identity.as_bytes(), &existing_bond)?;
            }

            Ok(Response {
                submessages: Vec::new(),
                messages,
                attributes: Vec::new(),
                data: None,
            })
        }
        None => Err(ContractError::NoMixnodeDelegationFound {
            identity: mix_identity,
            address: info.sender,
        }),
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::{
        execute, query, DEFAULT_SYBIL_RESISTANCE_PERCENT, INITIAL_DEFAULT_EPOCH_LENGTH,
        INITIAL_GATEWAY_BOND, INITIAL_MIXNODE_BOND, INITIAL_MIXNODE_BOND_REWARD_RATE,
        INITIAL_MIXNODE_DELEGATION_REWARD_RATE,
    };
    use crate::helpers::calculate_epoch_reward_rate;
    use crate::queries::DELEGATION_PAGE_DEFAULT_LIMIT;
    use crate::storage::{layer_distribution_read, mix_delegations_read, read_mixnode_bond};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{
        add_mixnode, good_gateway_bond, good_mixnode_bond, mix_node_fixture, raw_delegation_fixture,
    };
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, Addr, Uint128};
    use mixnet_contract::{
        ExecuteMsg, LayerDistribution, PagedGatewayResponse, PagedMixnodeResponse, QueryMsg,
        UnpackedDelegation,
    };
    use queries::tests::store_n_mix_delegations;

    #[test]
    fn validating_mixnode_bond() {
        // you must send SOME funds
        let result = validate_mixnode_bond(&[], INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::NoBondFound));

        // you must send at least 100 coins...
        let mut bond = good_mixnode_bond();
        bond[0].amount = INITIAL_MIXNODE_BOND.checked_sub(Uint128(1)).unwrap();
        let result = validate_mixnode_bond(&bond, INITIAL_MIXNODE_BOND);
        assert_eq!(
            result,
            Err(ContractError::InsufficientMixNodeBond {
                received: Into::<u128>::into(INITIAL_MIXNODE_BOND) - 1,
                minimum: INITIAL_MIXNODE_BOND.into(),
            })
        );

        // more than that is still fine
        let mut bond = good_mixnode_bond();
        bond[0].amount = INITIAL_MIXNODE_BOND + Uint128(1);
        let result = validate_mixnode_bond(&bond, INITIAL_MIXNODE_BOND);
        assert!(result.is_ok());

        // it must be sent in the defined denom!
        let mut bond = good_mixnode_bond();
        bond[0].denom = "baddenom".to_string();
        let result = validate_mixnode_bond(&bond, INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));

        let mut bond = good_mixnode_bond();
        bond[0].denom = "foomp".to_string();
        let result = validate_mixnode_bond(&bond, INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));
    }

    #[test]
    fn mixnode_add() {
        let mut deps = helpers::init_contract();

        // if we don't send enough funds
        let insufficient_bond = Into::<u128>::into(INITIAL_MIXNODE_BOND) - 1;
        let info = mock_info("anyone", &coins(insufficient_bond, DENOM));
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "anyonesmixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };

        // we are informed that we didn't send enough funds
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::InsufficientMixNodeBond {
                received: insufficient_bond,
                minimum: INITIAL_MIXNODE_BOND.into(),
            })
        );

        // no mixnode was inserted into the topology
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedMixnodeResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len());

        // if we send enough funds
        let info = mock_info("anyone", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "anyonesmixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };

        // we get back a message telling us everything was OK
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // we can query topology and the new node is there
        let query_response = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedMixnodeResponse = from_binary(&query_response).unwrap();
        assert_eq!(1, page.nodes.len());
        assert_eq!(
            &MixNode {
                identity_key: "anyonesmixnode".into(),
                ..helpers::mix_node_fixture()
            },
            page.nodes[0].mix_node()
        );

        // if there was already a mixnode bonded by particular user
        let info = mock_info("foomper", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "foompermixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(execute_response.attributes[0], attr("overwritten", false));

        let info = mock_info("foomper", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "foompermixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };

        // we get a log message about it (TODO: does it get back to the user?)
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(execute_response.attributes[0], attr("overwritten", true));

        // bonding fails if the user already owns a gateway
        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "ownersgateway".into(),
                ..helpers::gateway_fixture()
            },
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("gateway-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "ownersmixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(execute_response, Err(ContractError::AlreadyOwnsGateway));

        // but after he unbonds it, it's all fine again
        let info = mock_info("gateway-owner", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("gateway-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "ownersmixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // adding another node from another account, but with the same IP, should fail (or we would have a weird state). Is that right? Think about this, not sure yet.
        // if we attempt to register a second node from the same address, should we get an error? It would probably be polite.
    }

    #[test]
    fn adding_mixnode_without_existing_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        // before the execution the node had no associated owner
        assert!(mixnodes_owners_read(deps.as_ref().storage)
            .may_load("myAwesomeMixnode".as_bytes())
            .unwrap()
            .is_none());

        // it's all fine, owner is saved
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        assert_eq!(
            "myAwesomeMixnode",
            mixnodes_owners_read(deps.as_ref().storage)
                .load("mix-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn adding_mixnode_with_existing_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // request fails giving the existing owner address in the message
        let info = mock_info("mix-owner-pretender", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            Err(ContractError::DuplicateMixnode {
                owner: Addr::unchecked("mix-owner")
            }),
            execute_response
        );
    }

    #[test]
    fn adding_mixnode_with_existing_unchanged_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                host: "1.1.1.1:1789".into(),
                ..helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                host: "2.2.2.2:1789".into(),
                ..helpers::mix_node_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        // make sure the host information was updated
        assert_eq!(
            "2.2.2.2:1789".to_string(),
            mixnodes_read(deps.as_ref().storage)
                .load("myAwesomeMixnode".as_bytes())
                .unwrap()
                .mix_node
                .host
        );
    }

    #[test]
    fn adding_mixnode_updates_layer_distribution() {
        let mut deps = helpers::init_contract();

        assert_eq!(
            LayerDistribution::default(),
            layer_distribution_read(&deps.storage).load().unwrap(),
        );

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "mix1".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            LayerDistribution {
                layer1: 1,
                ..Default::default()
            },
            layer_distribution_read(&deps.storage).load().unwrap()
        );
    }

    #[test]
    fn mixnode_remove() {
        let mut deps = helpers::init_contract();

        // try un-registering when no nodes exist yet
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);

        // we're told that there is no node for our address
        assert_eq!(
            result,
            Err(ContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked("anyone")
            })
        );

        // let's add a node owned by bob
        helpers::add_mixnode("bob", good_mixnode_bond(), &mut deps);

        // attempt to un-register fred's node, which doesn't exist
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked("fred")
            })
        );

        // bob's node is still there
        let nodes = helpers::get_mix_nodes(&mut deps);
        assert_eq!(1, nodes.len());
        assert_eq!("bob", nodes[0].owner().clone());

        // add a node owned by fred
        let info = mock_info("fred", &good_mixnode_bond());
        try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            info,
            MixNode {
                identity_key: "fredsmixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        // let's make sure we now have 2 nodes:
        assert_eq!(2, helpers::get_mix_nodes(&mut deps).len());

        // un-register fred's node
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        let remove_fred = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // we should see log messages come back showing an unbond message
        let expected_attributes = vec![
            attr("action", "unbond"),
            attr(
                "mixnode_bond",
                format!(
                    "amount: {} {}, owner: fred, identity: fredsmixnode",
                    INITIAL_MIXNODE_BOND, DENOM
                ),
            ),
        ];

        // we should see a funds transfer from the contract back to fred
        let expected_messages = vec![BankMsg::Send {
            to_address: String::from(info.sender),
            amount: good_mixnode_bond(),
        }
        .into()];

        // run the executer and check that we got back the correct results
        let expected = Response {
            submessages: Vec::new(),
            messages: expected_messages,
            attributes: expected_attributes,
            data: None,
        };
        assert_eq!(remove_fred, expected);

        // only 1 node now exists, owned by bob:
        let mix_node_bonds = helpers::get_mix_nodes(&mut deps);
        assert_eq!(1, mix_node_bonds.len());
        assert_eq!(&Addr::unchecked("bob"), mix_node_bonds[0].owner());
    }

    #[test]
    fn removing_mixnode_clears_ownership() {
        let mut deps = helpers::init_contract();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            "myAwesomeMixnode",
            mixnodes_owners_read(deps.as_ref().storage)
                .load("mix-owner".as_bytes())
                .unwrap()
        );

        let info = mock_info("mix-owner", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        assert!(mixnodes_owners_read(deps.as_ref().storage)
            .may_load("mix-owner".as_bytes())
            .unwrap()
            .is_none());

        // and since it's removed, it can be reclaimed
        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());
        assert_eq!(
            "myAwesomeMixnode",
            mixnodes_owners_read(deps.as_ref().storage)
                .load("mix-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn validating_gateway_bond() {
        // you must send SOME funds
        let result = validate_gateway_bond(&[], INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::NoBondFound));

        // you must send at least 100 coins...
        let mut bond = good_gateway_bond();
        bond[0].amount = INITIAL_GATEWAY_BOND.checked_sub(Uint128(1)).unwrap();
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert_eq!(
            result,
            Err(ContractError::InsufficientGatewayBond {
                received: Into::<u128>::into(INITIAL_GATEWAY_BOND) - 1,
                minimum: INITIAL_GATEWAY_BOND.into(),
            })
        );

        // more than that is still fine
        let mut bond = good_gateway_bond();
        bond[0].amount = INITIAL_GATEWAY_BOND + Uint128(1);
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert!(result.is_ok());

        // it must be sent in the defined denom!
        let mut bond = good_gateway_bond();
        bond[0].denom = "baddenom".to_string();
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));

        let mut bond = good_gateway_bond();
        bond[0].denom = "foomp".to_string();
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));
    }

    #[test]
    fn gateway_add() {
        let mut deps = helpers::init_contract();

        // if we fail validation (by say not sending enough funds
        let insufficient_bond = Into::<u128>::into(INITIAL_GATEWAY_BOND) - 1;
        let info = mock_info("anyone", &coins(insufficient_bond, DENOM));
        let msg = ExecuteMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };

        // we are informed that we didn't send enough funds
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::InsufficientGatewayBond {
                received: insufficient_bond,
                minimum: INITIAL_GATEWAY_BOND.into(),
            })
        );

        // make sure no gateway was inserted into the topology
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetGateways {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedGatewayResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len());

        // if we send enough funds
        let info = mock_info("anyone", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "anyonesgateway".into(),
                ..helpers::gateway_fixture()
            },
        };

        // we get back a message telling us everything was OK
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // we can query topology and the new node is there
        let query_response = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetGateways {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedGatewayResponse = from_binary(&query_response).unwrap();
        assert_eq!(1, page.nodes.len());
        assert_eq!(
            &Gateway {
                identity_key: "anyonesgateway".into(),
                ..helpers::gateway_fixture()
            },
            page.nodes[0].gateway()
        );

        // if there was already a gateway bonded by particular user
        let info = mock_info("foomper", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "foompersgateway".into(),
                ..helpers::gateway_fixture()
            },
        };

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(execute_response.attributes[0], attr("overwritten", false));

        let info = mock_info("foomper", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "foompersgateway".into(),
                ..helpers::gateway_fixture()
            },
        };

        // we get a log message about it (TODO: does it get back to the user?)
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(execute_response.attributes[0], attr("overwritten", true));

        // bonding fails if the user already owns a mixnode
        let info = mock_info("mixnode-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "ownersmix".into(),
                ..helpers::mix_node_fixture()
            },
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mixnode-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(execute_response, Err(ContractError::AlreadyOwnsMixnode));

        // but after he unbonds it, it's all fine again
        let info = mock_info("mixnode-owner", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mixnode-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // adding another node from another account, but with the same IP, should fail (or we would have a weird state).
        // Is that right? Think about this, not sure yet.
    }

    #[test]
    fn adding_gateway_without_existing_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
            },
        };

        // before the execution the node had no associated owner
        assert!(gateways_owners_read(deps.as_ref().storage)
            .may_load("gateway-owner".as_bytes())
            .unwrap()
            .is_none());

        // it's all fine, owner is saved
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        assert_eq!(
            "myAwesomeGateway",
            gateways_owners_read(deps.as_ref().storage)
                .load("gateway-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn adding_gateway_with_existing_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // request fails giving the existing owner address in the message
        let info = mock_info("gateway-owner-pretender", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
            },
        };

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            Err(ContractError::DuplicateGateway {
                owner: Addr::unchecked("gateway-owner")
            }),
            execute_response
        );
    }

    #[test]
    fn adding_gateway_with_existing_unchanged_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                host: "1.1.1.1".into(),
                ..helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                host: "2.2.2.2".into(),
                ..helpers::gateway_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        // make sure the host information was updated
        assert_eq!(
            "2.2.2.2".to_string(),
            gateways_read(deps.as_ref().storage)
                .load("myAwesomeGateway".as_bytes())
                .unwrap()
                .gateway
                .host
        );
    }

    #[test]
    fn gateway_remove() {
        let mut deps = helpers::init_contract();

        // try unbond when no nodes exist yet
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);

        // we're told that there is no node for our address
        assert_eq!(
            result,
            Err(ContractError::NoAssociatedGatewayBond {
                owner: Addr::unchecked("anyone")
            })
        );

        // let's add a node owned by bob
        helpers::add_gateway("bob", good_gateway_bond(), &mut deps);

        // attempt to unbond fred's node, which doesn't exist
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::NoAssociatedGatewayBond {
                owner: Addr::unchecked("fred")
            })
        );

        // bob's node is still there
        let nodes = helpers::get_gateways(&mut deps);
        assert_eq!(1, nodes.len());

        let first_node = &nodes[0];
        assert_eq!(&Addr::unchecked("bob"), first_node.owner());

        // add a node owned by fred
        let info = mock_info("fred", &good_gateway_bond());
        try_add_gateway(
            deps.as_mut(),
            mock_env(),
            info,
            Gateway {
                identity_key: "fredsgateway".into(),
                ..helpers::gateway_fixture()
            },
        )
        .unwrap();

        // let's make sure we now have 2 nodes:
        assert_eq!(2, helpers::get_gateways(&mut deps).len());

        // unbond fred's node
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let remove_fred = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // we should see log messages come back showing an unbond message
        let expected_attributes = vec![
            attr("action", "unbond"),
            attr("address", "fred"),
            attr(
                "gateway_bond",
                format!(
                    "amount: {} {}, owner: fred, identity: fredsgateway",
                    INITIAL_GATEWAY_BOND, DENOM
                ),
            ),
        ];

        // we should see a funds transfer from the contract back to fred
        let expected_messages = vec![BankMsg::Send {
            to_address: String::from(info.sender),
            amount: good_gateway_bond(),
        }
        .into()];

        // run the executer and check that we got back the correct results
        let expected = Response {
            submessages: Vec::new(),
            messages: expected_messages,
            attributes: expected_attributes,
            data: None,
        };
        assert_eq!(remove_fred, expected);

        // only 1 node now exists, owned by bob:
        let gateway_bonds = helpers::get_gateways(&mut deps);
        assert_eq!(1, gateway_bonds.len());
        assert_eq!(&Addr::unchecked("bob"), gateway_bonds[0].owner());
    }

    #[test]
    fn removing_gateway_clears_ownership() {
        let mut deps = helpers::init_contract();

        let info = mock_info("gateway-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            "myAwesomeGateway",
            gateways_owners_read(deps.as_ref().storage)
                .load("gateway-owner".as_bytes())
                .unwrap()
        );

        let info = mock_info("gateway-owner", &[]);
        let msg = ExecuteMsg::UnbondGateway {};

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        assert!(gateways_owners_read(deps.as_ref().storage)
            .may_load("gateway-owner".as_bytes())
            .unwrap()
            .is_none());

        // and since it's removed, it can be reclaimed
        let info = mock_info("gateway-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());
        assert_eq!(
            "myAwesomeGateway",
            gateways_owners_read(deps.as_ref().storage)
                .load("gateway-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn updating_state_params() {
        let mut deps = helpers::init_contract();

        let new_params = StateParams {
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
        let current_state = config_read(deps.as_ref().storage).load().unwrap();
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

        let new_state = config_read(deps.as_ref().storage).load().unwrap();
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

        let new_state = config_read(deps.as_ref().storage).load().unwrap();
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

    #[cfg(test)]
    mod beginning_mixnode_rewarding {
        use super::*;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info("not-the-approved-validator", &[]),
                1,
            );
            assert_eq!(Err(ContractError::Unauthorized), res);

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn cannot_be_called_if_rewarding_is_already_in_progress_with_little_day() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                2,
            );
            assert_eq!(Err(ContractError::RewardingInProgress), res);
        }

        #[test]
        fn can_be_called_if_rewarding_is_in_progress_if_sufficient_number_of_blocks_elapsed() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let mut new_env = env.clone();

            new_env.block.height = env.block.height + MAX_REWARDING_DURATION_IN_BLOCKS;

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                new_env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                2,
            );
            assert!(res.is_ok());
        }

        #[test]
        fn provided_nonce_must_be_equal_the_current_plus_one() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let mut current_state = config_read(deps.as_mut().storage).load().unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            config(deps.as_mut().storage).save(&current_state).unwrap();

            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                11,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 11,
                    expected: 43
                }),
                res
            );

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                44,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 44,
                    expected: 43
                }),
                res
            );

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                42,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 42,
                    expected: 43
                }),
                res
            );

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                43,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn updates_contract_state() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let start_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = start_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let new_state = config_read(deps.as_mut().storage).load().unwrap();
            assert!(new_state.rewarding_in_progress);
            assert_eq!(
                new_state.rewarding_interval_starting_block,
                env.block.height
            );
            assert_eq!(
                start_state.latest_rewarding_interval_nonce + 1,
                new_state.latest_rewarding_interval_nonce
            );
        }
    }

    #[cfg(test)]
    mod finishing_mixnode_rewarding {
        use super::*;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info("not-the-approved-validator", &[]),
                1,
            );
            assert_eq!(Err(ContractError::Unauthorized), res);

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn cannot_be_called_if_rewarding_is_not_in_progress() {
            let mut deps = helpers::init_contract();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                0,
            );
            assert_eq!(Err(ContractError::RewardingNotInProgress), res);
        }

        #[test]
        fn provided_nonce_must_be_equal_the_current_one() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let mut current_state = config_read(deps.as_mut().storage).load().unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            config(deps.as_mut().storage).save(&current_state).unwrap();

            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                43,
            )
            .unwrap();

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                11,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 11,
                    expected: 43
                }),
                res
            );

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                44,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 44,
                    expected: 43
                }),
                res
            );

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                42,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 42,
                    expected: 43
                }),
                res
            );

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                43,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn updates_contract_state() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let new_state = config_read(deps.as_mut().storage).load().unwrap();
            assert!(!new_state.rewarding_in_progress);
        }
    }

    #[test]
    fn rewarding_mixnode() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        // errors out if executed by somebody else than network monitor
        let info = mock_info("not-the-monitor", &[]);
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            100,
            1,
        );
        assert_eq!(res, Err(ContractError::Unauthorized));

        // begin rewarding period
        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        // returns bond not found attribute if the target owner hasn't bonded any mixnodes
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            100,
            1,
        )
        .unwrap();
        assert_eq!(vec![attr("result", "bond not found")], res.attributes);

        let initial_bond = 100_000000;
        let initial_delegation = 200_000000;
        let mixnode_bond = MixNodeBond {
            bond_amount: coin(initial_bond, DENOM),
            total_delegation: coin(initial_delegation, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            block_height: env.block.height,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..mix_node_fixture()
            },
            profit_margin_percent: Some(10),
        };

        mixnodes(deps.as_mut().storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        mix_delegations(&mut deps.storage, &node_identity)
            .save(
                b"delegator",
                &RawDelegationData::new(initial_delegation.into(), env.block.height),
            )
            .unwrap();

        env.block.height += 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING;

        let bond_reward_rate = current_state.mixnode_epoch_bond_reward;
        let delegation_reward_rate = current_state.mixnode_epoch_delegation_reward;
        let expected_bond_reward = Uint128(initial_bond) * bond_reward_rate;
        let expected_delegation_reward = Uint128(initial_delegation) * delegation_reward_rate;

        // the node's bond and delegations are correctly increased and scaled by uptime
        // if node was 100% up, it will get full epoch reward
        let expected_bond = expected_bond_reward + Uint128(initial_bond);
        let expected_delegation = expected_delegation_reward + Uint128(initial_delegation);

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );

        // if node was 20% up, it will get 1/5th of epoch reward
        let scaled_bond_reward = scale_reward_by_uptime(bond_reward_rate, 20).unwrap();
        let scaled_delegation_reward = scale_reward_by_uptime(delegation_reward_rate, 20).unwrap();
        let expected_bond_reward = expected_bond * scaled_bond_reward;
        let expected_delegation_reward = expected_delegation * scaled_delegation_reward;
        let expected_bond = expected_bond_reward + expected_bond;
        let expected_delegation = expected_delegation_reward + expected_delegation;

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);

        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            20,
            2,
        )
        .unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );
    }

    #[test]
    fn rewarding_mixnodes_outside_rewarding_period() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(node_owner.as_ref(), &good_mixnode_bond()),
            MixNode {
                identity_key: node_identity.to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        );
        assert_eq!(Err(ContractError::RewardingNotInProgress), res);

        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            100,
            1,
        );
        assert!(res.is_ok())
    }

    #[test]
    fn rewarding_mixnodes_with_incorrect_rewarding_nonce() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(node_owner.as_ref(), &good_mixnode_bond()),
            MixNode {
                identity_key: node_identity.to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            0,
        );
        assert_eq!(
            Err(ContractError::InvalidRewardingIntervalNonce {
                received: 0,
                expected: 1
            }),
            res
        );

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            2,
        );
        assert_eq!(
            Err(ContractError::InvalidRewardingIntervalNonce {
                received: 2,
                expected: 1
            }),
            res
        );

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            100,
            1,
        );
        assert!(res.is_ok())
    }

    #[test]
    fn attempting_rewarding_mixnode_multiple_times_per_interval() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(node_owner.as_ref(), &good_mixnode_bond()),
            MixNode {
                identity_key: node_identity.to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

        // first reward goes through just fine
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        );
        assert!(res.is_ok());

        // but the other one fails
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        );
        assert_eq!(
            Err(ContractError::MixnodeAlreadyRewarded {
                identity: node_identity.clone()
            }),
            res
        );

        // but rewarding the same node in the following interval is fine again
        try_finish_mixnode_rewarding(deps.as_mut(), info.clone(), 1).unwrap();
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();

        let res = try_reward_mixnode(deps.as_mut(), env, info, node_identity.clone(), 100, 2);
        assert!(res.is_ok());
    }

    #[test]
    fn rewarding_mixnode_blockstamp_based() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        let initial_bond = 100_000000;
        let initial_delegation = 200_000000;
        let mixnode_bond = MixNodeBond {
            bond_amount: coin(initial_bond, DENOM),
            total_delegation: coin(initial_delegation, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            block_height: env.block.height,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..mix_node_fixture()
            },
            profit_margin_percent: Some(10),
        };

        mixnodes(deps.as_mut().storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        // delegation happens later, but not later enough
        env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;

        mix_delegations(&mut deps.storage, &node_identity)
            .save(
                b"delegator",
                &RawDelegationData::new(initial_delegation.into(), env.block.height),
            )
            .unwrap();

        let bond_reward_rate = current_state.mixnode_epoch_bond_reward;
        let delegation_reward_rate = current_state.mixnode_epoch_delegation_reward;
        let scaled_bond_reward = scale_reward_by_uptime(bond_reward_rate, 100).unwrap();
        let scaled_delegation_reward = scale_reward_by_uptime(delegation_reward_rate, 100).unwrap();

        // no reward is due
        let expected_bond_reward = Uint128(0);
        let expected_delegation_reward = Uint128(0);
        let expected_bond = expected_bond_reward + Uint128(initial_bond);
        let expected_delegation = expected_delegation_reward + Uint128(initial_delegation);

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );

        // reward can happen now, but only for bonded node
        env.block.height += 1;
        let expected_bond_reward = expected_bond * scaled_bond_reward;
        let expected_delegation_reward = Uint128(0);
        let expected_bond = expected_bond_reward + expected_bond;
        let expected_delegation = expected_delegation_reward + expected_delegation;

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            2,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 2).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );

        // reward happens now, both for node owner and delegators
        env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;
        let expected_bond_reward = expected_bond * scaled_bond_reward;
        let expected_delegation_reward = expected_delegation * scaled_delegation_reward;
        let expected_bond = expected_bond_reward + expected_bond;
        let expected_delegation = expected_delegation_reward + expected_delegation;

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 3).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            3,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 3).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );
    }

    #[cfg(test)]
    mod delegation_stake_validation {
        use super::*;
        use cosmwasm_std::coin;

        #[test]
        fn stake_cant_be_empty() {
            assert_eq!(
                Err(ContractError::EmptyDelegation),
                validate_delegation_stake(&[])
            )
        }

        #[test]
        fn stake_must_have_single_coin_type() {
            assert_eq!(
                Err(ContractError::MultipleDenoms),
                validate_delegation_stake(&[coin(123, DENOM), coin(123, "BTC"), coin(123, "DOGE")])
            )
        }

        #[test]
        fn stake_coin_must_be_of_correct_type() {
            assert_eq!(
                Err(ContractError::WrongDenom {}),
                validate_delegation_stake(&[coin(123, "DOGE")])
            )
        }

        #[test]
        fn stake_coin_must_have_value_greater_than_zero() {
            assert_eq!(
                Err(ContractError::EmptyDelegation),
                validate_delegation_stake(&[coin(0, DENOM)])
            )
        }

        #[test]
        fn stake_can_have_any_positive_value() {
            // this might change in the future, but right now an arbitrary (positive) value can be delegated
            assert!(validate_delegation_stake(&[coin(1, DENOM)]).is_ok());
            assert!(validate_delegation_stake(&[coin(123, DENOM)]).is_ok());
            assert!(validate_delegation_stake(&[coin(10000000000, DENOM)]).is_ok());
        }
    }

    #[cfg(test)]
    mod mix_stake_delegation {
        use super::*;
        use crate::storage::mix_delegations_read;
        use crate::support::tests::helpers::add_mixnode;

        #[test]
        fn fails_if_node_doesnt_exist() {
            let mut deps = helpers::init_contract();
            assert_eq!(
                Err(ContractError::MixNodeBondNotFound {
                    identity: "non-existent-mix-identity".into()
                }),
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    mock_env(),
                    mock_info("sender", &coins(123, DENOM)),
                    "non-existent-mix-identity".into()
                )
            );
        }

        #[test]
        fn succeeds_for_existing_node() {
            let mut deps = helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");

            let delegation = coin(123, DENOM);
            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &vec![delegation.clone()]),
                identity.clone()
            )
            .is_ok());

            assert_eq!(
                RawDelegationData::new(delegation.amount, mock_env().block.height),
                mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity.as_bytes())
                    .is_ok()
            );

            // node's "total_delegation" is increased
            assert_eq!(
                delegation,
                mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
            )
        }

        #[test]
        fn fails_if_node_unbonded() {
            let mut deps = helpers::init_contract();

            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");

            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();

            assert_eq!(
                Err(ContractError::MixNodeBondNotFound {
                    identity: identity.clone()
                }),
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(delegation_owner.as_str(), &coins(123, DENOM)),
                    identity
                )
            );
        }

        #[test]
        fn succeeds_if_node_rebonded() {
            let mut deps = helpers::init_contract();

            let mixnode_owner = "bob";
            add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation = coin(123, DENOM);
            let delegation_owner = Addr::unchecked("sender");

            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &vec![delegation.clone()]),
                identity.clone()
            )
            .is_ok());

            assert_eq!(
                RawDelegationData::new(delegation.amount, mock_env().block.height),
                mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity.as_bytes())
                    .is_ok()
            );

            // node's "total_delegation" is increased
            assert_eq!(
                delegation,
                mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
            )
        }

        #[test]
        fn is_possible_for_an_already_delegated_node() {
            let mut deps = helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");

            let delegation1 = coin(100, DENOM);
            let delegation2 = coin(50, DENOM);

            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &vec![delegation1.clone()]),
                identity.clone(),
            )
            .unwrap();

            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &vec![delegation2.clone()]),
                identity.clone(),
            )
            .unwrap();

            assert_eq!(
                RawDelegationData::new(
                    delegation1.amount + delegation2.amount,
                    mock_env().block.height
                ),
                mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity.as_bytes())
                    .is_ok()
            );

            // node's "total_delegation" is sum of both
            assert_eq!(
                delegation1.amount + delegation2.amount,
                mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
                    .amount
            )
        }

        #[test]
        fn block_height_is_updated_on_new_delegation() {
            let mut deps = helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");
            let delegation = coin(100, DENOM);

            let env1 = mock_env();
            let mut env2 = mock_env();
            let initial_height = env1.block.height;
            let updated_height = initial_height + 42;
            // second env has grown in block height
            env2.block.height = updated_height;

            try_delegate_to_mixnode(
                deps.as_mut(),
                env1,
                mock_info(delegation_owner.as_str(), &vec![delegation.clone()]),
                identity.clone(),
            )
            .unwrap();

            assert_eq!(
                RawDelegationData::new(delegation.amount, initial_height),
                mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );

            try_delegate_to_mixnode(
                deps.as_mut(),
                env2,
                mock_info(delegation_owner.as_str(), &vec![delegation.clone()]),
                identity.clone(),
            )
            .unwrap();

            assert_eq!(
                RawDelegationData::new(delegation.amount + delegation.amount, updated_height),
                mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
        }

        #[test]
        fn block_height_is_not_updated_on_different_delegator() {
            let mut deps = helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner1 = Addr::unchecked("sender1");
            let delegation_owner2 = Addr::unchecked("sender2");
            let delegation1 = coin(100, DENOM);
            let delegation2 = coin(120, DENOM);

            let env1 = mock_env();
            let mut env2 = mock_env();
            let initial_height = env1.block.height;
            let second_height = initial_height + 42;
            // second env has grown in block height
            env2.block.height = second_height;

            try_delegate_to_mixnode(
                deps.as_mut(),
                env1,
                mock_info(delegation_owner1.as_str(), &vec![delegation1.clone()]),
                identity.clone(),
            )
            .unwrap();

            assert_eq!(
                RawDelegationData::new(delegation1.amount, initial_height),
                mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner1.as_bytes())
                    .unwrap()
            );

            try_delegate_to_mixnode(
                deps.as_mut(),
                env2,
                mock_info(delegation_owner2.as_str(), &vec![delegation2.clone()]),
                identity.clone(),
            )
            .unwrap();

            assert_eq!(
                RawDelegationData::new(delegation1.amount, initial_height),
                mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner1.as_bytes())
                    .unwrap()
            );
            assert_eq!(
                RawDelegationData::new(delegation2.amount, second_height),
                mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner2.as_bytes())
                    .unwrap()
            );
        }

        #[test]
        fn is_disallowed_for_already_delegated_node_if_it_unbonded() {
            let mut deps = helpers::init_contract();

            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");

            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();

            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();

            assert_eq!(
                Err(ContractError::MixNodeBondNotFound {
                    identity: identity.clone()
                }),
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(delegation_owner.as_str(), &coins(50, DENOM)),
                    identity
                )
            );
        }

        #[test]
        fn is_allowed_for_multiple_nodes() {
            let mut deps = helpers::init_contract();
            let mixnode_owner1 = "bob";
            let mixnode_owner2 = "fred";
            let identity1 = add_mixnode(mixnode_owner1, good_mixnode_bond(), &mut deps);
            let identity2 = add_mixnode(mixnode_owner2, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");

            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(123, DENOM)),
                identity1.clone()
            )
            .is_ok());

            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(42, DENOM)),
                identity2.clone()
            )
            .is_ok());

            assert_eq!(
                RawDelegationData::new(123u128.into(), mock_env().block.height),
                mix_delegations_read(&deps.storage, &identity1)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity1.as_bytes())
                    .is_ok()
            );

            assert_eq!(
                RawDelegationData::new(42u128.into(), mock_env().block.height),
                mix_delegations_read(&deps.storage, &identity2)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity2.as_bytes())
                    .is_ok()
            );
        }

        #[test]
        fn is_allowed_by_multiple_users() {
            let mut deps = helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);

            let delegation1 = coin(123, DENOM);
            let delegation2 = coin(234, DENOM);

            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info("sender1", &vec![delegation1.clone()]),
                identity.clone()
            )
            .is_ok());

            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info("sender2", &vec![delegation2.clone()]),
                identity.clone()
            )
            .is_ok());

            // node's "total_delegation" is sum of both
            assert_eq!(
                delegation1.amount + delegation2.amount,
                mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
                    .amount
            )
        }

        #[test]
        fn delegation_is_not_removed_if_node_unbonded() {
            let mut deps = helpers::init_contract();

            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");

            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();

            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();

            assert_eq!(
                RawDelegationData::new(100u128.into(), mock_env().block.height),
                mix_delegations_read(&deps.storage, &identity)
                    .load(delegation_owner.as_bytes())
                    .unwrap()
            );
            assert!(
                reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .load(identity.as_bytes())
                    .is_ok()
            );
        }
    }

    #[cfg(test)]
    mod removing_mix_stake_delegation {
        use super::*;
        use crate::storage::mix_delegations_read;
        use crate::support::tests::helpers::add_mixnode;

        #[test]
        fn fails_if_delegation_never_existed() {
            let mut deps = helpers::init_contract();

            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");

            assert_eq!(
                Err(ContractError::NoMixnodeDelegationFound {
                    identity: identity.clone(),
                    address: delegation_owner.clone(),
                }),
                try_remove_delegation_from_mixnode(
                    deps.as_mut(),
                    mock_info(delegation_owner.as_str(), &[]),
                    identity,
                )
            );
        }

        #[test]
        fn succeeds_if_delegation_existed() {
            let mut deps = helpers::init_contract();

            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");

            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();

            assert_eq!(
                Ok(Response {
                    submessages: vec![],
                    messages: vec![BankMsg::Send {
                        to_address: delegation_owner.clone().into(),
                        amount: coins(100, DENOM),
                    }
                    .into()],
                    attributes: Vec::new(),
                    data: None,
                }),
                try_remove_delegation_from_mixnode(
                    deps.as_mut(),
                    mock_info(delegation_owner.as_str(), &[]),
                    identity.clone(),
                )
            );

            assert!(mix_delegations_read(&deps.storage, &identity)
                .may_load(delegation_owner.as_bytes())
                .unwrap()
                .is_none());
            assert!(
                reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .may_load(identity.as_bytes())
                    .unwrap()
                    .is_none()
            );

            // and total delegation is cleared
            assert_eq!(
                Uint128::zero(),
                mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
                    .amount
            )
        }

        #[test]
        fn succeeds_if_delegation_existed_even_if_node_unbonded() {
            let mut deps = helpers::init_contract();

            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner = Addr::unchecked("sender");

            try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner.as_str(), &coins(100, DENOM)),
                identity.clone(),
            )
            .unwrap();

            try_remove_mixnode(deps.as_mut(), mock_info(mixnode_owner, &[])).unwrap();

            assert_eq!(
                Ok(Response {
                    submessages: vec![],
                    messages: vec![BankMsg::Send {
                        to_address: delegation_owner.clone().into(),
                        amount: coins(100, DENOM),
                    }
                    .into()],
                    attributes: Vec::new(),
                    data: None,
                }),
                try_remove_delegation_from_mixnode(
                    deps.as_mut(),
                    mock_info(delegation_owner.as_str(), &[]),
                    identity.clone(),
                )
            );

            assert!(mix_delegations_read(&deps.storage, &identity)
                .may_load(delegation_owner.as_bytes())
                .unwrap()
                .is_none());
            assert!(
                reverse_mix_delegations_read(&deps.storage, &delegation_owner)
                    .may_load(identity.as_bytes())
                    .unwrap()
                    .is_none()
            );
        }

        #[test]
        fn total_delegation_is_preserved_if_only_some_undelegate() {
            let mut deps = helpers::init_contract();
            let mixnode_owner = "bob";
            let identity = add_mixnode(mixnode_owner, good_mixnode_bond(), &mut deps);
            let delegation_owner1 = Addr::unchecked("sender1");
            let delegation_owner2 = Addr::unchecked("sender2");

            let delegation1 = coin(123, DENOM);
            let delegation2 = coin(234, DENOM);

            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner1.as_str(), &vec![delegation1.clone()]),
                identity.clone()
            )
            .is_ok());

            assert!(try_delegate_to_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(delegation_owner2.as_str(), &vec![delegation2.clone()]),
                identity.clone()
            )
            .is_ok());

            // sender1 undelegates
            try_remove_delegation_from_mixnode(
                deps.as_mut(),
                mock_info(delegation_owner1.as_str(), &[]),
                identity.clone(),
            )
            .unwrap();

            // but total delegation should still equal to what sender2 sent
            // node's "total_delegation" is sum of both
            assert_eq!(
                delegation2,
                mixnodes_read(&deps.storage)
                    .load(identity.as_bytes())
                    .unwrap()
                    .total_delegation
            )
        }
    }

    #[test]
    fn delegators_on_mix_node_reward_rate() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        let initial_mix_bond = 100_000000;
        let initial_delegation1 = 50000; // will see single digits rewards
        let initial_delegation2 = 100; // won't see any rewards due to such a small delegation
        let initial_delegation3 = 100000_000000; // will see big proper rewards

        let node_owner = "node-owner";
        let identity = add_mixnode(node_owner, good_mixnode_bond(), &mut deps);

        mix_delegations(&mut deps.storage, &identity)
            .save(
                b"delegator1",
                &RawDelegationData::new(initial_delegation1.into(), env.block.height),
            )
            .unwrap();
        mix_delegations(&mut deps.storage, &identity)
            .save(
                b"delegator2",
                &RawDelegationData::new(initial_delegation2.into(), env.block.height),
            )
            .unwrap();
        mix_delegations(&mut deps.storage, &identity)
            .save(
                b"delegator3",
                &RawDelegationData::new(initial_delegation3.into(), env.block.height),
            )
            .unwrap();

        env.block.height += 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING;

        let bond_reward = current_state.mixnode_epoch_bond_reward;
        let delegation_reward = current_state.mixnode_epoch_delegation_reward;

        // the node's bond and delegations are correctly increased and scaled by uptime
        // if node was 100% up, it will get full epoch reward
        let expected_mix_reward = Uint128(initial_mix_bond) * bond_reward;
        let expected_delegation1_reward = Uint128(initial_delegation1) * delegation_reward;
        let expected_delegation2_reward = Uint128(initial_delegation2) * delegation_reward;
        let expected_delegation3_reward = Uint128(initial_delegation3) * delegation_reward;

        let expected_bond = expected_mix_reward + Uint128(initial_mix_bond);
        let expected_delegation1 = expected_delegation1_reward + Uint128(initial_delegation1);
        let expected_delegation2 = expected_delegation2_reward + Uint128(initial_delegation2);
        let expected_delegation3 = expected_delegation3_reward + Uint128(initial_delegation3);

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            identity.clone(),
            100,
            1,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, identity.as_bytes()).unwrap()
        );

        assert_eq!(
            expected_delegation1,
            mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator1".as_bytes())
                .unwrap()
                .amount
        );

        assert_eq!(
            expected_delegation2,
            mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator2".as_bytes())
                .unwrap()
                .amount
        );

        assert_eq!(
            expected_delegation3,
            mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator3".as_bytes())
                .unwrap()
                .amount
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_mix_reward),
                attr(
                    "total delegation increase",
                    expected_delegation1_reward
                        + expected_delegation2_reward
                        + expected_delegation3_reward
                ),
            ],
            res.attributes
        );

        // if node was 20% up, it will get 1/5th of epoch reward
        let scaled_bond_reward = scale_reward_by_uptime(bond_reward, 20).unwrap();
        let scaled_delegation_reward = scale_reward_by_uptime(delegation_reward, 20).unwrap();

        let expected_mix_reward = expected_bond * scaled_bond_reward;
        let expected_delegation1_reward = expected_delegation1 * scaled_delegation_reward;
        let expected_delegation2_reward = expected_delegation2 * scaled_delegation_reward;
        let expected_delegation3_reward = expected_delegation3 * scaled_delegation_reward;

        let expected_bond = expected_mix_reward + expected_bond;
        let expected_delegation1 = expected_delegation1_reward + expected_delegation1;
        let expected_delegation2 = expected_delegation2_reward + expected_delegation2;
        let expected_delegation3 = expected_delegation3_reward + expected_delegation3;

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            identity.clone(),
            20,
            2,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 2).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, identity.as_bytes()).unwrap()
        );

        assert_eq!(
            expected_delegation1,
            mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator1".as_bytes())
                .unwrap()
                .amount
        );

        assert_eq!(
            expected_delegation2,
            mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator2".as_bytes())
                .unwrap()
                .amount
        );

        assert_eq!(
            expected_delegation3,
            mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator3".as_bytes())
                .unwrap()
                .amount
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_mix_reward),
                attr(
                    "total delegation increase",
                    expected_delegation1_reward
                        + expected_delegation2_reward
                        + expected_delegation3_reward
                ),
            ],
            res.attributes
        );

        // if the node was 0% up, nobody will get any rewards
        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 3).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            identity.clone(),
            0,
            3,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 3).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, identity.as_bytes()).unwrap()
        );

        assert_eq!(
            expected_delegation1,
            mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator1".as_bytes())
                .unwrap()
                .amount
        );

        assert_eq!(
            expected_delegation2,
            mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator2".as_bytes())
                .unwrap()
                .amount
        );

        assert_eq!(
            expected_delegation3,
            mix_delegations_read(deps.as_ref().storage, &identity)
                .load("delegator3".as_bytes())
                .unwrap()
                .amount
        );

        assert_eq!(
            vec![
                attr("bond increase", Uint128(0)),
                attr("total delegation increase", Uint128(0)),
            ],
            res.attributes
        );
    }

    #[test]
    fn multiple_page_delegations() {
        let mut deps = helpers::init_contract();
        let node_identity: IdentityKey = "foo".into();

        store_n_mix_delegations(
            DELEGATION_PAGE_DEFAULT_LIMIT * 10,
            &mut deps.storage,
            &node_identity,
        );
        let mix_bucket = all_mix_delegations_read::<RawDelegationData>(&deps.storage);
        let mix_delegations =
            Delegations::new(mix_bucket).collect::<Vec<UnpackedDelegation<RawDelegationData>>>();
        assert_eq!(
            DELEGATION_PAGE_DEFAULT_LIMIT * 10,
            mix_delegations.len() as u32
        );
    }

    #[cfg(test)]
    mod finding_old_delegations {
        use super::*;

        #[test]
        fn when_there_werent_any() {
            let deps = helpers::init_contract();

            let node_identity: IdentityKey = "nodeidentity".into();

            let read_bucket = mix_delegations_read(&deps.storage, &node_identity);
            let old_delegations = total_delegations(read_bucket).unwrap();

            assert_eq!(Coin::new(0, DENOM), old_delegations);
        }

        #[test]
        fn when_some_existed() {
            let num_delegations = vec![
                1,
                5,
                OLD_DELEGATIONS_CHUNK_SIZE - 1,
                OLD_DELEGATIONS_CHUNK_SIZE,
                OLD_DELEGATIONS_CHUNK_SIZE + 1,
                OLD_DELEGATIONS_CHUNK_SIZE * 3,
                OLD_DELEGATIONS_CHUNK_SIZE * 3 + 1,
            ];

            for delegations in num_delegations {
                let mut deps = helpers::init_contract();

                let node_identity: IdentityKey = "nodeidentity".into();

                // delegate some stake
                let mut write_bucket = mix_delegations(&mut deps.storage, &node_identity);
                for i in 1..=delegations {
                    let delegator = Addr::unchecked(format!("delegator{}", i));
                    let delegation = raw_delegation_fixture(i as u128);
                    write_bucket
                        .save(delegator.as_bytes(), &delegation)
                        .unwrap();
                }

                let read_bucket = mix_delegations_read(&deps.storage, &node_identity);
                let old_delegations = total_delegations(read_bucket).unwrap();

                let total_delegation = (1..=delegations as u128).into_iter().sum();
                assert_eq!(Coin::new(total_delegation, DENOM), old_delegations);
            }
        }
    }

    #[test]
    fn choose_layer_mix_node() {
        let mut deps = helpers::init_contract();
        for owner in ["alice", "bob"] {
            try_add_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(owner, &good_mixnode_bond()),
                MixNode {
                    identity_key: owner.to_string(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();
        }
        let bonded_mix_nodes = helpers::get_mix_nodes(&mut deps);
        let alice_node = bonded_mix_nodes.get(0).unwrap().clone();
        let bob_node = bonded_mix_nodes.get(1).unwrap().clone();
        assert_eq!(alice_node.mix_node.identity_key, "alice");
        assert_eq!(alice_node.layer, Layer::One);
        assert_eq!(bob_node.mix_node.identity_key, "bob");
        assert_eq!(bob_node.layer, Layer::Two);
    }

    #[test]
    fn test_tokenomics_rewarding() {
        use crate::contract::{EPOCH_REWARD_PERCENT, INITIAL_REWARD_POOL};

        type U128 = fixed::types::U75F53;

        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        let current_state = config(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;
        let period_reward_pool = (INITIAL_REWARD_POOL / 100) * EPOCH_REWARD_PERCENT as u128;
        assert_eq!(period_reward_pool, 5_000_000_000_000);
        let k = 200; // Imagining our active set size is 200
        let circulating_supply = circulating_supply(&deps.storage).u128();
        assert_eq!(circulating_supply, 750_000_000_000_000u128);
        // mut_reward_pool(deps.as_mut().storage)
        //     .save(&Uint128(period_reward_pool))
        //     .unwrap();

        try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info(
                "alice",
                &vec![Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128(10000_000_000),
                }],
            ),
            MixNode {
                identity_key: "alice".to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d1", &vec![coin(8000_000000, DENOM)]),
            "alice".to_string(),
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d2", &vec![coin(2000_000000, DENOM)]),
            "alice".to_string(),
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(
            deps.as_mut(),
            env.clone(),
            mock_info(rewarding_validator_address.as_ref(), &[]),
            1,
        )
        .unwrap();

        env.block.height += 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING;

        let mix_1 = mixnodes_read(&deps.storage).load(b"alice").unwrap();
        let mix_1_uptime = 100;

        let mut params = NodeRewardParams::new(
            period_reward_pool,
            k,
            0,
            circulating_supply,
            mix_1_uptime,
            DEFAULT_SYBIL_RESISTANCE_PERCENT,
        );

        params.set_reward_blockstamp(env.block.height);

        assert_eq!(params.performance(), 1);

        let mix_1_reward_result = mix_1.reward(&params);

        assert_eq!(
            mix_1_reward_result.sigma(),
            U128::from_num(0.0000266666666666)
        );
        assert_eq!(
            mix_1_reward_result.lambda(),
            U128::from_num(0.0000133333333333)
        );
        assert_eq!(mix_1_reward_result.reward().int(), 102646153);

        let mix1_operator_profit = mix_1.operator_reward(&params);

        let mix1_delegator1_reward = mix_1.reward_delegation(Uint128(8000_000000), &params);

        let mix1_delegator2_reward = mix_1.reward_delegation(Uint128(2000_000000), &params);

        assert_eq!(mix1_operator_profit, U128::from_num(74455384));
        assert_eq!(mix1_delegator1_reward, U128::from_num(22552615));
        assert_eq!(mix1_delegator2_reward, U128::from_num(5638153));

        let pre_reward_bond = read_mixnode_bond(&deps.storage, b"alice").unwrap().u128();
        assert_eq!(pre_reward_bond, 10000_000_000);

        let pre_reward_delegation = read_mixnode_delegation(&deps.storage, b"alice")
            .unwrap()
            .u128();
        assert_eq!(pre_reward_delegation, 10000_000_000);

        try_reward_mixnode_v2(deps.as_mut(), env, info, "alice".to_string(), params, 1).unwrap();

        assert_eq!(
            read_mixnode_bond(&deps.storage, b"alice").unwrap().u128(),
            U128::from_num(pre_reward_bond) + U128::from_num(mix1_operator_profit)
        );
        assert_eq!(
            read_mixnode_delegation(&deps.storage, b"alice")
                .unwrap()
                .u128(),
            pre_reward_delegation + mix1_delegator1_reward + mix1_delegator2_reward
        );

        assert_eq!(
            reward_pool_value(&deps.storage).u128(),
            U128::from_num(INITIAL_REWARD_POOL)
                - (U128::from_num(mix1_operator_profit)
                    + U128::from_num(mix1_delegator1_reward)
                    + U128::from_num(mix1_delegator2_reward))
        )
    }
}
