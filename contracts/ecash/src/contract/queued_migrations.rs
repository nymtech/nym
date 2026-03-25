// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::deposit::DepositStorage;
use crate::deposit_stats::DepositStatsStorage;
use cosmwasm_std::DepsMut;
use cw_storage_plus::Item;
use nym_ecash_contract_common::counters::PoolCounters;
use nym_ecash_contract_common::EcashContractError;

pub fn add_tiered_pricing(deps: DepsMut) -> Result<(), EcashContractError> {
    let deposits = DepositStorage::new();
    let deposits_stats = DepositStatsStorage::new();
    let pool_counters: Item<PoolCounters> = Item::new("pool_counters");

    // All the deposits made so far were performed with the default price.
    // The `reduced_deposits` Map (whitelist) is intentionally left empty — no
    // addresses have custom pricing until the admin explicitly configures them.
    let deposits_performed = deposits.total_deposits_made(deps.storage)?;
    let deposits_amounts = pool_counters.load(deps.storage)?.total_deposited;

    deposits_stats
        .deposits_with_default_price
        .save(deps.storage, &deposits_performed)?;

    deposits_stats
        .deposits_with_default_price_amounts
        .save(deps.storage, &deposits_amounts)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deposit::DepositStorage;
    use crate::deposit_stats::DepositStatsStorage;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::mock_dependencies;
    use cw_storage_plus::Item;
    use nym_ecash_contract_common::counters::PoolCounters;

    const DENOM: &str = "unym";

    fn save_pool_counters(storage: &mut dyn cosmwasm_std::Storage, total_deposited: u128) {
        let pool_counters: Item<PoolCounters> = Item::new("pool_counters");
        pool_counters
            .save(
                storage,
                &PoolCounters {
                    total_deposited: coin(total_deposited, DENOM),
                    total_redeemed: coin(0, DENOM),
                },
            )
            .unwrap();
    }

    #[test]
    fn migration_with_no_prior_deposits_initialises_stats_to_zero() {
        let mut deps = mock_dependencies();

        // No deposit_id_counter saved — contract never had a deposit.
        save_pool_counters(deps.as_mut().storage, 0);

        add_tiered_pricing(deps.as_mut()).unwrap();

        let stats = DepositStatsStorage::new();
        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            0
        );
        assert_eq!(
            stats
                .deposits_with_default_price_amounts
                .load(deps.as_ref().storage)
                .unwrap(),
            coin(0, DENOM)
        );
    }

    #[test]
    fn migration_with_prior_deposits_backfills_correct_count() {
        let mut deps = mock_dependencies();
        let n_deposits: u32 = 3;
        let total: u128 = n_deposits as u128 * 75_000_000;

        // Simulate n_deposits having been made: counter stores the next available id,
        // which equals the number of deposits already performed.
        let deposits = DepositStorage::new();
        deposits
            .deposit_id_counter
            .save(deps.as_mut().storage, &n_deposits)
            .unwrap();

        save_pool_counters(deps.as_mut().storage, total);

        add_tiered_pricing(deps.as_mut()).unwrap();

        let stats = DepositStatsStorage::new();
        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            n_deposits
        );
        assert_eq!(
            stats
                .deposits_with_default_price_amounts
                .load(deps.as_ref().storage)
                .unwrap(),
            coin(total, DENOM)
        );
    }

    #[test]
    fn migration_with_single_deposit_backfills_count_of_one() {
        let mut deps = mock_dependencies();

        // After one deposit, next_id returns 0 and saves counter=1.
        let deposits = DepositStorage::new();
        deposits
            .deposit_id_counter
            .save(deps.as_mut().storage, &1u32)
            .unwrap();

        save_pool_counters(deps.as_mut().storage, 75_000_000);

        add_tiered_pricing(deps.as_mut()).unwrap();

        let stats = DepositStatsStorage::new();
        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            1
        );
    }
}
