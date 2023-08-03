// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::helpers::{mix_coin, mix_coins, vesting_owner};
use crate::support::setup::{TestSetup, MIX_DENOM};
use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::error::MixnetContractError;
use nym_mixnet_contract_common::{ContractStateParams, MixNodeCostParams};
use nym_mixnet_contract_common::{MixOwnershipResponse, QueryMsg as MixnetQueryMsg};
use nym_vesting_contract_common::{ExecuteMsg as VestingExecuteMsg, VestingContractError};

#[test]
fn decrease_mixnode_pledge_from_vesting_account_with_minimum_pledge() {
    let mut test = TestSetup::new_simple();
    let vesting_account = "vesting-account";

    // 0. get the minimum pledge amount
    let state_params: ContractStateParams = test
        .app
        .wrap()
        .query_wasm_smart(test.mixnet_contract(), &MixnetQueryMsg::GetStateParams {})
        .unwrap();
    let minimum_pledge = state_params.minimum_mixnode_pledge;

    // 1. create vesting account
    let create_msg = VestingExecuteMsg::CreateAccount {
        owner_address: vesting_account.to_string(),
        staking_address: None,
        vesting_spec: None,
        cap: None,
    };

    test.app
        .execute_contract(
            vesting_owner(),
            test.vesting_contract(),
            &create_msg,
            &mix_coins(1_000_000_000),
        )
        .unwrap();

    // 2. bond mixnode with the vesting account
    let pledge = minimum_pledge.clone();

    let cost_params = MixNodeCostParams {
        profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
        interval_operating_cost: mix_coin(40_000_000),
    };

    let (mix_node, owner_signature) = test.valid_mixnode_with_sig(
        vesting_account,
        Some(test.vesting_contract()),
        cost_params.clone(),
        pledge.clone(),
    );

    let bond_msg = VestingExecuteMsg::BondMixnode {
        mix_node,
        cost_params,
        owner_signature,
        amount: pledge.clone(),
    };
    test.app
        .execute_contract(
            Addr::unchecked(vesting_account),
            test.vesting_contract(),
            &bond_msg,
            &[],
        )
        .unwrap();

    // 3. try to decrease the pledge

    // trying to decrease by a zero amount - not valid
    let decrease_pledge_msg = VestingExecuteMsg::DecreasePledge {
        amount: mix_coin(0),
    };
    let res_zero = test
        .app
        .execute_contract(
            Addr::unchecked(vesting_account),
            test.vesting_contract(),
            &decrease_pledge_msg,
            &[],
        )
        .unwrap_err();

    assert_eq!(
        VestingContractError::EmptyFunds,
        res_zero.downcast().unwrap()
    );

    // trying to go below the cap - also not valid
    let amount = mix_coin(50_000);
    let decrease_pledge_msg = VestingExecuteMsg::DecreasePledge {
        amount: amount.clone(),
    };
    let res_below = test
        .app
        .execute_contract(
            Addr::unchecked(vesting_account),
            test.vesting_contract(),
            &decrease_pledge_msg,
            &[],
        )
        .unwrap_err();
    assert_eq!(
        MixnetContractError::InvalidPledgeReduction {
            current: pledge.amount,
            decrease_by: amount.amount,
            minimum: minimum_pledge.amount,
            denom: minimum_pledge.denom
        },
        res_below.downcast().unwrap()
    )
}

#[test]
fn decrease_mixnode_pledge_from_vesting_account_with_sufficient_pledge() {
    let mut test = TestSetup::new_simple();
    let vesting_account = "vesting-account";

    // 1. create vesting account
    let create_msg = VestingExecuteMsg::CreateAccount {
        owner_address: vesting_account.to_string(),
        staking_address: None,
        vesting_spec: None,
        cap: None,
    };

    test.app
        .execute_contract(
            vesting_owner(),
            test.vesting_contract(),
            &create_msg,
            &mix_coins(10_000_000_000),
        )
        .unwrap();

    // 2. bond mixnode with the vesting account
    let pledge = mix_coin(150_000_000);

    let cost_params = MixNodeCostParams {
        profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
        interval_operating_cost: mix_coin(40_000_000),
    };

    let (mix_node, owner_signature) = test.valid_mixnode_with_sig(
        vesting_account,
        Some(test.vesting_contract()),
        cost_params.clone(),
        pledge.clone(),
    );

    let bond_msg = VestingExecuteMsg::BondMixnode {
        mix_node,
        cost_params,
        owner_signature,
        amount: pledge,
    };
    test.app
        .execute_contract(
            Addr::unchecked(vesting_account),
            test.vesting_contract(),
            &bond_msg,
            &[],
        )
        .unwrap();

    // 3. try to decrease the pledge
    let before: MixOwnershipResponse = test
        .app
        .wrap()
        .query_wasm_smart(
            test.mixnet_contract(),
            &MixnetQueryMsg::GetOwnedMixnode {
                address: vesting_account.to_string(),
            },
        )
        .unwrap();
    let balance_before = test
        .app
        .wrap()
        .query_balance(test.vesting_contract(), MIX_DENOM)
        .unwrap();
    assert_eq!(balance_before.amount.u128(), 9_850_000_000);

    let decrease_pledge_msg = VestingExecuteMsg::DecreasePledge {
        amount: mix_coin(50_000_000),
    };
    test.app
        .execute_contract(
            Addr::unchecked(vesting_account),
            test.vesting_contract(),
            &decrease_pledge_msg,
            &[],
        )
        .unwrap();

    let after_decrease: MixOwnershipResponse = test
        .app
        .wrap()
        .query_wasm_smart(
            test.mixnet_contract(),
            &MixnetQueryMsg::GetOwnedMixnode {
                address: vesting_account.to_string(),
            },
        )
        .unwrap();

    // note: nothing has changed with the pledge because the event hasn't been resolved yet!
    assert_eq!(before.address, after_decrease.address);
    let before_details = before.mixnode_details.unwrap();
    let after_details = after_decrease.mixnode_details.unwrap();
    assert_eq!(
        before_details.rewarding_details,
        after_details.rewarding_details
    );
    assert_eq!(
        before_details.bond_information,
        after_details.bond_information
    );

    // but we have the pending change saved now!
    assert!(before_details.pending_changes.pledge_change.is_none());
    assert_eq!(Some(1), after_details.pending_changes.pledge_change);

    // 4. resolve events
    test.advance_mixnet_epoch();

    let balance_after = test
        .app
        .wrap()
        .query_balance(test.vesting_contract(), MIX_DENOM)
        .unwrap();
    assert_eq!(balance_after.amount.u128(), 9_900_000_000);
}
