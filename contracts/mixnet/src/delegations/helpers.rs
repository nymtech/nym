// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::storage;
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Coin, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::NodeRewarding;
use mixnet_contract_common::Delegation;

pub(crate) fn undelegate(
    store: &mut dyn Storage,
    delegation: Delegation,
    mut mix_rewarding: NodeRewarding,
) -> Result<Coin, MixnetContractError> {
    let tokens = mix_rewarding.undelegate(&delegation)?;

    rewards_storage::MIXNODE_REWARDING.save(store, delegation.node_id, &mix_rewarding)?;
    storage::delegations().replace(store, delegation.storage_key(), None, Some(&delegation))?;

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::{Addr, Decimal, Uint128};
    use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

    #[test]
    fn undelegation_updates_mix_rewarding_storage_and_deletes_delegation() {
        let mut test = TestSetup::new();
        let active_params = test.active_node_params(100.0);

        let mix_id =
            test.add_rewarded_set_nymnode_id("mix-owner", Some(Uint128::new(100_000_000_000)));
        let delegator = "delegator";
        let og_amount = Uint128::new(200_000_000);
        test.add_immediate_delegation(delegator, og_amount, mix_id);

        test.skip_to_next_epoch_end();
        test.force_change_mix_rewarded_set(vec![mix_id]);
        let dist1 = test.reward_with_distribution_ignore_state(mix_id, active_params);
        test.skip_to_next_epoch_end();
        let dist2 = test.reward_with_distribution_ignore_state(mix_id, active_params);

        let mix_rewarding = test.mix_rewarding(mix_id);
        let delegation = test.delegation(mix_id, delegator, &None);

        let expected_amount = og_amount + truncate_reward_amount(dist1.delegates + dist2.delegates);

        let res = undelegate(test.deps_mut().storage, delegation, mix_rewarding).unwrap();
        assert_eq!(res.amount, expected_amount);

        let mix_rewarding = test.mix_rewarding(mix_id);
        assert_eq!(mix_rewarding.delegates, Decimal::zero());
        assert_eq!(mix_rewarding.unique_delegations, 0);

        let storage_key =
            Delegation::generate_storage_key(mix_id, &Addr::unchecked(delegator), None);
        assert!(storage::delegations()
            .may_load(test.deps().storage, storage_key)
            .unwrap()
            .is_none());
    }
}
