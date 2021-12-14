use crate::contract::VESTING_PERIOD;
use cosmwasm_std::{Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod account;
pub use account::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VestingPeriod {
    pub start_time: u64,
}

impl VestingPeriod {
    pub fn end_time(&self) -> Timestamp {
        Timestamp::from_seconds(self.start_time + VESTING_PERIOD)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PledgeData {
    amount: Uint128,
    block_time: Timestamp,
}

pub fn populate_vesting_periods(start_time: u64, n: usize) -> Vec<VestingPeriod> {
    let mut periods = Vec::with_capacity(n as usize);
    for i in 0..n {
        let period = VestingPeriod {
            start_time: start_time + i as u64 * VESTING_PERIOD,
        };
        periods.push(period);
    }
    periods
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, NUM_VESTING_PERIODS, VESTING_PERIOD};
    use crate::messages::ExecuteMsg;
    use crate::storage::load_account;
    use crate::support::tests::helpers::{init_contract, vesting_account_fixture};
    use crate::traits::DelegatingAccount;
    use crate::traits::VestingAccount;
    use crate::traits::{GatewayBondingAccount, MixnodeBondingAccount};
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};
    use mixnet_contract::{Gateway, MixNode};

    #[test]
    fn test_account_creation() {
        let mut deps = init_contract();
        let env = mock_env();
        let account = vesting_account_fixture(&mut deps.storage, &env);
        let created_account = load_account(&account.owner_address(), &deps.storage).unwrap();
        let created_account_test = load_account(&Addr::unchecked("owner"), &deps.storage).unwrap();
        let created_account_test_by_staking =
            load_account(&Addr::unchecked("staking"), &deps.storage).unwrap();
        assert_eq!(Some(&account), created_account.as_ref());
        assert_eq!(Some(&account), created_account_test.as_ref());
        assert_eq!(created_account_test_by_staking, created_account_test);
        assert_eq!(
            account.load_balance(&deps.storage).unwrap(),
            Uint128::new(1_000_000_000_000)
        );
        assert_eq!(
            account.load_balance(&deps.storage).unwrap(),
            Uint128::new(1_000_000_000_000)
        )
    }

    #[test]
    fn test_ownership_transfer() {
        let mut deps = init_contract();
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let account = vesting_account_fixture(&mut deps.storage, &env);
        let msg = ExecuteMsg::TransferOwnership {
            to_address: "new_owner".to_string(),
        };
        let _response = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let new_owner_account = load_account(&Addr::unchecked("new_owner"), &deps.storage).unwrap().unwrap();
        assert_eq!(new_owner_account.load_balance(&deps.storage), account.load_balance(&deps.storage));
        // Check old account is gone
        let old_owner_account = load_account(&Addr::unchecked("owner"), &deps.storage).unwrap();
        assert!(old_owner_account.is_none());
        // Not the owner
        let response = execute(deps.as_mut(), env, info, msg);
        assert!(response.is_err())
    }

    #[test]
    fn test_period_logic() {
        let mut deps = init_contract();
        let env = mock_env();

        let account = vesting_account_fixture(&mut deps.storage, &env);

        assert_eq!(account.periods().len(), NUM_VESTING_PERIODS as usize);
        assert_eq!(account.periods().len(), 8);

        let current_period = account.get_current_vesting_period(Timestamp::from_seconds(0));
        assert_eq!(0, current_period);

        let block_time =
            Timestamp::from_seconds(account.start_time().seconds() + VESTING_PERIOD + 1);
        let current_period = account.get_current_vesting_period(block_time);
        assert_eq!(current_period, 1);
        let vested_coins = account.get_vested_coins(Some(block_time), &env).unwrap();
        let vesting_coins = account.get_vesting_coins(Some(block_time), &env).unwrap();
        assert_eq!(
            vested_coins.amount,
            Uint128::new(
                account.get_original_vesting().amount.u128() / NUM_VESTING_PERIODS as u128
            )
        );
        assert_eq!(
            vesting_coins.amount,
            Uint128::new(
                account.get_original_vesting().amount.u128()
                    - account.get_original_vesting().amount.u128() / NUM_VESTING_PERIODS as u128
            )
        );

        let block_time =
            Timestamp::from_seconds(account.start_time().seconds() + 5 * VESTING_PERIOD + 1);
        let current_period = account.get_current_vesting_period(block_time);
        assert_eq!(current_period, 5);
        let vested_coins = account.get_vested_coins(Some(block_time), &env).unwrap();
        let vesting_coins = account.get_vesting_coins(Some(block_time), &env).unwrap();
        assert_eq!(
            vested_coins.amount,
            Uint128::new(
                5 * account.get_original_vesting().amount.u128() / NUM_VESTING_PERIODS as u128
            )
        );
        assert_eq!(
            vesting_coins.amount,
            Uint128::new(
                account.get_original_vesting().amount.u128()
                    - 5 * account.get_original_vesting().amount.u128()
                        / NUM_VESTING_PERIODS as u128
            )
        );
    }

    #[test]
    fn test_delegations() {
        let mut deps = init_contract();
        let env = mock_env();

        let account = vesting_account_fixture(&mut deps.storage, &env);

        // Try delegating too much
        let err = account.try_delegate_to_mixnode(
            "alice".to_string(),
            Coin {
                amount: Uint128::new(1_000_000_000_001),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let ok = account.try_delegate_to_mixnode(
            "alice".to_string(),
            Coin {
                amount: Uint128::new(500_000_000_000),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        let balance = account.load_balance(&deps.storage).unwrap();
        assert_eq!(balance, Uint128::new(500_000_000_000));

        // Try delegating too much again
        let err = account.try_delegate_to_mixnode(
            "alice".to_string(),
            Coin {
                amount: Uint128::new(500_000_000_001),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let total_delegations = account
            .total_delegations_for_mix("alice".to_string(), &deps.storage)
            .unwrap();
        assert_eq!(Uint128::new(500_000_000_000), total_delegations);

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

        let account = vesting_account_fixture(&mut deps.storage, &env);

        let mix_node = MixNode {
            host: "mix.node.org".to_string(),
            mix_port: 1789,
            verloc_port: 1790,
            http_api_port: 8000,
            sphinx_key: "sphinx".to_string(),
            identity_key: "identity".to_string(),
            version: "0.10.0".to_string(),
            profit_margin_percent: 10,
        };
        // Try delegating too much
        let err = account.try_bond_mixnode(
            mix_node.clone(),
            "alice".to_string(),
            Coin {
                amount: Uint128::new(1_000_000_000_001),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let ok = account.try_bond_mixnode(
            mix_node.clone(),
            "alice".to_string(),
            Coin {
                amount: Uint128::new(500_000_000_000),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        let balance = account.load_balance(&deps.storage).unwrap();
        assert_eq!(balance, Uint128::new(500_000_000_000));

        // Try delegating too much again
        let err = account.try_bond_mixnode(
            mix_node,
            "alice".to_string(),
            Coin {
                amount: Uint128::new(500_000_000_001),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let pledge = account.load_mixnode_pledge(&deps.storage).unwrap().unwrap();
        assert_eq!(Uint128::new(500_000_000_000), pledge.amount);

        // Current period -> block_time: None
        let bonded_free = account.get_pledged_free(None, &env, &deps.storage).unwrap();
        assert_eq!(Uint128::new(0), bonded_free.amount);

        let bonded_vesting = account
            .get_pledged_vesting(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(pledge.amount - bonded_free.amount, bonded_vesting.amount);

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
                (account.tokens_per_period().unwrap() * i as u128).min(pledge.amount.u128()),
                bonded_free.amount.u128()
            );

            let bonded_vesting = account
                .get_pledged_vesting(
                    Some(Timestamp::from_seconds(period.start_time + 1)),
                    &env,
                    &deps.storage,
                )
                .unwrap();
            assert_eq!(pledge.amount - bonded_free.amount, bonded_vesting.amount);
        }

        let bonded_free = account
            .get_pledged_free(
                Some(Timestamp::from_seconds(1764416964)),
                &env,
                &deps.storage,
            )
            .unwrap();
        assert_eq!(pledge.amount, bonded_free.amount);

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

        let account = vesting_account_fixture(&mut deps.storage, &env);

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
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let ok = account.try_bond_gateway(
            gateway.clone(),
            "alice".to_string(),
            Coin {
                amount: Uint128::new(500_000_000_000),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        let balance = account.load_balance(&deps.storage).unwrap();
        assert_eq!(balance, Uint128::new(500_000_000_000));

        // Try delegating too much again
        let err = account.try_bond_gateway(
            gateway,
            "alice".to_string(),
            Coin {
                amount: Uint128::new(500_000_000_001),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let pledge = account.load_gateway_pledge(&deps.storage).unwrap().unwrap();
        assert_eq!(Uint128::new(500_000_000_000), pledge.amount);

        // Current period -> block_time: None
        let bonded_free = account.get_pledged_free(None, &env, &deps.storage).unwrap();
        assert_eq!(Uint128::new(0), bonded_free.amount);

        let bonded_vesting = account
            .get_pledged_vesting(None, &env, &deps.storage)
            .unwrap();
        assert_eq!(pledge.amount - bonded_free.amount, bonded_vesting.amount);

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
                (account.tokens_per_period().unwrap() * i as u128).min(pledge.amount.u128()),
                bonded_free.amount.u128()
            );

            let bonded_vesting = account
                .get_pledged_vesting(
                    Some(Timestamp::from_seconds(period.start_time + 1)),
                    &env,
                    &deps.storage,
                )
                .unwrap();
            assert_eq!(pledge.amount - bonded_free.amount, bonded_vesting.amount);
        }

        let bonded_free = account
            .get_pledged_free(
                Some(Timestamp::from_seconds(1764416964)),
                &env,
                &deps.storage,
            )
            .unwrap();
        assert_eq!(pledge.amount, bonded_free.amount);

        let bonded_vesting = account
            .get_pledged_vesting(
                Some(Timestamp::from_seconds(1764416964)),
                &env,
                &deps.storage,
            )
            .unwrap();
        assert_eq!(Uint128::zero(), bonded_vesting.amount);
    }
}
