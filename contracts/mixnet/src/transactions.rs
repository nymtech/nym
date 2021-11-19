// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) const OLD_DELEGATIONS_CHUNK_SIZE: usize = 500;

// approximately 1 day (assuming 5s per block)
pub(crate) const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 17280;

// approximately 30min (assuming 5s per block)
pub(crate) const MAX_REWARDING_DURATION_IN_BLOCKS: u64 = 360;

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::bonding_gateways::transactions::try_add_gateway;
    use crate::bonding_gateways::transactions::validate_gateway_bond;
    use crate::bonding_mixnodes::transactions::try_add_mixnode;
    use crate::contract::{
        execute, query, DEFAULT_SYBIL_RESISTANCE_PERCENT, INITIAL_DEFAULT_EPOCH_LENGTH,
        INITIAL_GATEWAY_BOND, INITIAL_MIXNODE_BOND, INITIAL_MIXNODE_BOND_REWARD_RATE,
        INITIAL_MIXNODE_DELEGATION_REWARD_RATE,
    };
    use crate::delegating_mixnodes::transactions::try_delegate_to_mixnode;
    use crate::error::ContractError;
    use crate::helpers::calculate_epoch_reward_rate;
    use crate::helpers::scale_reward_by_uptime;
    use crate::mixnet_params::transactions::try_update_state_params;
    use crate::rewards::transactions::{
        try_begin_mixnode_rewarding, try_finish_mixnode_rewarding, try_reward_mixnode,
        try_reward_mixnode_v2,
    };
    use crate::storage::read_mixnode_bond;
    use crate::storage::*;
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{good_gateway_bond, good_mixnode_bond, mix_node_fixture};
    use config::defaults::DENOM;
    use cosmwasm_std::attr;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Decimal;
    use cosmwasm_std::{coin, from_binary, Addr, Uint128};
    use cosmwasm_std::{coins, BankMsg, Coin, Response};
    use mixnet_contract::mixnode::NodeRewardParams;
    use mixnet_contract::Gateway;
    use mixnet_contract::MixNode;
    use mixnet_contract::MixNodeBond;
    use mixnet_contract::StateParams;
    use mixnet_contract::{ExecuteMsg, PagedGatewayResponse, QueryMsg};
    use mixnet_contract::{IdentityKey, Layer, RawDelegationData};

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
        use crate::rewards::transactions::try_begin_mixnode_rewarding;

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
        use crate::rewards::transactions::{
            try_begin_mixnode_rewarding, try_finish_mixnode_rewarding,
        };

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
        assert_eq!(bob_node.layer, mixnet_contract::Layer::Two);
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
                &[Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128(10_000_000_000),
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
        assert_eq!(pre_reward_bond, 10_000_000_000);

        let pre_reward_delegation = read_mixnode_delegation(&deps.storage, b"alice")
            .unwrap()
            .u128();
        assert_eq!(pre_reward_delegation, 10_000_000_000);

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
