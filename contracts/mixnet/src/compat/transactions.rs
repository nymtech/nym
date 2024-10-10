// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnodes::helpers::get_mixnode_details_by_owner;
use crate::mixnodes::transactions::{
    try_decrease_mixnode_pledge, try_increase_mixnode_pledge, try_update_mixnode_cost_params,
};
use crate::nodes::helpers::get_node_details_by_owner;
use crate::nodes::transactions::{
    try_decrease_nym_node_pledge, try_increase_nym_node_pledge, try_update_nym_node_cost_params,
};
use crate::rewards::transactions::{
    try_withdraw_mixnode_operator_reward, try_withdraw_nym_node_operator_reward,
};
use crate::support::helpers::{
    ensure_operating_cost_within_range, ensure_profit_margin_within_range,
};
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::NodeCostParams;

pub(crate) fn try_increase_pledge(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    // check if owns mixnode or nymnode and change accordingly
    if let Some(nym_node_details) = get_node_details_by_owner(deps.storage, info.sender.clone())? {
        try_increase_nym_node_pledge(deps, env, info.funds, nym_node_details)
    } else if let Some(legacy_mixnode_details) =
        get_mixnode_details_by_owner(deps.storage, info.sender.clone())?
    {
        try_increase_mixnode_pledge(deps, env, info.funds, legacy_mixnode_details)
    } else {
        Err(MixnetContractError::NoAssociatedNodeBond { owner: info.sender })
    }
}

pub fn try_decrease_pledge(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    decrease_by: Coin,
) -> Result<Response, MixnetContractError> {
    // check if owns mixnode or nymnode and change accordingly
    if let Some(nym_node_details) = get_node_details_by_owner(deps.storage, info.sender.clone())? {
        try_decrease_nym_node_pledge(deps, env, decrease_by, nym_node_details)
    } else if let Some(legacy_mixnode_details) =
        get_mixnode_details_by_owner(deps.storage, info.sender.clone())?
    {
        try_decrease_mixnode_pledge(deps, env, decrease_by, legacy_mixnode_details)
    } else {
        Err(MixnetContractError::NoAssociatedNodeBond { owner: info.sender })
    }
}

pub(crate) fn try_update_cost_params(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    new_costs: NodeCostParams,
) -> Result<Response, MixnetContractError> {
    // ensure the profit margin is within the defined range
    ensure_profit_margin_within_range(deps.storage, new_costs.profit_margin_percent)?;

    // ensure the operating cost is within the defined range
    ensure_operating_cost_within_range(deps.storage, &new_costs.interval_operating_cost)?;

    // check if owns mixnode or nymnode and change accordingly
    if let Some(nym_node_details) = get_node_details_by_owner(deps.storage, info.sender.clone())? {
        try_update_nym_node_cost_params(deps, env, new_costs, nym_node_details)
    } else if let Some(legacy_mixnode_details) =
        get_mixnode_details_by_owner(deps.storage, info.sender.clone())?
    {
        try_update_mixnode_cost_params(deps, env, new_costs, legacy_mixnode_details)
    } else {
        Err(MixnetContractError::NoAssociatedNodeBond { owner: info.sender })
    }
}

