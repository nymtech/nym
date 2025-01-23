// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, DepsMut};
use cw_controllers::Admin;
use cw_storage_plus::Item;
use nym_pool_contract_common::constants::storage_keys;
use nym_pool_contract_common::NymPoolContractError;

pub const NYM_POOL_STORAGE: NymPoolStorage = NymPoolStorage::new();

pub struct NymPoolStorage {
    pub(crate) contract_admin: Admin,
    pub(crate) pool_denomination: Item<String>,
    // pub(crate)
}

impl NymPoolStorage {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        NymPoolStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            pool_denomination: Item::new(storage_keys::POOL_DENOMINATION),
        }
    }

    pub fn initialise(
        &self,
        deps: DepsMut,
        admin: Addr,
        pool_denom: &String,
    ) -> Result<(), NymPoolContractError> {
        self.pool_denomination.save(deps.storage, pool_denom)?;
        self.contract_admin.set(deps, Some(admin))?;
        Ok(())
    }
}
