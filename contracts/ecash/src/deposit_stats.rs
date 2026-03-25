// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::{coin, Addr, Coin, Order, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use nym_ecash_contract_common::EcashContractError;
use std::collections::HashMap;

pub(crate) struct DepositStatsStorage {
    /// Total deposits performed with the default price
    pub(crate) deposits_with_default_price: Item<u32>,

    /// Total amounts deposited with the default price
    pub(crate) deposits_with_default_price_amounts: Item<Coin>,

    /// Total deposits performed with a custom price by account
    pub(crate) deposits_with_custom_price: Map<Addr, u32>,

    /// Total amounts deposited with a custom price by account
    pub(crate) deposits_with_custom_price_amounts: Map<Addr, Coin>,
}

impl DepositStatsStorage {
    pub(crate) const fn new() -> Self {
        Self {
            deposits_with_default_price: Item::new("deposits_with_default_price"),
            deposits_with_default_price_amounts: Item::new("deposits_with_default_price_amounts"),
            deposits_with_custom_price: Map::new("deposits_with_custom_price"),
            deposits_with_custom_price_amounts: Map::new("deposits_with_custom_price_amounts"),
        }
    }

    pub(crate) fn new_default_deposit(
        &self,
        store: &mut dyn Storage,
        deposited: &Coin,
    ) -> Result<(), EcashContractError> {
        self.deposits_with_default_price
            .update(store, |count| StdResult::Ok(count + 1))?;
        self.deposits_with_default_price_amounts
            .update(store, |amount| {
                let mut updated = amount;
                updated.amount += deposited.amount;
                StdResult::Ok(updated)
            })?;

        Ok(())
    }

    pub(crate) fn new_reduced_deposit(
        &self,
        store: &mut dyn Storage,
        sender: &Addr,
        deposited: &Coin,
    ) -> Result<(), EcashContractError> {
        self.deposits_with_custom_price
            .update(store, sender.clone(), |count| {
                StdResult::Ok(count.unwrap_or_default() + 1)
            })?;

        self.deposits_with_custom_price_amounts
            .update(store, sender.clone(), |amount| {
                let updated = match amount {
                    None => deposited.clone(),
                    Some(mut existing) => {
                        existing.amount += deposited.amount;
                        existing
                    }
                };
                StdResult::Ok(updated)
            })?;

        Ok(())
    }

    pub(crate) fn get_total_deposits_made_with_default_price(
        &self,
        store: &dyn Storage,
    ) -> StdResult<u32> {
        Ok(self
            .deposits_with_default_price
            .may_load(store)?
            .unwrap_or(0))
    }

    pub(crate) fn get_total_deposited_with_default_price(
        &self,
        store: &dyn Storage,
        denom: &str,
    ) -> StdResult<Coin> {
        Ok(self
            .deposits_with_default_price_amounts
            .may_load(store)?
            .unwrap_or_else(|| coin(0, denom)))
    }

    pub(crate) fn get_custom_price_deposits(
        &self,
        store: &dyn Storage,
        denom: &str,
    ) -> StdResult<CustomPriceDepositStats> {
        let mut total_count = 0;
        let mut total_amount = coin(0, denom);
        let mut per_account_count = HashMap::new();
        let mut per_account_amount = HashMap::new();

        for item in self
            .deposits_with_custom_price
            .range(store, None, None, Order::Ascending)
        {
            let (addr, count) = item?;
            total_count += count;
            per_account_count.insert(addr.into_string(), count);
        }

        for item in
            self.deposits_with_custom_price_amounts
                .range(store, None, None, Order::Ascending)
        {
            let (addr, amount) = item?;
            total_amount.amount += amount.amount;
            per_account_amount.insert(addr.into_string(), amount);
        }

        Ok(CustomPriceDepositStats {
            total_count,
            total_amount,
            per_account_count,
            per_account_amount,
        })
    }
}

impl DepositStatsStorage {
    /// Asserts that the per-tier deposit counts sum to the given total.
    /// Only meaningful when all deposits go through the contract entry point
    /// (not after raw storage writes that bypass bookkeeping).
    #[cfg(test)]
    pub(crate) fn assert_counts_consistent(&self, store: &dyn Storage, total_deposits_made: u32) {
        let default_count = self
            .get_total_deposits_made_with_default_price(store)
            .unwrap();
        let custom = self.get_custom_price_deposits(store, "unused").unwrap();
        assert_eq!(
            default_count + custom.total_count,
            total_deposits_made,
            "deposit stats invariant violated: default ({default_count}) + custom ({}) != total ({total_deposits_made})",
            custom.total_count,
        );
    }
}

pub(crate) struct CustomPriceDepositStats {
    pub(crate) total_count: u32,
    pub(crate) total_amount: Coin,
    pub(crate) per_account_count: HashMap<String, u32>,
    pub(crate) per_account_amount: HashMap<String, Coin>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::mock_dependencies;

    const DENOM: &str = "unym";
    const DEFAULT_AMOUNT: u128 = 75_000_000;
    const REDUCED_AMOUNT: u128 = 10_000_000;

    /// Mirror what `instantiate` does: zero-initialise the default-price counters.
    /// The custom-price Maps need no initialisation (they start empty).
    fn init_stats(storage: &mut dyn Storage) -> DepositStatsStorage {
        let stats = DepositStatsStorage::new();
        stats.deposits_with_default_price.save(storage, &0).unwrap();
        stats
            .deposits_with_default_price_amounts
            .save(storage, &coin(0, DENOM))
            .unwrap();
        stats
    }

