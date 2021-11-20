use super::helpers;
use crate::error::ContractError;
use crate::mixnet_params::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::{attr, DepsMut, Env, MessageInfo, Response, Uint128};
use mixnet_contract::mixnode::NodeRewardParams;
use mixnet_contract::IdentityKey;

// approximately 1 day (assuming 5s per block)
pub(crate) const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 17280;

// approximately 30min (assuming 5s per block)
pub(crate) const MAX_REWARDING_DURATION_IN_BLOCKS: u64 = 360;

// Note: this function is designed to work with only a single validator entity distributing rewards
// The main purpose of this function is to update `latest_rewarding_interval_nonce` which
// will trigger a different seed selection for the pseudorandom generation of the "demanded" set of mixnodes.
pub(crate) fn try_begin_mixnode_rewarding(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let mut state = mixnet_params_storage::config_read(deps.storage).load()?;

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

    mixnet_params_storage::config(deps.storage).save(&state)?;

    let mut response = Response::new();
    response.add_attribute(
        "rewarding interval nonce",
        rewarding_interval_nonce.to_string(),
    );
    Ok(response)
}

// Note: if any changes are made to this function or anything it is calling down the stack,
// for example delegation reward distribution, the gas limits must be retested and both
// validator-api/src/rewarding/mixnodes::{MIXNODE_REWARD_OP_BASE_GAS_LIMIT, PER_MIXNODE_DELEGATION_GAS_INCREASE}
// must be updated appropriately.
pub(crate) fn try_reward_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
    uptime: u32,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let state = mixnet_params_storage::config_read(deps.storage).load()?;

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
    if mixnodes_storage::rewarded_mixnodes_read(deps.storage, rewarding_interval_nonce)
        .may_load(mix_identity.as_bytes())?
        .is_some()
    {
        return Err(ContractError::MixnodeAlreadyRewarded {
            identity: mix_identity,
        });
    }

    // optimisation for uptime being 0. No rewards will be given so just terminate here
    if uptime == 0 {
        mixnodes_storage::rewarded_mixnodes(deps.storage, rewarding_interval_nonce)
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
    let mut current_bond =
        match mixnodes_storage::mixnodes_read(deps.storage).load(mix_identity.as_bytes()) {
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
        let bond_scaled_reward_rate = helpers::scale_reward_by_uptime(bond_reward_rate, uptime)?;
        let delegation_scaled_reward_rate =
            helpers::scale_reward_by_uptime(delegation_reward_rate, uptime)?;

        total_delegation_reward = mixnodes_storage::increase_mix_delegated_stakes(
            deps.storage,
            &mix_identity,
            delegation_scaled_reward_rate,
            env.block.height,
        )?;

        node_reward = current_bond.bond_amount.amount * bond_scaled_reward_rate;
        current_bond.bond_amount.amount += node_reward;
        current_bond.total_delegation.amount += total_delegation_reward;
        mixnodes_storage::mixnodes(deps.storage).save(mix_identity.as_bytes(), &current_bond)?;
    }

    mixnodes_storage::rewarded_mixnodes(deps.storage, rewarding_interval_nonce)
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
    let state = mixnet_params_storage::config_read(deps.storage).load()?;

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
    if mixnodes_storage::rewarded_mixnodes_read(deps.storage, rewarding_interval_nonce)
        .may_load(mix_identity.as_bytes())?
        .is_some()
    {
        return Err(ContractError::MixnodeAlreadyRewarded {
            identity: mix_identity,
        });
    }

    // check if the bond even exists
    let mut current_bond =
        match mixnodes_storage::mixnodes_read(deps.storage).load(mix_identity.as_bytes()) {
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

    let total_delegation_reward = mixnodes_storage::increase_mix_delegated_stakes_v2(
        deps.storage,
        &current_bond,
        &reward_params,
    )?;

    // update current bond with the reward given to the node and the delegators
    // if it has been bonded for long enough
    if current_bond.block_height + MINIMUM_BLOCK_AGE_FOR_REWARDING
        <= reward_params.reward_blockstamp()
    {
        current_bond.bond_amount.amount += Uint128(operator_reward);
        current_bond.total_delegation.amount += total_delegation_reward;
        mixnodes_storage::mixnodes(deps.storage).save(mix_identity.as_bytes(), &current_bond)?;
        mixnet_params_storage::decr_reward_pool(Uint128(operator_reward), deps.storage)?;
        mixnet_params_storage::decr_reward_pool(total_delegation_reward, deps.storage)?;
    }

    mixnodes_storage::rewarded_mixnodes(deps.storage, rewarding_interval_nonce)
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
    let mut state = mixnet_params_storage::config_read(deps.storage).load()?;

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
    mixnet_params_storage::config(deps.storage).save(&state)?;

    Ok(Response::new())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::DEFAULT_SYBIL_RESISTANCE_PERCENT;
    use crate::error::ContractError;
    use crate::mixnodes::bonding_transactions::try_add_mixnode;
    use crate::mixnodes::delegation_transactions::try_delegate_to_mixnode;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::rewards::transactions::{
        try_begin_mixnode_rewarding, try_finish_mixnode_rewarding, try_reward_mixnode,
        try_reward_mixnode_v2,
    };
    use crate::support::tests::test_helpers;
    use crate::support::tests::test_helpers::{good_mixnode_bond, mix_node_fixture};
    use config::defaults::DENOM;
    use cosmwasm_std::attr;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Coin;
    use cosmwasm_std::{coin, Addr, Uint128};
    use mixnet_contract::mixnode::NodeRewardParams;
    use mixnet_contract::MixNode;
    use mixnet_contract::MixNodeBond;
    use mixnet_contract::{IdentityKey, Layer, RawDelegationData};

    #[cfg(test)]
    mod beginning_mixnode_rewarding {
        use super::*;
        use crate::rewards::transactions::try_begin_mixnode_rewarding;
        use crate::support::tests::test_helpers;
        use cosmwasm_std::testing::mock_env;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            mixnet_params_storage::config(deps.as_mut().storage)
                .save(&current_state)
                .unwrap();

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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let start_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
            let rewarding_validator_address = start_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let new_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
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
        use crate::rewards::transactions::{
            try_begin_mixnode_rewarding, try_finish_mixnode_rewarding,
        };
        use crate::support::tests::test_helpers;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
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
            let mut deps = test_helpers::init_contract();
            let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            mixnet_params_storage::config(deps.as_mut().storage)
                .save(&current_state)
                .unwrap();

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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
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

            let new_state = mixnet_params_storage::config_read(deps.as_mut().storage)
                .load()
                .unwrap();
            assert!(!new_state.rewarding_in_progress);
        }
    }

    #[test]
    fn rewarding_mixnode() {
        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();
        let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
            .load()
            .unwrap();
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

        mixnodes_storage::mixnodes(deps.as_mut().storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        mixnodes_storage::mix_delegations(&mut deps.storage, &node_identity)
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
            mixnodes_storage::read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
        );
        assert_eq!(
            expected_delegation,
            mixnodes_storage::read_mixnode_delegation(
                deps.as_ref().storage,
                node_identity.as_bytes()
            )
            .unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );

        // if node was 20% up, it will get 1/5th of epoch reward
        let scaled_bond_reward = helpers::scale_reward_by_uptime(bond_reward_rate, 20).unwrap();
        let scaled_delegation_reward =
            helpers::scale_reward_by_uptime(delegation_reward_rate, 20).unwrap();
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
            mixnodes_storage::read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
        );
        assert_eq!(
            expected_delegation,
            mixnodes_storage::read_mixnode_delegation(
                deps.as_ref().storage,
                node_identity.as_bytes()
            )
            .unwrap()
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
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
            .load()
            .unwrap();
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
                ..test_helpers::mix_node_fixture()
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
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
            .load()
            .unwrap();
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
                ..test_helpers::mix_node_fixture()
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
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
            .load()
            .unwrap();
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
                ..test_helpers::mix_node_fixture()
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
        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();
        let current_state = mixnet_params_storage::config_read(deps.as_mut().storage)
            .load()
            .unwrap();
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

        mixnodes_storage::mixnodes(deps.as_mut().storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        // delegation happens later, but not later enough
        env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;

        mixnodes_storage::mix_delegations(&mut deps.storage, &node_identity)
            .save(
                b"delegator",
                &RawDelegationData::new(initial_delegation.into(), env.block.height),
            )
            .unwrap();

        let bond_reward_rate = current_state.mixnode_epoch_bond_reward;
        let delegation_reward_rate = current_state.mixnode_epoch_delegation_reward;
        let scaled_bond_reward = helpers::scale_reward_by_uptime(bond_reward_rate, 100).unwrap();
        let scaled_delegation_reward =
            helpers::scale_reward_by_uptime(delegation_reward_rate, 100).unwrap();

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
            mixnodes_storage::read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
        );
        assert_eq!(
            expected_delegation,
            mixnodes_storage::read_mixnode_delegation(
                deps.as_ref().storage,
                node_identity.as_bytes()
            )
            .unwrap()
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
            mixnodes_storage::read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
        );
        assert_eq!(
            expected_delegation,
            mixnodes_storage::read_mixnode_delegation(
                deps.as_ref().storage,
                node_identity.as_bytes()
            )
            .unwrap()
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
            mixnodes_storage::read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
        );
        assert_eq!(
            expected_delegation,
            mixnodes_storage::read_mixnode_delegation(
                deps.as_ref().storage,
                node_identity.as_bytes()
            )
            .unwrap()
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
    fn test_tokenomics_rewarding() {
        use crate::contract::{EPOCH_REWARD_PERCENT, INITIAL_REWARD_POOL};

        type U128 = fixed::types::U75F53;

        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();
        let current_state = mixnet_params_storage::config(deps.as_mut().storage)
            .load()
            .unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;
        let period_reward_pool = (INITIAL_REWARD_POOL / 100) * EPOCH_REWARD_PERCENT as u128;
        assert_eq!(period_reward_pool, 5_000_000_000_000);
        let k = 200; // Imagining our active set size is 200
        let circulating_supply = mixnet_params_storage::circulating_supply(&deps.storage).u128();
        assert_eq!(circulating_supply, 750_000_000_000_000u128);
        // mut_reward_pool(deps.as_mut().storage)
        //     .save(&Uint128(period_reward_pool))
        //     .unwrap();

        try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info(
                "alice",
                &[Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128(10_000_000_000),
                }],
            ),
            MixNode {
                identity_key: "alice".to_string(),
                ..test_helpers::mix_node_fixture()
            },
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d1", &[coin(8000_000000, DENOM)]),
            "alice".to_string(),
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d2", &[coin(2000_000000, DENOM)]),
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

        let mix_1 = mixnodes_storage::mixnodes_read(&deps.storage)
            .load(b"alice")
            .unwrap();
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

        let pre_reward_bond = mixnodes_storage::read_mixnode_bond(&deps.storage, b"alice")
            .unwrap()
            .u128();
        assert_eq!(pre_reward_bond, 10_000_000_000);

        let pre_reward_delegation =
            mixnodes_storage::read_mixnode_delegation(&deps.storage, b"alice")
                .unwrap()
                .u128();
        assert_eq!(pre_reward_delegation, 10_000_000_000);

        try_reward_mixnode_v2(deps.as_mut(), env, info, "alice".to_string(), params, 1).unwrap();

        assert_eq!(
            mixnodes_storage::read_mixnode_bond(&deps.storage, b"alice")
                .unwrap()
                .u128(),
            U128::from_num(pre_reward_bond) + U128::from_num(mix1_operator_profit)
        );
        assert_eq!(
            mixnodes_storage::read_mixnode_delegation(&deps.storage, b"alice")
                .unwrap()
                .u128(),
            pre_reward_delegation + mix1_delegator1_reward + mix1_delegator2_reward
        );

        assert_eq!(
            mixnet_params_storage::reward_pool_value(&deps.storage).u128(),
            U128::from_num(INITIAL_REWARD_POOL)
                - (U128::from_num(mix1_operator_profit)
                    + U128::from_num(mix1_delegator1_reward)
                    + U128::from_num(mix1_delegator2_reward))
        )
    }
}
