// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Coin, Deps, DepsMut, Env, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use nym_pool_contract_common::constants::storage_keys;
use nym_pool_contract_common::{
    Allowance, Grant, GranteeAddress, GranterAddress, NymPoolContractError,
};
use std::collections::HashMap;

pub const NYM_POOL_STORAGE: NymPoolStorage = NymPoolStorage::new();

pub struct NymPoolStorage {
    pub(crate) contract_admin: Admin,
    pub(crate) pool_denomination: Item<String>,
    pub(crate) grants: Map<GranteeAddress, Grant>,
    pub(crate) locked: LockedStorage,
}

impl NymPoolStorage {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        NymPoolStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            pool_denomination: Item::new(storage_keys::POOL_DENOMINATION),
            grants: Map::new(storage_keys::GRANTS),
            locked: LockedStorage::new(),
        }
    }

    fn contract_balance(&self, deps: Deps, env: &Env) -> Result<Coin, NymPoolContractError> {
        let denom = self.pool_denomination.load(deps.storage)?;
        Ok(deps.querier.query_balance(&env.contract.address, denom)?)
    }

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        env: Env,
        admin: Addr,
        pool_denom: &String,
        initial_grants: HashMap<String, Allowance>,
    ) -> Result<(), NymPoolContractError> {
        // add all initial grants
        for (grantee, allowance) in initial_grants {
            self.add_grant(deps.branch(), &env, &admin, grantee, allowance)?;
        }

        // set the denom
        self.pool_denomination.save(deps.storage, pool_denom)?;

        // set the contract admin
        self.contract_admin.set(deps, Some(admin))?;

        Ok(())
    }

    // currently it just checks whether the provided address it the admin,
    // but this API would allow us to extend it to have a list of permitted granters
    fn is_whitelisted_granter(
        &self,
        deps: Deps,
        addr: &Addr,
    ) -> Result<bool, NymPoolContractError> {
        self.contract_admin.is_admin(deps, addr).map_err(Into::into)
    }

    fn ensure_is_whitelisted_granter(
        &self,
        deps: Deps,
        addr: &Addr,
    ) -> Result<(), NymPoolContractError> {
        if !self.is_whitelisted_granter(deps, addr)? {
            return Err(NymPoolContractError::InvalidGranter {
                addr: addr.to_string(),
            });
        }
        Ok(())
    }

    pub fn available_tokens(&self, deps: Deps, env: &Env) -> Result<Coin, NymPoolContractError> {
        let locked = self.locked.total_locked.load(deps.storage)?;
        let balance = self.contract_balance(deps, env)?;

        // the amount of available tokens is the current contract balance minus all the locked tokens
        let mut available = balance;
        available.amount = available.amount.saturating_sub(locked);
        Ok(available)
    }

    pub fn try_load_grant(
        &self,
        grantee: &GranteeAddress,
    ) -> Result<Option<Grant>, NymPoolContractError> {
        todo!()
    }

    pub fn load_grant(&self, grantee: &GranteeAddress) -> Result<Grant, NymPoolContractError> {
        self.try_load_grant(grantee)?
            .ok_or(NymPoolContractError::GrantNotFound {
                grantee: grantee.to_string(),
            })
    }

    pub fn add_grant(
        &self,
        deps: DepsMut,
        env: &Env,
        granter: &GranterAddress,
        grantee: String,
        allowance: Allowance,
    ) -> Result<(), NymPoolContractError> {
        let grantee = deps.api.addr_validate(&grantee)?;

        // the granter should be permitted to add new grants
        self.ensure_is_whitelisted_granter(deps.as_ref(), granter)?;

        // check for existing grant
        if let Some(existing_grant) = self.try_load_grant(&grantee)? {
            return Err(NymPoolContractError::GrantAlreadyExist {
                granter: existing_grant.granter.to_string(),
                grantee: grantee.to_string(),
                created_at_height: existing_grant.granted_at_height,
            });
        }

        // the allowance should be well-formed
        let expected_denom = self.pool_denomination.load(deps.storage)?;
        allowance.validate(env, &expected_denom)?;

        // if allowance includes explicit limit,
        // it should not be higher than the total remaining tokens
        // note: we already verified denomination matched when we validated the allowance
        if let Some(ref spend_limit) = allowance.basic().spend_limit {
            let available = self.available_tokens(deps.as_ref(), env)?;
            if spend_limit.amount > available.amount {
                return Err(NymPoolContractError::InsufficientTokens {
                    available,
                    requested_grant: spend_limit.clone(),
                });
            }
        }

        self.grants.save(
            deps.storage,
            grantee.clone(),
            &Grant {
                granter: grantee.clone(),
                grantee,
                granted_at_height: env.block.height,
                allowance,
            },
        )?;

        // TODO: emit events

        Ok(())
    }

    pub fn update_grant(&self) -> Result<(), NymPoolContractError> {
        todo!()
    }

    pub fn revoke_grant(&self) -> Result<(), NymPoolContractError> {
        todo!()
    }
}

pub(crate) struct LockedStorage {
    pub(crate) total_locked: Item<Uint128>,
    pub(crate) grantees: Map<GranteeAddress, Uint128>,
}

impl LockedStorage {
    #[allow(clippy::new_without_default)]
    const fn new() -> Self {
        LockedStorage {
            total_locked: Item::new(storage_keys::TOTAL_LOCKED),
            grantees: Map::new(storage_keys::LOCKED_GRANTEES),
        }
    }
}
