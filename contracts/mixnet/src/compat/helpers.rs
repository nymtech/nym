// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnode_storage;
use crate::nodes::storage as nymnodes_storage;
use crate::support::helpers::ensure_epoch_in_progress_state;
use cosmwasm_std::{Coin, StdResult, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::helpers::{NodeBond, NodeDetails, PendingChanges};
use mixnet_contract_common::NodeId;

pub fn ensure_can_withdraw_rewards<D>(node_details: &D) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    // we can only withdraw rewards for a bonded node (i.e. not in the process of unbonding)
    // otherwise we know there are no rewards to withdraw
    node_details.bond_info().ensure_bonded()?;

    Ok(())
}

pub fn ensure_can_modify_cost_params<D>(
    storage: &dyn Storage,
    node_details: &D,
) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    // changing cost params is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(storage)?;

    // we can only change cost params for a bonded node (i.e. not in the process of unbonding)
    node_details.bond_info().ensure_bonded()?;

    Ok(())
}

fn ensure_can_modify_pledge<D>(
    storage: &dyn Storage,
    node_details: &D,
) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    // changing pledge is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(storage)?;

    // we can only change pledge for a bonded node (i.e. not in the process of unbonding)
    node_details.bond_info().ensure_bonded()?;

    // the node can't have any pending pledge changes
    node_details
        .pending_changes()
        .ensure_no_pending_pledge_changes()?;

    Ok(())
}

// remove duplicate code and make sure the same checks are performed everywhere
// (so nothing is accidentally missing)
pub fn ensure_can_increase_pledge<D>(
    storage: &dyn Storage,
    node_details: &D,
) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    ensure_can_modify_pledge(storage, node_details)
}

// remove duplicate code and make sure the same checks are performed everywhere
// (so nothing is accidentally missing)
pub fn ensure_can_decrease_pledge<D>(
    storage: &dyn Storage,
    node_details: &D,
    decrease_by: &Coin,
) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    ensure_can_modify_pledge(storage, node_details)?;

    let minimum_pledge = mixnet_params_storage::minimum_node_pledge(storage)?;

    // check that the denomination is correct
    if decrease_by.denom != minimum_pledge.denom {
        return Err(MixnetContractError::WrongDenom {
            received: decrease_by.denom.clone(),
            expected: minimum_pledge.denom,
        });
    }

    // also check if the request contains non-zero amount
    // (otherwise it's a no-op and we should we waste gas when resolving events?)
    if decrease_by.amount.is_zero() {
        return Err(MixnetContractError::ZeroCoinAmount);
    }

    // decreasing pledge can't result in the new pledge being lower than the minimum amount
    let new_pledge_amount = node_details
        .bond_info()
        .original_pledge()
        .amount
        .saturating_sub(decrease_by.amount);
    if new_pledge_amount < minimum_pledge.amount {
        return Err(MixnetContractError::InvalidPledgeReduction {
            current: node_details.bond_info().original_pledge().amount,
            decrease_by: decrease_by.amount,
            minimum: minimum_pledge.amount,
            denom: minimum_pledge.denom,
        });
    }

    Ok(())
}

pub fn get_bond(
    storage: &dyn Storage,
    node_id: NodeId,
) -> Result<Box<dyn NodeBond>, MixnetContractError> {
    if let Ok(mix_bond) = mixnode_storage::mixnode_bonds().load(storage, node_id) {
        Ok(Box::new(mix_bond))
    } else {
        let node_bond = nymnodes_storage::nym_nodes()
            .load(storage, node_id)
            .map_err(|_| MixnetContractError::NymNodeBondNotFound { node_id })?;
        Ok(Box::new(node_bond))
    }
}