    #[test]
    fn single_default_deposit_increments_count_and_amount() {
        let mut deps = mock_dependencies();
        let stats = init_stats(deps.as_mut().storage);

        stats
            .new_default_deposit(deps.as_mut().storage, &coin(DEFAULT_AMOUNT, DENOM))
            .unwrap();

        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            1
        );
        assert_eq!(
            stats
                .deposits_with_default_price_amounts
                .load(deps.as_ref().storage)
                .unwrap(),
            coin(DEFAULT_AMOUNT, DENOM)
        );
    }

    #[test]
    fn multiple_default_deposits_accumulate() {
        let mut deps = mock_dependencies();
        let stats = init_stats(deps.as_mut().storage);

        for _ in 0..3 {
            stats
                .new_default_deposit(deps.as_mut().storage, &coin(DEFAULT_AMOUNT, DENOM))
                .unwrap();
        }

        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            3
        );
        assert_eq!(
            stats
                .deposits_with_default_price_amounts
                .load(deps.as_ref().storage)
                .unwrap(),
            coin(DEFAULT_AMOUNT * 3, DENOM)
        );
    }

    #[test]
    fn single_reduced_deposit_is_tracked_per_address() {
        let mut deps = mock_dependencies();
        let stats = init_stats(deps.as_mut().storage);
        let alice = deps.api.addr_make("alice");

        stats
            .new_reduced_deposit(deps.as_mut().storage, &alice, &coin(REDUCED_AMOUNT, DENOM))
            .unwrap();

        assert_eq!(
            stats
                .deposits_with_custom_price
                .load(deps.as_ref().storage, alice.clone())
                .unwrap(),
            1
        );
        assert_eq!(
            stats
                .deposits_with_custom_price_amounts
                .load(deps.as_ref().storage, alice.clone())
                .unwrap(),
            coin(REDUCED_AMOUNT, DENOM)
        );
        // default-price stats must be untouched
        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            0
        );
    }

    #[test]
    fn multiple_reduced_deposits_same_address_accumulate() {
        let mut deps = mock_dependencies();
        let stats = init_stats(deps.as_mut().storage);
        let alice = deps.api.addr_make("alice");

        for _ in 0..4 {
            stats
                .new_reduced_deposit(deps.as_mut().storage, &alice, &coin(REDUCED_AMOUNT, DENOM))
                .unwrap();
        }

        assert_eq!(
            stats
                .deposits_with_custom_price
                .load(deps.as_ref().storage, alice.clone())
                .unwrap(),
            4
        );
        assert_eq!(
            stats
                .deposits_with_custom_price_amounts
                .load(deps.as_ref().storage, alice.clone())
                .unwrap(),
            coin(REDUCED_AMOUNT * 4, DENOM)
        );
    }

    #[test]
    fn reduced_deposits_for_different_addresses_tracked_independently() {
        let mut deps = mock_dependencies();
        let stats = init_stats(deps.as_mut().storage);
        let alice = deps.api.addr_make("alice");
        let bob = deps.api.addr_make("bob");

        stats
            .new_reduced_deposit(deps.as_mut().storage, &alice, &coin(REDUCED_AMOUNT, DENOM))
            .unwrap();
        stats
            .new_reduced_deposit(deps.as_mut().storage, &alice, &coin(REDUCED_AMOUNT, DENOM))
            .unwrap();
        stats
            .new_reduced_deposit(deps.as_mut().storage, &bob, &coin(5_000_000, DENOM))
            .unwrap();

        assert_eq!(
            stats
                .deposits_with_custom_price
                .load(deps.as_ref().storage, alice.clone())
                .unwrap(),
            2
        );
        assert_eq!(
            stats
                .deposits_with_custom_price
                .load(deps.as_ref().storage, bob.clone())
                .unwrap(),
            1
        );
        assert_eq!(
            stats
                .deposits_with_custom_price_amounts
                .load(deps.as_ref().storage, alice)
                .unwrap(),
            coin(REDUCED_AMOUNT * 2, DENOM)
        );
        assert_eq!(
            stats
                .deposits_with_custom_price_amounts
                .load(deps.as_ref().storage, bob)
                .unwrap(),
            coin(5_000_000, DENOM)
        );
    }

    #[test]
    fn default_and_reduced_stats_do_not_interfere() {
        let mut deps = mock_dependencies();
        let stats = init_stats(deps.as_mut().storage);
        let alice = deps.api.addr_make("alice");

        stats
            .new_default_deposit(deps.as_mut().storage, &coin(DEFAULT_AMOUNT, DENOM))
            .unwrap();
        stats
            .new_reduced_deposit(deps.as_mut().storage, &alice, &coin(REDUCED_AMOUNT, DENOM))
            .unwrap();
        stats
            .new_default_deposit(deps.as_mut().storage, &coin(DEFAULT_AMOUNT, DENOM))
            .unwrap();

        assert_eq!(
            stats
                .deposits_with_default_price
                .load(deps.as_ref().storage)
                .unwrap(),
            2
        );
        assert_eq!(
            stats
                .deposits_with_custom_price
                .load(deps.as_ref().storage, alice)
                .unwrap(),
            1
        );
    }
}
