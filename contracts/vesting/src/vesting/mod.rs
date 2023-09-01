mod account;

pub(crate) use account::StorableVestingAccountExt;
use vesting_contract_common::{VestingPeriod, VestingSpecification};

pub fn populate_vesting_periods(
    start_time: u64,
    vesting_spec: VestingSpecification,
) -> Vec<VestingPeriod> {
    vesting_spec.populate_vesting_periods(start_time)
}

#[cfg(test)]
mod tests {
    use crate::contract::*;
    use crate::storage::*;
    use crate::support::tests::helpers::vesting_account_percent_fixture;
    use crate::support::tests::helpers::{
        init_contract, vesting_account_mid_fixture, vesting_account_new_fixture, TEST_COIN_DENOM,
    };
    use crate::traits::DelegatingAccount;
    use crate::traits::VestingAccount;
    use crate::traits::{GatewayBondingAccount, MixnodeBondingAccount};
    use crate::vesting::account::StorableVestingAccountExt;
    use crate::vesting::populate_vesting_periods;
    use contracts_common::signing::MessageSignature;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, coins, Addr, Coin, Timestamp, Uint128};
    use mixnet_contract_common::mixnode::MixNodeCostParams;
    use mixnet_contract_common::{Gateway, MixNode, Percent};
    use vesting_contract_common::messages::ExecuteMsg;
    use vesting_contract_common::{Account, PledgeCap, VestingSpecification};
    use vesting_contract_common::{Period, VestingContractError};

    #[test]
    fn test_account_creation() {
        let mut deps = init_contract();
        let env = mock_env();
        let info = mock_info("not_admin", &coins(1_000_000_000_000, TEST_COIN_DENOM));
        let msg = ExecuteMsg::CreateAccount {
            owner_address: "owner".to_string(),
            staking_address: Some("staking".to_string()),
            vesting_spec: None,
            cap: Some(PledgeCap::Absolute(Uint128::from(100_000_000_000u128))),
        };
        // Try creating an account when not admin
        let response = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert!(response.is_err());

        let info = mock_info("admin", &coins(1_000_000_000_000, TEST_COIN_DENOM));
        let _response = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        let created_account = load_account(Addr::unchecked("owner"), &deps.storage)
            .unwrap()
            .unwrap();

        assert_eq!(
            created_account.load_balance(&deps.storage).unwrap(),
            // One was liquidated
            Uint128::new(1_000_000_000_000)
        );

        // nothing is saved for "staking" account!
        let created_account_test_by_staking =
            load_account(Addr::unchecked("staking"), &deps.storage).unwrap();
        assert!(created_account_test_by_staking.is_none());

        // but we can stake on its behalf!
        let stake_msg = ExecuteMsg::DelegateToMixnode {
            on_behalf_of: Some("owner".to_string()),
            mix_id: 42,
            amount: coin(500, TEST_COIN_DENOM),
        };

        let response = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("staking", &[]),
            stake_msg,
        );
        assert!(response.is_ok());

        assert_eq!(
            created_account.load_balance(&deps.storage).unwrap(),
            // One was liquidated
            Uint128::new(999_999_999_500)
        );

        // Try create the same account again
        let response = execute(deps.as_mut(), env.clone(), info, msg);
        assert!(response.is_err());

        let account_again = vesting_account_new_fixture(&mut deps.storage, &env);
        assert_eq!(created_account.storage_key(), 1);
        assert_ne!(created_account.storage_key(), account_again.storage_key());
    }

    #[test]
    fn test_ownership_transfer() {
        let mut deps = init_contract();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let account = vesting_account_new_fixture(&mut deps.storage, &env);
        let staker = account.staking_address().unwrap();
        let msg = ExecuteMsg::TransferOwnership {
            to_address: "new_owner".to_string(),
        };
        let _response = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let new_owner_account = load_account(Addr::unchecked("new_owner"), &deps.storage)
            .unwrap()
            .unwrap();
        assert_eq!(
            new_owner_account.load_balance(&deps.storage),
            account.load_balance(&deps.storage)
        );

        // Check old account is gone
        let old_owner_account = load_account(Addr::unchecked("owner"), &deps.storage).unwrap();
        assert!(old_owner_account.is_none());

        // Not the owner
        let response = execute(deps.as_mut(), env.clone(), info, msg);
        assert!(response.is_err());

        // can't stake on behalf of the original owner anymore, but we can do it for the new one!
        let stake_msg = ExecuteMsg::DelegateToMixnode {
            on_behalf_of: Some("owner".to_string()),
            mix_id: 42,
            amount: coin(500, TEST_COIN_DENOM),
        };
        let response = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(staker.as_ref(), &[]),
            stake_msg,
        );
        assert!(response.is_err());

        let new_stake_msg = ExecuteMsg::DelegateToMixnode {
            on_behalf_of: Some("new_owner".to_string()),
            mix_id: 42,
            amount: coin(500, TEST_COIN_DENOM),
        };
        let response = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(staker.as_ref(), &[]),
            new_stake_msg,
        );
        assert!(response.is_ok());

        let info = mock_info("new_owner", &[]);
        let msg = ExecuteMsg::UpdateStakingAddress {
            to_address: Some("new_staking".to_string()),
        };
        let _response = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::WithdrawVestedCoins {
            amount: Coin {
                amount: Uint128::new(1),
                denom: TEST_COIN_DENOM.to_string(),
            },
        };
        let info = mock_info("new_owner", &[]);
        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + 100_000_000_000_000_000);
        let response = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert!(response.is_ok());

        let info = mock_info("owner", &[]);
        let response = execute(deps.as_mut(), env.clone(), info, msg);
        assert!(response.is_err());
    }

    #[test]
    fn test_staking_account() {
        let mut deps = init_contract();
        let mut env = mock_env();
        let info = mock_info("staking", &[]);
        let msg = ExecuteMsg::TransferOwnership {
            to_address: "new_owner".to_string(),
        };
        let response = execute(deps.as_mut(), env.clone(), info.clone(), msg);
        // Only owner can transfer
        assert!(response.is_err());

        let msg = ExecuteMsg::WithdrawVestedCoins {
            amount: Coin {
                amount: Uint128::new(1),
                denom: "nym".to_string(),
            },
        };
        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + 100_000_000_000_000_000);
        let response = execute(deps.as_mut(), env, info, msg);
        // Only owner can withdraw
        assert!(response.is_err());
    }

    #[test]
    fn test_staking_address_change() {
        let mut deps = init_contract();
        let env = mock_env();
        let account = vesting_account_new_fixture(&mut deps.storage, &env);
        let original_staker = account.staking_address().unwrap();

        // can stake on behalf without an issue
        let stake_msg = ExecuteMsg::DelegateToMixnode {
            on_behalf_of: Some("owner".to_string()),
            mix_id: 42,
            amount: coin(500, TEST_COIN_DENOM),
        };
        let response = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(original_staker.as_ref(), &[]),
            stake_msg.clone(),
        );
        assert!(response.is_ok());

        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::UpdateStakingAddress {
            to_address: Some("new_staking".to_string()),
        };
        let _response = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // the old staking account can't do any staking anymore!
        let response = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(original_staker.as_ref(), &[]),
            stake_msg.clone(),
        );
        assert!(response.is_err());

        // but the new one can
        let response = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("new_staking", &[]),
            stake_msg,
        );
        assert!(response.is_ok());
    }

    #[test]
    fn test_staking_account_transfer() {
        let mut deps = init_contract();
        let env = mock_env();

        let amount1 = coin(1000000000, "unym");
        let amount2 = coin(100, "unym");

        // create the accounts
        let msg1 = ExecuteMsg::CreateAccount {
            owner_address: "vesting1".to_string(),
            staking_address: None,
            vesting_spec: None,
            cap: None,
        };
        let res1 = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("admin", &[amount1.clone()]),
            msg1,
        );
        assert!(res1.is_ok());

        let msg2 = ExecuteMsg::CreateAccount {
            owner_address: "vesting2".to_string(),
            staking_address: None,
            vesting_spec: None,
            cap: None,
        };
        let res2 = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("admin", &[amount2.clone()]),
            msg2,
        );
        assert!(res2.is_ok());

        let vesting1 = try_get_vesting_coins("vesting1", None, env.clone(), deps.as_ref()).unwrap();
        assert_eq!(vesting1, amount1);

        let vesting2 = try_get_vesting_coins("vesting2", None, env.clone(), deps.as_ref()).unwrap();
        assert_eq!(vesting2, amount2);

        let staking_address_change = ExecuteMsg::UpdateStakingAddress {
            to_address: Some("vesting1".to_string()),
        };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("vesting2", &[]),
            staking_address_change,
        );
        assert_eq!(
            Err(VestingContractError::StakingAccountExists(
                "vesting1".to_string()
            )),
            res
        );

        // ensure nothing has changed!
        let vesting1 = try_get_vesting_coins("vesting1", None, env.clone(), deps.as_ref()).unwrap();
        assert_eq!(vesting1, amount1);

        let vesting2 = try_get_vesting_coins("vesting2", None, env, deps.as_ref()).unwrap();
        assert_eq!(vesting2, amount2);
    }

    #[test]
    fn test_period_logic() {
        let mut deps = init_contract();
        let env = mock_env();
        let num_vesting_periods = 8;
        let vesting_period = 3 * 30 * 86400;

        let account = vesting_account_new_fixture(&mut deps.storage, &env);

        assert_eq!(account.periods().len(), num_vesting_periods as usize);

        let current_period = account
            .get_current_vesting_period(Timestamp::from_seconds(0))
            .unwrap();
        assert_eq!(Period::Before, current_period);

        let block_time =
            Timestamp::from_seconds(account.start_time().seconds() + vesting_period + 1);
        let current_period = account.get_current_vesting_period(block_time).unwrap();
        assert_eq!(current_period, Period::In(1));
        let vested_coins = account
            .get_vested_coins(Some(block_time), &env, &deps.storage)
            .unwrap();
        let vesting_coins = account
            .get_vesting_coins(Some(block_time), &env, &deps.storage)
            .unwrap();
        assert_eq!(
            vested_coins.amount,
            Uint128::new(
                account
                    .get_original_vesting()
                    .unwrap()
                    .amount()
                    .amount
                    .u128()
                    / num_vesting_periods as u128
            )
        );
        assert_eq!(
            vesting_coins.amount,
            Uint128::new(
                account
                    .get_original_vesting()
                    .unwrap()
                    .amount()
                    .amount
                    .u128()
                    - account
                        .get_original_vesting()
                        .unwrap()
                        .amount()
                        .amount
                        .u128()
                        / num_vesting_periods as u128
            )
        );

        let block_time =
            Timestamp::from_seconds(account.start_time().seconds() + 5 * vesting_period + 1);
        let current_period = account.get_current_vesting_period(block_time).unwrap();
        assert_eq!(current_period, Period::In(5));
        let vested_coins = account
            .get_vested_coins(Some(block_time), &env, &deps.storage)
            .unwrap();
        let vesting_coins = account
            .get_vesting_coins(Some(block_time), &env, &deps.storage)
            .unwrap();
        assert_eq!(
            vested_coins.amount,
            Uint128::new(
                5 * account
                    .get_original_vesting()
                    .unwrap()
                    .amount()
                    .amount
                    .u128()
                    / num_vesting_periods as u128
            )
        );
        assert_eq!(
            vesting_coins.amount,
            Uint128::new(
                account
                    .get_original_vesting()
                    .unwrap()
                    .amount()
                    .amount
                    .u128()
                    - 5 * account
                        .get_original_vesting()
                        .unwrap()
                        .amount()
                        .amount
                        .u128()
                        / num_vesting_periods as u128
            )
        );
        let vesting_over_period = num_vesting_periods + 1;
        let block_time = Timestamp::from_seconds(
            account.start_time().seconds() + vesting_over_period * vesting_period + 1,
        );
        let current_period = account.get_current_vesting_period(block_time).unwrap();
        assert_eq!(current_period, Period::After);
        let vested_coins = account
            .get_vested_coins(Some(block_time), &env, &deps.storage)
            .unwrap();
        let vesting_coins = account
            .get_vesting_coins(Some(block_time), &env, &deps.storage)
            .unwrap();
        assert_eq!(
            vested_coins.amount,
            Uint128::new(
                account
                    .get_original_vesting()
                    .unwrap()
                    .amount()
                    .amount
                    .u128()
            )
        );
        assert_eq!(vesting_coins.amount, Uint128::zero());
    }

    #[test]
    fn test_withdraw_case() {
        let mut deps = init_contract();
        let env = mock_env();
        let account = vesting_account_mid_fixture(&mut deps.storage, &env);

        let vested_coins = account.get_vested_coins(None, &env, &deps.storage).unwrap();
        let vesting_coins = account
            .get_vesting_coins(None, &env, &deps.storage)
            .unwrap();
        let locked_coins = account.locked_coins(None, &env, &deps.storage).unwrap();
        assert_eq!(vested_coins.amount, Uint128::new(250_000_000_000));
        assert_eq!(vesting_coins.amount, Uint128::new(750_000_000_000));
        assert_eq!(locked_coins.amount, Uint128::new(750_000_000_000));
        let spendable = account.spendable_coins(None, &env, &deps.storage).unwrap();
        assert_eq!(spendable.amount, Uint128::new(250_000_000_000));
        let withdrawn = account.load_withdrawn(&deps.storage).unwrap();
        assert_eq!(withdrawn, Uint128::zero());

        let mix_id = 1;

        let delegation = Coin {
            amount: Uint128::new(90_000_000_000),
            denom: TEST_COIN_DENOM.to_string(),
        };

        let ok =
            account.try_delegate_to_mixnode(mix_id, delegation.clone(), &env, &mut deps.storage);
        assert!(ok.is_ok());

        let vested_coins = account.get_vested_coins(None, &env, &deps.storage).unwrap();
        let vesting_coins = account
            .get_vesting_coins(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(vested_coins.amount, Uint128::new(250_000_000_000));
        assert_eq!(vesting_coins.amount, Uint128::new(750_000_000_000));

        let delegated = account.total_delegations(&deps.storage).unwrap();
        assert_eq!(delegated, Uint128::new(90_000_000_000));

        let locked_coins = account.locked_coins(None, &env, &deps.storage).unwrap();
        // vesting - delegated
        assert_eq!(locked_coins.amount, Uint128::new(660_000_000_000));
        let spendable = account.spendable_coins(None, &env, &deps.storage).unwrap();
        assert_eq!(spendable.amount, Uint128::new(250_000_000_000));

        let ok = account.try_undelegate_from_mixnode(mix_id, &deps.storage);
        assert!(ok.is_ok());

        account
            .track_undelegation(mix_id, delegation.clone(), &mut deps.storage)
            .unwrap();

        let delegated = account.total_delegations(&deps.storage).unwrap();
        assert_eq!(delegated, Uint128::zero());

        assert_eq!(
            account.load_balance(&deps.storage).unwrap(),
            Uint128::new(1_000_000_000_000)
        );

        account
            .withdraw(
                &account.spendable_coins(None, &env, &deps.storage).unwrap(),
                &mut deps.storage,
            )
            .unwrap();

        assert_eq!(
            account.load_balance(&deps.storage).unwrap(),
            Uint128::new(750_000_000_000)
        );

        let withdrawn = account.load_withdrawn(&deps.storage).unwrap();
        assert_eq!(withdrawn, Uint128::new(250_000_000_000));

        let vested_coins = account.get_vested_coins(None, &env, &deps.storage).unwrap();
        let vesting_coins = account
            .get_vesting_coins(None, &env, &deps.storage)
            .unwrap();
        let locked_coins = account.locked_coins(None, &env, &deps.storage).unwrap();
        assert_eq!(vested_coins.amount, Uint128::new(250_000_000_000));
        assert_eq!(vesting_coins.amount, Uint128::new(750_000_000_000));
        assert_eq!(locked_coins.amount, Uint128::new(750_000_000_000));
        let spendable = account.spendable_coins(None, &env, &deps.storage).unwrap();
        assert_eq!(spendable.amount, Uint128::zero());

        let ok = account.try_delegate_to_mixnode(mix_id, delegation, &env, &mut deps.storage);
        assert!(ok.is_ok());

        let vested_coins = account.get_vested_coins(None, &env, &deps.storage).unwrap();
        let vesting_coins = account
            .get_vesting_coins(None, &env, &deps.storage)
            .unwrap();
        let locked_coins = account.locked_coins(None, &env, &deps.storage).unwrap();
        assert_eq!(vested_coins.amount, Uint128::new(250_000_000_000));
        assert_eq!(vesting_coins.amount, Uint128::new(750_000_000_000));
        let spendable = account.spendable_coins(None, &env, &deps.storage).unwrap();
        assert_eq!(spendable.amount, Uint128::zero());

        let delegated = account.total_delegations(&deps.storage).unwrap();

        assert_eq!(delegated, Uint128::new(90_000_000_000));

        // vesting - delegated_vesting - pledged_vesting
        assert_eq!(locked_coins.amount, Uint128::new(660_000_000_000));
    }

    #[test]
    fn test_percent_cap() {
        let mut deps = init_contract();
        let env = mock_env();

        let account = vesting_account_percent_fixture(&mut deps.storage, &env);

        assert_eq!(
            account.absolute_pledge_cap().unwrap(),
            Uint128::new(100_000_000_000)
        )
    }

    #[test]
    fn test_delegations() {
        let mut deps = init_contract();
        let env = mock_env();

        // let account = vesting_account_new_fixture(&mut deps.storage, &env);

        let msg = ExecuteMsg::CreateAccount {
            owner_address: "owner".to_string(),
            staking_address: Some("staking".to_string()),
            vesting_spec: None,
            cap: Some(PledgeCap::Absolute(Uint128::from(100_000_000_000u128))),
        };
        let info = mock_info("admin", &coins(1_000_000_000_000, TEST_COIN_DENOM));

        let _response = execute(deps.as_mut(), env.clone(), info, msg);
        let account = load_account(Addr::unchecked("owner"), &deps.storage)
            .unwrap()
            .unwrap();

        // Try delegating too much
        let err = account.try_delegate_to_mixnode(
            1,
            Coin {
                amount: Uint128::new(1_000_000_000_001),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let ok = account.try_delegate_to_mixnode(
            1,
            Coin {
                amount: Uint128::new(90_000_000_000),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        // Fails due to delegation locked delegation cap
        let ok = account.try_delegate_to_mixnode(
            1,
            Coin {
                amount: Uint128::new(20_000_000_000),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_err());

        let balance = account.load_balance(&deps.storage).unwrap();
        assert_eq!(balance, Uint128::new(910000000000));

        // Try delegating too much againcalca
        let err = account.try_delegate_to_mixnode(
            1,
            Coin {
                amount: Uint128::new(500_000_000_001),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let total_delegations = account.total_delegations_for_mix(1, &deps.storage).unwrap();
        assert_eq!(Uint128::new(90_000_000_000), total_delegations);
    }

    #[test]
    fn test_mixnode_bonds() {
        let mut deps = init_contract();
        let env = mock_env();

        let account = vesting_account_new_fixture(&mut deps.storage, &env);

        let mix_node = MixNode {
            host: "mix.node.org".to_string(),
            mix_port: 1789,
            verloc_port: 1790,
            http_api_port: 8000,
            sphinx_key: "sphinx".to_string(),
            identity_key: "identity".to_string(),
            version: "0.10.0".to_string(),
        };

        let cost_params = MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
            interval_operating_cost: Coin {
                denom: "NYM".to_string(),
                amount: Uint128::new(40),
            },
        };
        // Try delegating too much
        let err = account.try_bond_mixnode(
            mix_node.clone(),
            cost_params.clone(),
            MessageSignature::from(vec![1, 2, 3]),
            Coin {
                amount: Uint128::new(1_000_000_000_001),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let ok = account.try_bond_mixnode(
            mix_node.clone(),
            cost_params.clone(),
            MessageSignature::from(vec![1, 2, 3]),
            Coin {
                amount: Uint128::new(90_000_000_000),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        let balance = account.load_balance(&deps.storage).unwrap();
        assert_eq!(balance, Uint128::new(910_000_000_000));

        // Try delegating too much again
        let err = account.try_bond_mixnode(
            mix_node,
            cost_params,
            MessageSignature::from(vec![1, 2, 3]),
            Coin {
                amount: Uint128::new(10_000_000_001),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let pledge = account.load_mixnode_pledge(&deps.storage).unwrap().unwrap();
        assert_eq!(Uint128::new(90_000_000_000), pledge.amount().amount);
    }

    #[test]
    fn test_gateway_bonds() {
        let mut deps = init_contract();
        let env = mock_env();

        let account = vesting_account_new_fixture(&mut deps.storage, &env);

        let gateway = Gateway {
            host: "1.1.1.1".to_string(),
            mix_port: 1789,
            clients_port: 9000,
            location: "Sweden".to_string(),
            sphinx_key: "sphinx".to_string(),
            identity_key: "identity".to_string(),
            version: "0.10.0".to_string(),
        };

        // Try delegating too much
        let err = account.try_bond_gateway(
            gateway.clone(),
            MessageSignature::from(vec![1, 2, 3]),
            Coin {
                amount: Uint128::new(1_000_000_000_001),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let ok = account.try_bond_gateway(
            gateway.clone(),
            MessageSignature::from(vec![1, 2, 3]),
            Coin {
                amount: Uint128::new(90_000_000_000),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        let balance = account.load_balance(&deps.storage).unwrap();
        assert_eq!(balance, Uint128::new(910_000_000_000));

        // Try delegating too much again
        let err = account.try_bond_gateway(
            gateway,
            MessageSignature::from(vec![1, 2, 3]),
            Coin {
                amount: Uint128::new(500_000_000_001),
                denom: TEST_COIN_DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let pledge = account.load_gateway_pledge(&deps.storage).unwrap().unwrap();
        assert_eq!(Uint128::new(90_000_000_000), pledge.amount().amount);
    }

    #[test]
    fn test_delegations_cap() {
        let mut deps = init_contract();
        let mut env = mock_env();

        let vesting_period_length_secs = 3600;

        let account_creation_timestamp = 1650000000;
        let account_creation_blockheight = 12345;

        env.block.height = account_creation_blockheight;
        env.block.time = Timestamp::from_seconds(account_creation_timestamp);

        // lets define some helper timestamps

        // lets create our vesting account
        let periods = populate_vesting_periods(
            account_creation_timestamp,
            VestingSpecification::new(None, Some(vesting_period_length_secs), None),
        );

        let vesting_account = Account::save_new(
            Addr::unchecked("owner"),
            Some(Addr::unchecked("staking")),
            Coin {
                amount: Uint128::new(1_000_000_000_000),
                denom: TEST_COIN_DENOM.to_string(),
            },
            Timestamp::from_seconds(account_creation_timestamp),
            periods,
            Some(PledgeCap::Absolute(Uint128::from(100_000_000_000u128))),
            deps.as_mut().storage,
        )
        .unwrap();

        // time for some delegations

        let mix_id = 42;

        let delegation = Coin {
            amount: Uint128::new(42),
            denom: TEST_COIN_DENOM.to_string(),
        };

        // you can have at most `MAX_PER_MIX_DELEGATIONS` delegations so those should be fine
        for _ in 0..MAX_PER_MIX_DELEGATIONS {
            vesting_account
                .try_delegate_to_mixnode(mix_id, delegation.clone(), &env, &mut deps.storage)
                .unwrap();

            env.block.height += 1;
            env.block.time = env.block.time.plus_seconds(42);
        }

        // but the additional one is going to fail
        let res = vesting_account
            .try_delegate_to_mixnode(mix_id, delegation, &env, &mut deps.storage)
            .unwrap_err();

        assert_eq!(
            res,
            VestingContractError::TooManyDelegations {
                address: vesting_account.owner_address(),
                acc_id: vesting_account.storage_key(),
                mix_id,
                num: MAX_PER_MIX_DELEGATIONS,
                cap: MAX_PER_MIX_DELEGATIONS
            }
        );
    }
}