pub fn may_get_bond(
    storage: &dyn Storage,
    node_id: NodeId,
) -> StdResult<Option<Box<dyn NodeBond>>> {
    if let Some(mix_bond) = mixnode_storage::mixnode_bonds().may_load(storage, node_id)? {
        Ok(Some(Box::new(mix_bond)))
    } else if let Some(node_bond) = nymnodes_storage::nym_nodes().may_load(storage, node_id)? {
        Ok(Some(Box::new(node_bond)))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod reward_withdrawing_permission {
        use super::*;
        use crate::support::tests::test_helpers::TestSetup;

        #[test]
        fn for_legacy_mixnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let node_id = test.add_legacy_mixnode("owner", None);
            let details = test.mixnode_by_id(node_id).unwrap();

            // node must not be in the process of unbonding
            assert!(ensure_can_withdraw_rewards(&details).is_ok());

            test.start_unbonding_mixnode(node_id);
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_withdraw_rewards(&details).unwrap_err();
            assert_eq!(res, MixnetContractError::NodeIsUnbonding { node_id });
            Ok(())
        }

        #[test]
        fn for_nymnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let node_id = test.add_dummy_nymnode("owner", None);
            let details = test.nymnode_by_id(node_id).unwrap();

            // node must not be in the process of unbonding
            assert!(ensure_can_withdraw_rewards(&details).is_ok());

            test.start_unbonding_nymnode(node_id);
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_withdraw_rewards(&details).unwrap_err();
            assert_eq!(res, MixnetContractError::NodeIsUnbonding { node_id });
            Ok(())
        }
    }

    #[cfg(test)]
    mod modifying_cost_params_permission {
        use super::*;
        use crate::support::tests::test_helpers::TestSetup;

        #[test]
        fn for_legacy_mixnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let node_id = test.add_legacy_mixnode("owner", None);

            let details = test.mixnode_by_id(node_id).unwrap();
            assert!(ensure_can_modify_cost_params(test.deps().storage, &details).is_ok());

            // epoch must not be mid-transition
            test.skip_to_current_epoch_end();
            test.start_epoch_transition();
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_modify_cost_params(test.deps().storage, &details).unwrap_err();
            assert!(matches!(
                res,
                MixnetContractError::EpochAdvancementInProgress { .. }
            ));
            test.set_epoch_in_progress_state();

            // node must not be in the process of unbonding
            test.start_unbonding_mixnode(node_id);
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_modify_cost_params(test.deps().storage, &details).unwrap_err();
            assert_eq!(res, MixnetContractError::NodeIsUnbonding { node_id });

            Ok(())
        }

        #[test]
        fn for_nymnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let node_id = test.add_dummy_nymnode("owner", None);

            let details = test.nymnode_by_id(node_id).unwrap();
            assert!(ensure_can_modify_cost_params(test.deps().storage, &details).is_ok());

            // epoch must not be mid-transition
            test.skip_to_current_epoch_end();
            test.start_epoch_transition();
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_modify_cost_params(test.deps().storage, &details).unwrap_err();
            assert!(matches!(
                res,
                MixnetContractError::EpochAdvancementInProgress { .. }
            ));
            test.set_epoch_in_progress_state();

            // node must not be in the process of unbonding
            test.start_unbonding_nymnode(node_id);
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_modify_cost_params(test.deps().storage, &details).unwrap_err();
            assert_eq!(res, MixnetContractError::NodeIsUnbonding { node_id });

            Ok(())
        }
    }

    #[cfg(test)]
    mod increasing_pledge_permission {
        use super::*;
        use crate::compat::transactions::{try_decrease_pledge, try_increase_pledge};
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::testing::mock_info;

        #[test]
        fn for_legacy_mixnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let node_id = test.add_legacy_mixnode("owner", None);

            let details = test.mixnode_by_id(node_id).unwrap();
            assert!(ensure_can_increase_pledge(test.deps().storage, &details).is_ok());

            // epoch must not be mid-transition
            test.skip_to_current_epoch_end();
            test.start_epoch_transition();
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_increase_pledge(test.deps().storage, &details).unwrap_err();
            assert!(matches!(
                res,
                MixnetContractError::EpochAdvancementInProgress { .. }
            ));
            test.set_epoch_in_progress_state();

            // node must not be in the process of unbonding
            test.start_unbonding_mixnode(node_id);
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_increase_pledge(test.deps().storage, &details).unwrap_err();
            assert_eq!(res, MixnetContractError::NodeIsUnbonding { node_id });

            // node can't have any pending pledge changes:
            // - increase
            let node_id = test.add_legacy_mixnode("owner2", Some(100_000_000_000u128.into()));
            let pledge_change = test.coin(100000);
            test.execute_fn(try_increase_pledge, mock_info("owner2", &[pledge_change]))?;
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_increase_pledge(test.deps().storage, &details).unwrap_err();
            assert_eq!(
                res,
                MixnetContractError::PendingPledgeChange {
                    pending_event_id: details.pending_changes.pledge_change.unwrap()
                }
            );

            // - decrease
            let node_id = test.add_legacy_mixnode("owner3", Some(100_000_000_000u128.into()));
            let pledge_change = test.coin(100000);
            let env = test.env();
            try_decrease_pledge(
                test.deps_mut(),
                env,
                mock_info("owner3", &[]),
                pledge_change,
            )?;
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_increase_pledge(test.deps().storage, &details).unwrap_err();
            assert_eq!(
                res,
                MixnetContractError::PendingPledgeChange {
                    pending_event_id: details.pending_changes.pledge_change.unwrap()
                }
            );
            Ok(())
        }

        #[test]
        fn for_nymnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let node_id = test.add_dummy_nymnode("owner", None);

            let details = test.nymnode_by_id(node_id).unwrap();
            assert!(ensure_can_increase_pledge(test.deps().storage, &details).is_ok());

            // epoch must not be mid-transition
            test.skip_to_current_epoch_end();
            test.start_epoch_transition();
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_increase_pledge(test.deps().storage, &details).unwrap_err();
            assert!(matches!(
                res,
                MixnetContractError::EpochAdvancementInProgress { .. }
            ));
            test.set_epoch_in_progress_state();

            // node must not be in the process of unbonding
            test.start_unbonding_nymnode(node_id);
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_increase_pledge(test.deps().storage, &details).unwrap_err();
            assert_eq!(res, MixnetContractError::NodeIsUnbonding { node_id });

            // node can't have any pending pledge changes:
            // - increase
            let node_id = test.add_dummy_nymnode("owner2", Some(100_000_000_000u128.into()));
            let pledge_change = test.coin(100000);
            test.execute_fn(try_increase_pledge, mock_info("owner2", &[pledge_change]))?;
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_increase_pledge(test.deps().storage, &details).unwrap_err();
            assert_eq!(
                res,
                MixnetContractError::PendingPledgeChange {
                    pending_event_id: details.pending_changes.pledge_change.unwrap()
                }
            );

            // - decrease
            let node_id = test.add_dummy_nymnode("owner3", Some(100_000_000_000u128.into()));
            let pledge_change = test.coin(100000);
            let env = test.env();
            try_decrease_pledge(
                test.deps_mut(),
                env,
                mock_info("owner3", &[]),
                pledge_change,
            )?;
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_increase_pledge(test.deps().storage, &details).unwrap_err();
            assert_eq!(
                res,
                MixnetContractError::PendingPledgeChange {
                    pending_event_id: details.pending_changes.pledge_change.unwrap()
                }
            );
            Ok(())
        }
    }

    #[cfg(test)]
    mod decreasing_pledge_permission {
        use super::*;
        use crate::compat::transactions::{try_decrease_pledge, try_increase_pledge};
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::coin;
        use cosmwasm_std::testing::mock_info;

        #[test]
        fn for_legacy_mixnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let node_id = test.add_legacy_mixnode("owner", Some(100_000_000_000u128.into()));
            let valid_decrease = test.coin(100);

            let details = test.mixnode_by_id(node_id).unwrap();
            assert!(
                ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease).is_ok()
            );

            // epoch must not be mid-transition
            test.skip_to_current_epoch_end();
            test.start_epoch_transition();
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease)
                .unwrap_err();
            assert!(matches!(
                res,
                MixnetContractError::EpochAdvancementInProgress { .. }
            ));
            test.set_epoch_in_progress_state();

            // node must not be in the process of unbonding
            test.start_unbonding_mixnode(node_id);
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease)
                .unwrap_err();
            assert_eq!(res, MixnetContractError::NodeIsUnbonding { node_id });

            // node can't have any pending pledge changes:
            // - increase
            let node_id = test.add_legacy_mixnode("owner2", Some(100_000_000_000u128.into()));
            let pledge_change = test.coin(100000);
            test.execute_fn(try_increase_pledge, mock_info("owner2", &[pledge_change]))?;
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease)
                .unwrap_err();
            assert_eq!(
                res,
                MixnetContractError::PendingPledgeChange {
                    pending_event_id: details.pending_changes.pledge_change.unwrap()
                }
            );

            // - decrease
            let node_id = test.add_legacy_mixnode("owner3", Some(100_000_000_000u128.into()));
            let pledge_change = test.coin(100000);
            let env = test.env();
            try_decrease_pledge(
                test.deps_mut(),
                env,
                mock_info("owner3", &[]),
                pledge_change,
            )?;
            let details = test.mixnode_by_id(node_id).unwrap();
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease)
                .unwrap_err();
            assert_eq!(
                res,
                MixnetContractError::PendingPledgeChange {
                    pending_event_id: details.pending_changes.pledge_change.unwrap()
                }
            );

            // denom must match
            let node_id = test.add_legacy_mixnode("owner4", Some(100_000_000_000u128.into()));
            let details = test.mixnode_by_id(node_id).unwrap();
            let bad_decrease = coin(123, "weird-denom");
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &bad_decrease)
                .unwrap_err();
            assert!(matches!(res, MixnetContractError::WrongDenom { .. }));

            // value must be non-zero
            let node_id = test.add_legacy_mixnode("owner5", Some(100_000_000_000u128.into()));
            let details = test.mixnode_by_id(node_id).unwrap();
            let bad_decrease = test.coin(0);
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &bad_decrease)
                .unwrap_err();
            assert_eq!(res, MixnetContractError::ZeroCoinAmount);

            // new pledge must be bigger than minimum
            let node_id = test.add_legacy_mixnode("owner6", Some(100_000_100u128.into()));
            let details = test.mixnode_by_id(node_id).unwrap();
            let bad_decrease = test.coin(101);
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &bad_decrease)
                .unwrap_err();
            assert!(matches!(
                res,
                MixnetContractError::InvalidPledgeReduction { .. }
            ));

            Ok(())
        }

        #[test]
        fn for_nymnode() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let node_id = test.add_dummy_nymnode("owner", Some(100_000_000_000u128.into()));
            let valid_decrease = test.coin(100);

            let details = test.nymnode_by_id(node_id).unwrap();
            assert!(
                ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease).is_ok()
            );

            // epoch must not be mid-transition
            test.skip_to_current_epoch_end();
            test.start_epoch_transition();
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease)
                .unwrap_err();
            assert!(matches!(
                res,
                MixnetContractError::EpochAdvancementInProgress { .. }
            ));
            test.set_epoch_in_progress_state();

            // node must not be in the process of unbonding
            test.start_unbonding_nymnode(node_id);
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease)
                .unwrap_err();
            assert_eq!(res, MixnetContractError::NodeIsUnbonding { node_id });

            // node can't have any pending pledge changes:
            // - increase
            let node_id = test.add_dummy_nymnode("owner2", Some(100_000_000_000u128.into()));
            let pledge_change = test.coin(100000);
            test.execute_fn(try_increase_pledge, mock_info("owner2", &[pledge_change]))?;
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease)
                .unwrap_err();
            assert_eq!(
                res,
                MixnetContractError::PendingPledgeChange {
                    pending_event_id: details.pending_changes.pledge_change.unwrap()
                }
            );

            // - decrease
            let node_id = test.add_dummy_nymnode("owner3", Some(100_000_000_000u128.into()));
            let pledge_change = test.coin(100000);
            let env = test.env();
            try_decrease_pledge(
                test.deps_mut(),
                env,
                mock_info("owner3", &[]),
                pledge_change,
            )?;
            let details = test.nymnode_by_id(node_id).unwrap();
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &valid_decrease)
                .unwrap_err();
            assert_eq!(
                res,
                MixnetContractError::PendingPledgeChange {
                    pending_event_id: details.pending_changes.pledge_change.unwrap()
                }
            );

            // denom must match
            let node_id = test.add_dummy_nymnode("owner4", Some(100_000_000_000u128.into()));
            let details = test.nymnode_by_id(node_id).unwrap();
            let bad_decrease = coin(123, "weird-denom");
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &bad_decrease)
                .unwrap_err();
            assert!(matches!(res, MixnetContractError::WrongDenom { .. }));

            // value must be non-zero
            let node_id = test.add_dummy_nymnode("owner5", Some(100_000_000_000u128.into()));
            let details = test.nymnode_by_id(node_id).unwrap();
            let bad_decrease = test.coin(0);
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &bad_decrease)
                .unwrap_err();
            assert_eq!(res, MixnetContractError::ZeroCoinAmount);

            // new pledge must be bigger than minimum
            let node_id = test.add_dummy_nymnode("owner6", Some(100_000_100u128.into()));
            let details = test.nymnode_by_id(node_id).unwrap();
            let bad_decrease = test.coin(101);
            let res = ensure_can_decrease_pledge(test.deps().storage, &details, &bad_decrease)
                .unwrap_err();
            assert!(matches!(
                res,
                MixnetContractError::InvalidPledgeReduction { .. }
            ));

            Ok(())
        }
    }
}
