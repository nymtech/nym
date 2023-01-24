use cosmwasm_std::Timestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod account;
pub use account::*;

use vesting_contract_common::messages::VestingSpecification;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct VestingPeriod {
    pub start_time: u64,
    pub period_seconds: u64,
}

impl VestingPeriod {
    pub fn end_time(&self) -> Timestamp {
        Timestamp::from_seconds(self.start_time + self.period_seconds)
    }
}

pub fn populate_vesting_periods(
    start_time: u64,
    vesting_spec: VestingSpecification,
) -> Vec<VestingPeriod> {
    let mut periods = Vec::with_capacity(vesting_spec.num_periods() as usize);
    for i in 0..vesting_spec.num_periods() {
        let period = VestingPeriod {
            start_time: start_time + i * vesting_spec.period_seconds(),
            period_seconds: vesting_spec.period_seconds(),
        };
        periods.push(period);
    }
    periods
}

#[cfg(test)]
mod tests {
    use crate::contract::*;
    use crate::errors::ContractError;
    use crate::storage::*;
    use crate::support::tests::helpers::vesting_account_percent_fixture;
    use crate::support::tests::helpers::{
        init_contract, vesting_account_mid_fixture, vesting_account_new_fixture, TEST_COIN_DENOM,
    };
    use crate::traits::DelegatingAccount;
    use crate::traits::VestingAccount;
    use crate::traits::{GatewayBondingAccount, MixnodeBondingAccount};
    use crate::vesting::{populate_vesting_periods, Account};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, coins, Addr, Coin, MessageInfo, Timestamp, Uint128};
    use mixnet_contract_common::mixnode::MixNodeCostParams;
    use mixnet_contract_common::{Gateway, MixNode, Percent};
    use vesting_contract_common::messages::{ExecuteMsg, VestingSpecification};
    use vesting_contract_common::Period;
    use vesting_contract_common::PledgeCap;

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
        let response = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
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
        let response = execute(deps.as_mut(), env.clone(), info.clone(), msg);
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
        let response = execute(deps.as_mut(), env.clone(), info, msg.clone());
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
        let response = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        // Only owner can transfer
        assert!(response.is_err());

        let msg = ExecuteMsg::WithdrawVestedCoins {
            amount: Coin {
                amount: Uint128::new(1),
                denom: "nym".to_string(),
            },
        };
        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + 100_000_000_000_000_000);
        let response = execute(deps.as_mut(), env.clone(), info, msg.clone());
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
            Err(ContractError::StakingAccountExists("vesting1".to_string())),
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
        let locked_coins = account.locked_coins(None, &env, &mut deps.storage).unwrap();
        assert_eq!(vested_coins.amount, Uint128::new(250_000_000_000));
        assert_eq!(vesting_coins.amount, Uint128::new(750_000_000_000));
        assert_eq!(locked_coins.amount, Uint128::new(750_000_000_000));
        let spendable = account
            .spendable_coins(None, &env, &mut deps.storage)
            .unwrap();
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

        let delegated_free = account
            .get_delegated_free(None, &env, &mut deps.storage)
            .unwrap();
        let delegated_vesting = account
            .get_delegated_vesting(None, &env, &mut deps.storage)
            .unwrap();

        assert_eq!(delegated_free.amount, Uint128::new(90_000_000_000));
        assert_eq!(delegated_vesting.amount, Uint128::zero());

        let locked_coins = account.locked_coins(None, &env, &mut deps.storage).unwrap();
        // vesting - delegated_vesting - pledged_vesting
        assert_eq!(locked_coins.amount, Uint128::new(750_000_000_000));
        let spendable = account
            .spendable_coins(None, &env, &mut deps.storage)
            .unwrap();
        assert_eq!(spendable.amount, Uint128::new(70_000_000_000));

        let ok = account.try_undelegate_from_mixnode(mix_id, &mut deps.storage);
        assert!(ok.is_ok());

        account
            .track_undelegation(mix_id, delegation.clone(), &mut deps.storage)
            .unwrap();

        let delegated_free = account
            .get_delegated_free(None, &env, &mut deps.storage)
            .unwrap();
        let delegated_vesting = account
            .get_delegated_vesting(None, &env, &mut deps.storage)
            .unwrap();

        assert_eq!(delegated_free.amount, Uint128::zero());
        assert_eq!(delegated_vesting.amount, Uint128::zero());

        assert_eq!(
            account.load_balance(&deps.storage).unwrap(),
            Uint128::new(1000_000_000_000)
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
        let locked_coins = account.locked_coins(None, &env, &mut deps.storage).unwrap();
        assert_eq!(vested_coins.amount, Uint128::new(250_000_000_000));
        assert_eq!(vesting_coins.amount, Uint128::new(750_000_000_000));
        assert_eq!(locked_coins.amount, Uint128::new(750_000_000_000));
        let spendable = account
            .spendable_coins(None, &env, &mut deps.storage)
            .unwrap();
        assert_eq!(spendable.amount, Uint128::zero());

        let ok =
            account.try_delegate_to_mixnode(mix_id, delegation.clone(), &env, &mut deps.storage);
        assert!(ok.is_ok());

        let vested_coins = account.get_vested_coins(None, &env, &deps.storage).unwrap();
        let vesting_coins = account
            .get_vesting_coins(None, &env, &deps.storage)
            .unwrap();
        let locked_coins = account.locked_coins(None, &env, &mut deps.storage).unwrap();
        assert_eq!(vested_coins.amount, Uint128::new(250_000_000_000));
        assert_eq!(vesting_coins.amount, Uint128::new(750_000_000_000));
        let spendable = account
            .spendable_coins(None, &env, &mut deps.storage)
            .unwrap();
        assert_eq!(spendable.amount, Uint128::zero());

        let delegated_free = account
            .get_delegated_free(None, &env, &mut deps.storage)
            .unwrap();
        let delegated_vesting = account
            .get_delegated_vesting(None, &env, &mut deps.storage)
            .unwrap();

        assert_eq!(delegated_free.amount, Uint128::zero());
        assert_eq!(delegated_vesting.amount, Uint128::new(90_000_000_000));

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

        let _response = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
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

        let account = load_account(Addr::unchecked("owner"), &deps.storage)
            .unwrap()
            .unwrap();

        // Current period -> block_time: None
        let delegated_free = account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(Uint128::new(0), delegated_free.amount);

        let delegated_vesting = account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(
            account.total_delegations(&deps.storage).unwrap() - delegated_free.amount,
            delegated_vesting.amount
        );

        // All periods
        for (i, period) in account.periods().iter().enumerate() {
            let delegated_free = account
                .get_delegated_free(
                    Some(Timestamp::from_seconds(period.start_time + 1)),
                    &env,
                    &deps.storage,
                )
                .unwrap();
            assert_eq!(
                (account.tokens_per_period().unwrap() * i as u128)
                    .min(account.total_delegations(&deps.storage).unwrap().u128()),
                delegated_free.amount.u128()
            );

            let delegated_vesting = account
                .get_delegated_vesting(
                    Some(Timestamp::from_seconds(period.start_time + 1)),
                    &env,
                    &deps.storage,
                )
                .unwrap();
            assert_eq!(
                account.total_delegations(&deps.storage).unwrap() - delegated_free.amount,
                delegated_vesting.amount
            );
        }

        let delegated_free = account
            .get_delegated_free(
                Some(Timestamp::from_seconds(1764416964)),
                &env,
                &deps.storage,
            )
            .unwrap();
        assert_eq!(total_delegations, delegated_free.amount);

        let delegated_free = account
            .get_delegated_vesting(
                Some(Timestamp::from_seconds(1764416964)),
                &env,
                &deps.storage,
            )
            .unwrap();
        assert_eq!(Uint128::zero(), delegated_free.amount);
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
            "alice".to_string(),
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
            "alice".to_string(),
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
            "alice".to_string(),
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

        // Current period -> block_time: None
        let bonded_free = account.get_pledged_free(None, &env, &deps.storage).unwrap();
        assert_eq!(Uint128::new(0), bonded_free.amount);

        let bonded_vesting = account
            .get_pledged_vesting(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(
            pledge.amount().amount - bonded_free.amount,
            bonded_vesting.amount
        );

        // All periods
        for (i, period) in account.periods().iter().enumerate() {
            let bonded_free = account
                .get_pledged_free(
                    Some(Timestamp::from_seconds(period.start_time + 1)),
                    &env,
                    &deps.storage,
                )
                .unwrap();
            assert_eq!(
                (account.tokens_per_period().unwrap() * i as u128)
                    .min(pledge.amount().amount.u128()),
                bonded_free.amount.u128()
            );

            let bonded_vesting = account
                .get_pledged_vesting(
                    Some(Timestamp::from_seconds(period.start_time + 1)),
                    &env,
                    &deps.storage,
                )
                .unwrap();
            assert_eq!(
                pledge.amount().amount - bonded_free.amount,
                bonded_vesting.amount
            );
        }

        let bonded_free = account
            .get_pledged_free(
                Some(Timestamp::from_seconds(1764416964)),
                &env,
                &deps.storage,
            )
            .unwrap();
        assert_eq!(pledge.amount().amount, bonded_free.amount);

        let bonded_vesting = account
            .get_pledged_vesting(
                Some(Timestamp::from_seconds(1764416964)),
                &env,
                &deps.storage,
            )
            .unwrap();
        assert_eq!(Uint128::zero(), bonded_vesting.amount);
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
            "alice".to_string(),
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
            "alice".to_string(),
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
            "alice".to_string(),
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

        // Current period -> block_time: None
        let bonded_free = account.get_pledged_free(None, &env, &deps.storage).unwrap();
        assert_eq!(Uint128::new(0), bonded_free.amount);

        let bonded_vesting = account
            .get_pledged_vesting(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(
            pledge.amount().amount - bonded_free.amount,
            bonded_vesting.amount
        );

        // All periods
        for (i, period) in account.periods().iter().enumerate() {
            let bonded_free = account
                .get_pledged_free(
                    Some(Timestamp::from_seconds(period.start_time + 1)),
                    &env,
                    &deps.storage,
                )
                .unwrap();
            assert_eq!(
                (account.tokens_per_period().unwrap() * i as u128)
                    .min(pledge.amount().amount.u128()),
                bonded_free.amount.u128()
            );

            let bonded_vesting = account
                .get_pledged_vesting(
                    Some(Timestamp::from_seconds(period.start_time + 1)),
                    &env,
                    &deps.storage,
                )
                .unwrap();
            assert_eq!(
                pledge.amount().amount - bonded_free.amount,
                bonded_vesting.amount
            );
        }

        let bonded_free = account
            .get_pledged_free(
                Some(Timestamp::from_seconds(1764416964)),
                &env,
                &deps.storage,
            )
            .unwrap();
        assert_eq!(pledge.amount().amount, bonded_free.amount);

        let bonded_vesting = account
            .get_pledged_vesting(
                Some(Timestamp::from_seconds(1764416964)),
                &env,
                &deps.storage,
            )
            .unwrap();
        assert_eq!(Uint128::zero(), bonded_vesting.amount);
    }

    #[test]
    fn delegated_free() {
        let mut deps = init_contract();
        let mut env = mock_env();

        let vesting_period_length_secs = 3600;

        let account_creation_timestamp = 1650000000;
        let account_creation_blockheight = 12345;

        // this value is completely arbitrary, I just wanted to keep consistent
        // (and make sure that if block timestamp increases so does the block height)
        let blocks_per_period = 100;

        env.block.height = account_creation_blockheight;
        env.block.time = Timestamp::from_seconds(account_creation_timestamp);

        // lets define some helper timestamps

        // our account is set to be created after 2 vesting periods already passed
        let vesting_start_blockheight = account_creation_blockheight - 2 * blocks_per_period;
        let vesting_start_timestamp = account_creation_timestamp - 2 * vesting_period_length_secs;

        let vesting_period2_start_blockheight = vesting_start_blockheight + blocks_per_period;
        let vesting_period2_start_timestamp = vesting_start_timestamp + vesting_period_length_secs;

        // this vesting period is currently in progress!
        let vesting_period3_start_blockheight =
            vesting_period2_start_blockheight + blocks_per_period;
        let vesting_period3_start_timestamp =
            vesting_period2_start_timestamp + vesting_period_length_secs;

        // and this one is in the future! (in relation to account creation)
        let vesting_period4_start_blockheight =
            vesting_period3_start_blockheight + blocks_per_period;
        let vesting_period4_start_timestamp =
            vesting_period3_start_timestamp + vesting_period_length_secs;

        // lets create our vesting account
        let periods = populate_vesting_periods(
            vesting_start_timestamp,
            VestingSpecification::new(None, Some(vesting_period_length_secs), None),
        );

        let vesting_account = Account::new(
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
            amount: Uint128::new(90_000_000_000),
            denom: TEST_COIN_DENOM.to_string(),
        };

        // delegate explicitly at the time the account was created
        // (i.e. after 2 vesting periods already elapsed)
        env.block.height = account_creation_blockheight;
        env.block.time = Timestamp::from_seconds(account_creation_timestamp);
        let ok = vesting_account.try_delegate_to_mixnode(
            mix_id,
            delegation.clone(),
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        let vested_coins = vesting_account
            .get_vested_coins(None, &env, &deps.storage)
            .unwrap();
        let vesting_coins = vesting_account
            .get_vesting_coins(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(vested_coins.amount, Uint128::new(250_000_000_000));
        assert_eq!(vesting_coins.amount, Uint128::new(750_000_000_000));
        let delegated_free = vesting_account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();

        // all good so far
        assert_eq!(delegated_free.amount, Uint128::new(90_000_000_000));
        assert_eq!(delegated_vesting.amount, Uint128::zero());

        // some time passes, and we're now into the next vesting period, more of our coins got unlocked!
        env.block.height = vesting_period4_start_blockheight;
        env.block.time = Timestamp::from_seconds(vesting_period4_start_timestamp);

        let vested_coins = vesting_account
            .get_vested_coins(None, &env, &deps.storage)
            .unwrap();
        let vesting_coins = vesting_account
            .get_vesting_coins(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(vested_coins.amount, Uint128::new(375_000_000_000));
        assert_eq!(vesting_coins.amount, Uint128::new(625_000_000_000));

        // and nothing about our existing delegation changed
        let delegated_free = vesting_account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(delegated_free.amount, Uint128::new(90_000_000_000));
        assert_eq!(delegated_vesting.amount, Uint128::zero());

        // however, create a new delegation now in this brand new vesting period
        let delegation = Coin {
            amount: Uint128::new(50_000_000_000),
            denom: TEST_COIN_DENOM.to_string(),
        };
        let ok = vesting_account.try_delegate_to_mixnode(
            mix_id,
            delegation.clone(),
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        // we're still good here, we have delegated in total 140M from our vested tokens!
        let delegated_free = vesting_account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(delegated_free.amount, Uint128::new(140_000_000_000));
        assert_eq!(delegated_vesting.amount, Uint128::zero());

        // but let's ask now a different question:
        // how many vested tokens have I had delegated during vesting period3? (i.e. after account creation)
        let delegated_free = vesting_account
            .get_delegated_free(
                Some(Timestamp::from_seconds(vesting_period3_start_timestamp)),
                &env,
                &deps.storage,
            )
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(
                Some(Timestamp::from_seconds(vesting_period3_start_timestamp)),
                &env,
                &deps.storage,
            )
            .unwrap();

        // returns 90M as the 50M delegation didn't exist at this point of time
        assert_eq!(delegated_free.amount, Uint128::new(90_000_000_000));

        // the 50M delegation wasn't a thing here for VESTING tokens either
        assert_eq!(delegated_vesting.amount, Uint128::zero());
    }

    #[test]
    fn vesting_cap_escape() {
        let mut deps = init_contract();
        let mut env = mock_env();

        let vesting_period_length_secs = 3600;
        let blocks_per_period = 100;

        let account_creation_timestamp = 1650000000;
        let vesting_start_timestamp = 1650000000;

        let account_creation_blockheight = 12345;

        env.block.height = account_creation_blockheight;
        env.block.time = Timestamp::from_seconds(account_creation_timestamp);

        let periods = populate_vesting_periods(
            vesting_start_timestamp,
            VestingSpecification::new(None, Some(vesting_period_length_secs), None),
        );

        let vesting_account = Account::new(
            Addr::unchecked("owner"),
            Some(Addr::unchecked("staking")),
            Coin {
                amount: Uint128::new(40_000_000_000_000),
                denom: TEST_COIN_DENOM.to_string(),
            },
            Timestamp::from_seconds(account_creation_timestamp),
            periods,
            Some(PledgeCap::Percent(
                Percent::from_percentage_value(10).unwrap(),
            )),
            deps.as_mut().storage,
        )
        .unwrap();

        let mix_id = 42;

        let delegation = Coin {
            amount: Uint128::new(5_000_000_000_000),
            denom: TEST_COIN_DENOM.to_string(),
        };

        // delegate explicitly at the time the account was created
        // (i.e. after 2 vesting periods already elapsed)
        env.block.height = account_creation_blockheight;
        env.block.time = Timestamp::from_seconds(account_creation_timestamp);
        let err = vesting_account.try_delegate_to_mixnode(
            mix_id,
            delegation.clone(),
            &env,
            &mut deps.storage,
        );
        // Can't delegate due to cap
        assert!(err.is_err());

        let delegated_free = vesting_account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();

        assert_eq!(delegated_free.amount, Uint128::new(0));
        assert_eq!(delegated_vesting.amount, Uint128::new(0));

        let delegation = Coin {
            amount: Uint128::new(4_000_000_000_000),
            denom: TEST_COIN_DENOM.to_string(),
        };

        // delegate explicitly at the time the account was created
        // (i.e. after 2 vesting periods already elapsed)
        env.block.height = account_creation_blockheight;
        env.block.time = Timestamp::from_seconds(account_creation_timestamp);
        let ok = vesting_account.try_delegate_to_mixnode(
            mix_id,
            delegation.clone(),
            &env,
            &mut deps.storage,
        );

        // Max delegation allowed by cap
        assert!(ok.is_ok());

        let delegated_free = vesting_account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();

        assert_eq!(delegated_free.amount, Uint128::new(0));
        assert_eq!(delegated_vesting.amount, Uint128::new(4_000_000_000_000));

        // Advance vesting period
        env.block.height += blocks_per_period * 2;
        env.block.time =
            Timestamp::from_seconds(env.block.time.seconds() + vesting_period_length_secs * 2);

        let delegated_free = vesting_account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();

        let spendable_coins = vesting_account
            .spendable_coins(None, &env, &deps.storage)
            .unwrap();

        // let balance = vesting_account.load_balance(&deps.storage).unwrap();
        // assert_eq!(balance, Uint128::new(6_000_000_000_000));

        // Entire delegation amount is no free, and more can be delegated from the locked tokens
        assert_eq!(delegated_free.amount, Uint128::new(4_000_000_000_000));
        assert_eq!(delegated_vesting.amount, Uint128::new(0));
        assert_eq!(spendable_coins.amount, Uint128::new(6_000_000_000_000));

        let total_pledged_locked = vesting_account
            .total_pledged_locked(&deps.storage, &env)
            .unwrap();
        assert_eq!(total_pledged_locked, Uint128::new(0));

        let delegation = Coin {
            amount: Uint128::new(5_000_000_000_000),
            denom: TEST_COIN_DENOM.to_string(),
        };

        let err = vesting_account.try_delegate_to_mixnode(
            mix_id,
            delegation.clone(),
            &env,
            &mut deps.storage,
        );

        // Can't delegate due to cap
        assert!(err.is_err());

        let delegation = Coin {
            amount: Uint128::new(4_000_000_000_000),
            denom: TEST_COIN_DENOM.to_string(),
        };

        let ok = vesting_account.try_delegate_to_mixnode(
            mix_id,
            delegation.clone(),
            &env,
            &mut deps.storage,
        );
        // Delegate max allowed amount
        assert!(ok.is_ok());

        let delegated_free = vesting_account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();
        let total_pledged_locked = vesting_account
            .total_pledged_locked(&deps.storage, &env)
            .unwrap();
        let spendable_coins = vesting_account
            .spendable_coins(None, &env, &deps.storage)
            .unwrap();
        // There is an additional problem here since the cap calculation does not take vesting periods into account
        assert_eq!(total_pledged_locked, Uint128::new(0));

        // Entire delegation amount is now free, and more can be delegated from the locked tokens
        assert_eq!(delegated_free.amount, Uint128::new(8_000_000_000_000));
        assert_eq!(delegated_vesting.amount, Uint128::new(0_000_000_000_000));
        assert_eq!(spendable_coins.amount, Uint128::new(2_000_000_000_000));

        // Delegate some more to saturate the cap
        env.block.height += 1;
        env.block.time = Timestamp::from_seconds(env.block.time.seconds() + 36);

        let delegation = Coin {
            amount: Uint128::new(4_000_000_000_000),
            denom: TEST_COIN_DENOM.to_string(),
        };

        vesting_account
            .try_delegate_to_mixnode(mix_id, delegation.clone(), &env, &mut deps.storage)
            .unwrap();

        let delegated_free = vesting_account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();
        let total_pledged_locked = vesting_account
            .total_pledged_locked(&deps.storage, &env)
            .unwrap();

        // There is an additional problem here since the cap calculation does not take vesting periods into account
        assert_eq!(total_pledged_locked, Uint128::new(4_000_000_000_000));

        assert_eq!(delegated_free.amount, Uint128::new(8_000_000_000_000));
        assert_eq!(delegated_vesting.amount, Uint128::new(4_000_000_000_000));

        let spendable_coins = vesting_account
            .spendable_coins(None, &env, &deps.storage)
            .unwrap();

        let locked_coins = vesting_account
            .locked_coins(None, &env, &deps.storage)
            .unwrap();

        assert_eq!(spendable_coins.amount, Uint128::new(2_000_000_000_000));
        assert_eq!(locked_coins.amount, Uint128::new(26_000_000_000_000));

        // Check that user can't withdraw free coins if they're delegated

        let info = MessageInfo {
            sender: Addr::unchecked("owner"),
            funds: vec![],
        };

        let ok = try_withdraw_vested_coins(
            Coin::new(1_000_000_000_000, TEST_COIN_DENOM),
            env.clone(),
            info,
            deps.as_mut(),
        );

        assert!(ok.is_ok());

        let spendable_coins = vesting_account
            .spendable_coins(None, &env, &deps.storage)
            .unwrap();

        assert_eq!(spendable_coins.amount, Uint128::new(1_000_000_000_000));

        let delegated_free = vesting_account
            .get_delegated_free(None, &env, &deps.storage)
            .unwrap();
        let delegated_vesting = vesting_account
            .get_delegated_vesting(None, &env, &deps.storage)
            .unwrap();
        let total_pledged_locked = vesting_account
            .total_pledged_locked(&deps.storage, &env)
            .unwrap();

        assert_eq!(total_pledged_locked, Uint128::new(4_000_000_000_000));

        assert_eq!(delegated_free.amount, Uint128::new(8_000_000_000_000));
        assert_eq!(delegated_vesting.amount, Uint128::new(4_000_000_000_000));

        let delegation = Coin {
            amount: Uint128::new(1_000_000_000_000),
            denom: TEST_COIN_DENOM.to_string(),
        };

        let err = vesting_account.try_delegate_to_mixnode(
            mix_id,
            delegation.clone(),
            &env,
            &mut deps.storage,
        );

        assert!(err.is_err());
    }
}
