// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::{Addr, Coin, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use nym_ecash_contract_common::EcashContractError;

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
}