pub(crate) fn try_withdraw_operator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    // check if owns mixnode or nymnode and change accordingly
    if let Some(nym_node_details) = get_node_details_by_owner(deps.storage, info.sender.clone())? {
        try_withdraw_nym_node_operator_reward(deps, nym_node_details)
    } else if let Some(legacy_mixnode_details) =
        get_mixnode_details_by_owner(deps.storage, info.sender.clone())?
    {
        try_withdraw_mixnode_operator_reward(deps, legacy_mixnode_details)
    } else {
        Err(MixnetContractError::NoAssociatedNodeBond { owner: info.sender })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod increasing_pledge {
        use super::*;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::Addr;

        #[test]
        fn when_there_are_no_nodes() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let sender = mock_info("owner", &[test.coin(100000)]);
            let err = test.execute_fn(try_increase_pledge, sender).unwrap_err();

            assert_eq!(
                MixnetContractError::NoAssociatedNodeBond {
                    owner: Addr::unchecked("owner"),
                },
                err
            );

            Ok(())
        }

        #[test]
        fn for_legacy_mixnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let node_id = test.add_legacy_mixnode("owner", Some(100_000_000u128.into()));
            let sender = mock_info("owner", &[test.coin(100_000)]);
            test.assert_simple_execution(try_increase_pledge, sender);

            let after = test.mixnode_by_id(node_id).unwrap();
            let event_id = after.pending_changes.pledge_change.unwrap();
            assert_eq!(event_id, 1);

            test.execute_all_pending_events();

            let after = test.mixnode_by_id(node_id).unwrap();
            assert_eq!(
                after.bond_information.original_pledge.amount.u128(),
                100_100_000u128
            );

            Ok(())
        }

        #[test]
        fn for_legacy_gateway() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            test.add_legacy_gateway("owner", None);
            let sender = mock_info("owner", &[test.coin(100000)]);
            let err = test.execute_fn(try_increase_pledge, sender).unwrap_err();

            // it's illegal to increase pledge for legacy gateways
            assert_eq!(
                MixnetContractError::NoAssociatedNodeBond {
                    owner: Addr::unchecked("owner"),
                },
                err
            );

            Ok(())
        }

        #[test]
        fn for_nymnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let node_id = test.add_dummy_nymnode("owner", Some(100_000_000u128.into()));
            let sender = mock_info("owner", &[test.coin(100_000)]);
            test.assert_simple_execution(try_increase_pledge, sender);

            let after = test.nymnode_by_id(node_id).unwrap();
            let event_id = after.pending_changes.pledge_change.unwrap();
            assert_eq!(event_id, 1);

            test.execute_all_pending_events();

            let after = test.nymnode_by_id(node_id).unwrap();
            assert_eq!(
                after.bond_information.original_pledge.amount.u128(),
                100_100_000u128
            );

            Ok(())
        }
    }

    #[cfg(test)]
    mod decreasing_pledge {
        use super::*;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::Addr;

        #[test]
        fn when_there_are_no_nodes() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let sender = mock_info("owner", &[]);
            let env = test.env();
            let decrease_by = test.coin(1000);
            let err = try_decrease_pledge(test.deps_mut(), env, sender, decrease_by).unwrap_err();

            assert_eq!(
                MixnetContractError::NoAssociatedNodeBond {
                    owner: Addr::unchecked("owner"),
                },
                err
            );

            Ok(())
        }

        #[test]
        fn for_legacy_mixnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let node_id = test.add_legacy_mixnode("owner", Some(120_000_000u128.into()));
            let sender = mock_info("owner", &[]);
            let env = test.env();
            let decrease_by = test.coin(1000);
            try_decrease_pledge(test.deps_mut(), env, sender, decrease_by)?;

            let after = test.mixnode_by_id(node_id).unwrap();
            let event_id = after.pending_changes.pledge_change.unwrap();
            assert_eq!(event_id, 1);

            test.execute_all_pending_events();

            let after = test.mixnode_by_id(node_id).unwrap();
            assert_eq!(
                after.bond_information.original_pledge.amount.u128(),
                119_999_000u128
            );

            Ok(())
        }

        #[test]
        fn for_legacy_gateway() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            test.add_legacy_gateway("owner", None);
            let sender = mock_info("owner", &[]);
            let env = test.env();
            let decrease_by = test.coin(1000);
            let err = try_decrease_pledge(test.deps_mut(), env, sender, decrease_by).unwrap_err();

            // it's illegal to decrease pledge for legacy gateways
            assert_eq!(
                MixnetContractError::NoAssociatedNodeBond {
                    owner: Addr::unchecked("owner"),
                },
                err
            );

            Ok(())
        }

        #[test]
        fn for_nymnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let node_id = test.add_dummy_nymnode("owner", Some(120_000_000u128.into()));
            let sender = mock_info("owner", &[]);
            let env = test.env();
            let decrease_by = test.coin(1000);

            try_decrease_pledge(test.deps_mut(), env, sender, decrease_by)?;

            let after = test.nymnode_by_id(node_id).unwrap();
            let event_id = after.pending_changes.pledge_change.unwrap();
            assert_eq!(event_id, 1);

            test.execute_all_pending_events();

            let after = test.nymnode_by_id(node_id).unwrap();
            assert_eq!(
                after.bond_information.original_pledge.amount.u128(),
                119_999_000u128
            );

            Ok(())
        }
    }

    #[cfg(test)]
    mod updating_cost_params {
        use super::*;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{Addr, Uint128};
        use mixnet_contract_common::{OperatingCostRange, ProfitMarginRange};
        use nym_contracts_common::Percent;

        fn new_dummy_params() -> NodeCostParams {
            NodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(69).unwrap(),
                interval_operating_cost: Coin {
                    denom: TEST_COIN_DENOM.to_string(),
                    amount: 123456789u128.into(),
                },
            }
        }

        #[test]
        fn profit_margin_must_be_within_range() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let minimum = Percent::from_percentage_value(10)?;
            let maximum = Percent::from_percentage_value(80)?;
            let range = ProfitMarginRange::new(minimum, maximum);
            test.update_profit_margin_range(range);

            // below lower
            test.add_dummy_nymnode("owner1", None);
            let sender = mock_info("owner1", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.profit_margin_percent = Percent::from_percentage_value(9)?;
            let err =
                try_update_cost_params(test.deps_mut(), env, sender, update.clone()).unwrap_err();
            assert!(matches!(
                err,
                MixnetContractError::ProfitMarginOutsideRange { .. }
            ));

            // zero
            test.add_dummy_nymnode("owner2", None);
            let sender = mock_info("owner2", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.profit_margin_percent = Percent::zero();
            let err =
                try_update_cost_params(test.deps_mut(), env, sender, update.clone()).unwrap_err();
            assert!(matches!(
                err,
                MixnetContractError::ProfitMarginOutsideRange { .. }
            ));

            // exactly at lower
            test.add_dummy_nymnode("owner3", None);
            let sender = mock_info("owner3", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.profit_margin_percent = minimum;
            let res = try_update_cost_params(test.deps_mut(), env, sender, update.clone());
            assert!(res.is_ok());

            // above upper
            test.add_dummy_nymnode("owner4", None);
            let sender = mock_info("owner4", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.profit_margin_percent = Percent::from_percentage_value(81)?;
            let err =
                try_update_cost_params(test.deps_mut(), env, sender, update.clone()).unwrap_err();
            assert!(matches!(
                err,
                MixnetContractError::ProfitMarginOutsideRange { .. }
            ));

            // a hundred
            test.add_dummy_nymnode("owner5", None);
            let sender = mock_info("owner5", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.profit_margin_percent = Percent::hundred();
            let err =
                try_update_cost_params(test.deps_mut(), env, sender, update.clone()).unwrap_err();
            assert!(matches!(
                err,
                MixnetContractError::ProfitMarginOutsideRange { .. }
            ));

            // exactly at upper
            test.add_dummy_nymnode("owner6", None);
            let sender = mock_info("owner6", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.profit_margin_percent = maximum;
            let res = try_update_cost_params(test.deps_mut(), env, sender, update.clone());
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn operating_cost_must_be_within_range() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let minimum = Uint128::new(1000);
            let maximum = Uint128::new(100_000_000);
            let range = OperatingCostRange::new(minimum, maximum);
            test.update_operating_cost_range(range);

            // below lower
            test.add_dummy_nymnode("owner1", None);
            let sender = mock_info("owner1", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.interval_operating_cost = test.coin(999);
            let err =
                try_update_cost_params(test.deps_mut(), env, sender, update.clone()).unwrap_err();
            assert!(matches!(
                err,
                MixnetContractError::OperatingCostOutsideRange { .. }
            ));

            // zero
            test.add_dummy_nymnode("owner2", None);
            let sender = mock_info("owner2", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.interval_operating_cost = test.coin(0);
            let err =
                try_update_cost_params(test.deps_mut(), env, sender, update.clone()).unwrap_err();
            assert!(matches!(
                err,
                MixnetContractError::OperatingCostOutsideRange { .. }
            ));

            // exactly at lower
            test.add_dummy_nymnode("owner3", None);
            let sender = mock_info("owner3", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.interval_operating_cost = test.coin(minimum.u128());
            let res = try_update_cost_params(test.deps_mut(), env, sender, update.clone());
            assert!(res.is_ok());

            // above upper
            test.add_dummy_nymnode("owner4", None);
            let sender = mock_info("owner4", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.interval_operating_cost = test.coin(100_000_001);
            let err =
                try_update_cost_params(test.deps_mut(), env, sender, update.clone()).unwrap_err();
            assert!(matches!(
                err,
                MixnetContractError::OperatingCostOutsideRange { .. }
            ));

            // max
            test.add_dummy_nymnode("owner5", None);
            let sender = mock_info("owner5", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.interval_operating_cost = test.coin(u128::MAX);
            let err =
                try_update_cost_params(test.deps_mut(), env, sender, update.clone()).unwrap_err();
            assert!(matches!(
                err,
                MixnetContractError::OperatingCostOutsideRange { .. }
            ));

            // exactly at upper
            test.add_dummy_nymnode("owner6", None);
            let sender = mock_info("owner6", &[]);
            let env = test.env();
            let mut update = new_dummy_params();
            update.interval_operating_cost = test.coin(100_000_000);
            let res = try_update_cost_params(test.deps_mut(), env, sender, update.clone());
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn when_there_are_no_nodes() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let sender = mock_info("owner", &[]);
            let env = test.env();
            let err = try_update_cost_params(test.deps_mut(), env, sender, new_dummy_params())
                .unwrap_err();

            assert_eq!(
                MixnetContractError::NoAssociatedNodeBond {
                    owner: Addr::unchecked("owner"),
                },
                err
            );

            Ok(())
        }

        #[test]
        fn for_legacy_mixnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let node_id = test.add_legacy_mixnode("owner", None);
            let sender = mock_info("owner", &[]);
            let env = test.env();

            let update = new_dummy_params();
            try_update_cost_params(test.deps_mut(), env, sender, update.clone())?;

            let after = test.mixnode_by_id(node_id).unwrap();
            let event_id = after.pending_changes.cost_params_change.unwrap();
            assert_eq!(event_id, 1);

            test.execute_all_pending_events();

            let after = test.mixnode_by_id(node_id).unwrap();
            assert_eq!(update, after.rewarding_details.cost_params);

            Ok(())
        }

        #[test]
        fn for_legacy_gateway() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            test.add_legacy_gateway("owner", None);

            let sender = mock_info("owner", &[]);
            let env = test.env();
            let err = try_update_cost_params(test.deps_mut(), env, sender, new_dummy_params())
                .unwrap_err();

            // it's illegal to update cost parameters for legacy gateways
            assert_eq!(
                MixnetContractError::NoAssociatedNodeBond {
                    owner: Addr::unchecked("owner"),
                },
                err
            );
            Ok(())
        }

        #[test]
        fn for_nymnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let node_id = test.add_dummy_nymnode("owner", None);
            let sender = mock_info("owner", &[]);
            let env = test.env();

            let update = new_dummy_params();
            try_update_cost_params(test.deps_mut(), env, sender, update.clone())?;

            let after = test.nymnode_by_id(node_id).unwrap();
            let event_id = after.pending_changes.cost_params_change.unwrap();
            assert_eq!(event_id, 1);

            test.execute_all_pending_events();

            let after = test.nymnode_by_id(node_id).unwrap();
            assert_eq!(update, after.rewarding_details.cost_params);

            Ok(())
        }
    }

    #[cfg(test)]
    mod withdrawing_operator_reward {
        use super::*;
        use crate::support::tests::test_helpers::{ExtractBankMsg, TestSetup};
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::Addr;

        #[test]
        fn when_there_are_no_nodes() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let sender = mock_info("owner", &[]);
            let err = try_withdraw_operator_reward(test.deps_mut(), sender).unwrap_err();

            assert_eq!(
                MixnetContractError::NoAssociatedNodeBond {
                    owner: Addr::unchecked("owner"),
                },
                err
            );

            Ok(())
        }

        #[test]
        fn for_legacy_mixnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let active_params = test.active_node_params(100.0);

            // no rewards
            test.add_legacy_mixnode("owner1", None);
            let sender = mock_info("owner1", &[]);

            let res = try_withdraw_operator_reward(test.deps_mut(), sender)?;
            let maybe_bank = res.unwrap_bank_msg();
            assert!(maybe_bank.is_none());

            let node_id = test.add_legacy_mixnode("owner2", None);
            let sender = mock_info("owner2", &[]);
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id]);
            test.start_epoch_transition();
            test.reward_with_distribution(node_id, active_params);

            let res = try_withdraw_operator_reward(test.deps_mut(), sender)?;
            let maybe_bank = res.unwrap_bank_msg();
            assert!(maybe_bank.is_some());

            Ok(())
        }

        #[test]
        fn for_legacy_gateway() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            test.add_legacy_gateway("owner", None);

            let sender = mock_info("owner", &[]);
            let err = try_withdraw_operator_reward(test.deps_mut(), sender).unwrap_err();

            // no rewards for legacy gateways...
            assert_eq!(
                MixnetContractError::NoAssociatedNodeBond {
                    owner: Addr::unchecked("owner"),
                },
                err
            );
            Ok(())
        }

        #[test]
        fn for_nymnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let active_params = test.active_node_params(100.0);

            // no rewards
            test.add_dummy_nymnode("owner1", None);
            let sender = mock_info("owner1", &[]);

            let res = try_withdraw_operator_reward(test.deps_mut(), sender)?;
            let maybe_bank = res.unwrap_bank_msg();
            assert!(maybe_bank.is_none());

            let node_id = test.add_dummy_nymnode("owner2", None);
            let sender = mock_info("owner2", &[]);
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id]);
            test.start_epoch_transition();
            test.reward_with_distribution(node_id, active_params);

            let res = try_withdraw_operator_reward(test.deps_mut(), sender)?;
            let maybe_bank = res.unwrap_bank_msg();
            assert!(maybe_bank.is_some());

            Ok(())
        }
    }
}
