// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Coin, Deps, DepsMut, Env, Storage, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use nym_pool_contract_common::constants::storage_keys;
use nym_pool_contract_common::{
    Allowance, Grant, GranteeAddress, GranterAddress, GranterInformation, NymPoolContractError,
};
use std::collections::HashMap;

pub const NYM_POOL_STORAGE: NymPoolStorage = NymPoolStorage::new();

pub struct NymPoolStorage {
    pub(crate) contract_admin: Admin,
    pub(crate) pool_denomination: Item<String>,
    pub(crate) granters: Map<GranterAddress, GranterInformation>,

    // pub(crate) expired: (),

    // unlike the feegrant module, we specifically don't allow multiple grants (from different granters)
    // towards the same grantee
    pub(crate) grants: Map<GranteeAddress, Grant>,
    pub(crate) locked: LockedStorage,
}

impl NymPoolStorage {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        NymPoolStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            pool_denomination: Item::new(storage_keys::POOL_DENOMINATION),
            granters: Map::new(storage_keys::GRANTERS),
            grants: Map::new(storage_keys::GRANTS),
            locked: LockedStorage::new(),
        }
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
            let grantee = deps.api.addr_validate(&grantee)?;
            self.insert_new_grant(deps.branch(), &env, &admin, grantee, allowance)?;
        }

        // set the denom
        self.pool_denomination.save(deps.storage, pool_denom)?;

        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;

        // initialise the locked storage (with the total of 0)
        self.locked.initialise(deps.branch())?;

        // set the admin to be a whitelisted granter
        self.add_new_granter(deps, env, admin.clone(), admin)?;

        Ok(())
    }

    fn contract_balance(&self, deps: Deps, env: &Env) -> Result<Coin, NymPoolContractError> {
        let denom = self.pool_denomination.load(deps.storage)?;
        Ok(deps.querier.query_balance(&env.contract.address, denom)?)
    }

    fn is_admin(&self, deps: Deps, addr: &Addr) -> Result<bool, NymPoolContractError> {
        self.contract_admin.is_admin(deps, addr).map_err(Into::into)
    }

    fn ensure_is_admin(&self, deps: Deps, addr: &Addr) -> Result<(), NymPoolContractError> {
        self.contract_admin
            .assert_admin(deps, addr)
            .map_err(Into::into)
    }

    pub fn try_load_granter(
        &self,
        deps: Deps,
        granter: &GranterAddress,
    ) -> Result<Option<GranterInformation>, NymPoolContractError> {
        self.granters
            .may_load(deps.storage, granter.clone())
            .map_err(Into::into)
    }

    fn is_whitelisted_granter(
        &self,
        deps: Deps,
        addr: &GranterAddress,
    ) -> Result<bool, NymPoolContractError> {
        Ok(self.try_load_granter(deps, addr)?.is_some())
    }

    fn ensure_is_whitelisted_granter(
        &self,
        deps: Deps,
        addr: &GranterAddress,
    ) -> Result<(), NymPoolContractError> {
        if !self.is_whitelisted_granter(deps, addr)? {
            return Err(NymPoolContractError::InvalidGranter {
                addr: addr.to_string(),
            });
        }
        Ok(())
    }

    pub fn add_new_granter(
        &self,
        deps: DepsMut,
        env: Env,
        sender: Addr,
        granter: GranterAddress,
    ) -> Result<(), NymPoolContractError> {
        // currently only the admin is permitted to add new granters
        self.ensure_is_admin(deps.as_ref(), &sender)?;

        if self
            .granters
            .may_load(deps.storage, granter.clone())?
            .is_some()
        {
            return Err(NymPoolContractError::AlreadyAGranter);
        }

        self.granters.save(
            deps.storage,
            granter,
            &GranterInformation {
                created_by: sender,
                created_at_height: env.block.height,
            },
        )?;

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
        deps: Deps,
        grantee: &GranteeAddress,
    ) -> Result<Option<Grant>, NymPoolContractError> {
        self.grants
            .may_load(deps.storage, grantee.clone())
            .map_err(Into::into)
    }

    pub fn load_grant(
        &self,
        deps: Deps,
        grantee: &GranteeAddress,
    ) -> Result<Grant, NymPoolContractError> {
        self.try_load_grant(deps, grantee)?
            .ok_or(NymPoolContractError::GrantNotFound {
                grantee: grantee.to_string(),
            })
    }

    pub fn insert_new_grant(
        &self,
        deps: DepsMut,
        env: &Env,
        granter: &GranterAddress,
        grantee: GranteeAddress,
        mut allowance: Allowance,
    ) -> Result<(), NymPoolContractError> {
        // the granter should be permitted to add new grants
        self.ensure_is_whitelisted_granter(deps.as_ref(), granter)?;

        // check for existing grant
        if let Some(existing_grant) = self.try_load_grant(deps.as_ref(), &grantee)? {
            return Err(NymPoolContractError::GrantAlreadyExist {
                granter: existing_grant.granter.to_string(),
                grantee: grantee.to_string(),
                created_at_height: existing_grant.granted_at_height,
            });
        }

        // the allowance should be well-formed
        let expected_denom = self.pool_denomination.load(deps.storage)?;
        allowance.validate_new(env, &expected_denom)?;

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

        // set initial state based on the env
        allowance.set_initial_state(env);

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

        Ok(())
    }

    pub fn update_grant(
        &self,
        deps: DepsMut,
        grantee_address: GranteeAddress,
        grant: Grant,
    ) -> Result<(), NymPoolContractError> {
        let locked = self
            .locked
            .grantee_locked(deps.storage, grantee_address.clone())?;

        // if we used up all allowance and have no locked tokens, we can just remove the grant from storage
        if grant.allowance.is_used_up() && locked.is_zero() {
            self.grants.remove(deps.storage, grantee_address)
        } else {
            self.grants.save(deps.storage, grantee_address, &grant)?;
        }

        Ok(())
    }

    pub fn remove_grant(
        &self,
        deps: DepsMut,
        grantee_address: GranteeAddress,
    ) -> Result<(), NymPoolContractError> {
        self.grants.remove(deps.storage, grantee_address.clone());

        // if there are any tokens still locked associated with this grantee, unlock them
        if let Some(grantee_locked) = self
            .locked
            .maybe_grantee_locked(deps.storage, grantee_address.clone())?
        {
            self.locked.unlock(deps, grantee_address, grantee_locked)?;
        }

        Ok(())
    }

    pub fn revoke_grant(
        &self,
        deps: DepsMut,
        grantee_address: GranteeAddress,
        revoker: Addr,
    ) -> Result<(), NymPoolContractError> {
        let grant = self.load_grant(deps.as_ref(), &grantee_address)?;
        let original_granter = grant.granter;

        let is_admin = self.is_admin(deps.as_ref(), &revoker)?;

        // grant can only be revoked by the granter who has originally granted it (assuming it's still whitelisted)
        // or by the admin
        if revoker != original_granter && !is_admin {
            // request came from a random sender - neither the original granter nor the current admin
            return Err(NymPoolContractError::UnauthorizedGrantRevocation);
        }

        // at this point we know the request must have come from either the original granter or contract admin,
        // however, if it was the former, we still need to verify whether it's still whitelisted
        // (if the granter was removed, it shouldn't have any permissions to modify old grants anymore)
        if !is_admin && !self.is_whitelisted_granter(deps.as_ref(), &revoker)? {
            return Err(NymPoolContractError::UnauthorizedGrantRevocation);
        }

        self.remove_grant(deps, grantee_address)
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

    fn initialise(&self, deps: DepsMut) -> Result<(), NymPoolContractError> {
        self.total_locked.save(deps.storage, &Uint128::zero())?;
        Ok(())
    }

    pub fn grantee_locked(
        &self,
        storage: &dyn Storage,
        grantee: GranteeAddress,
    ) -> Result<Uint128, NymPoolContractError> {
        Ok(self
            .maybe_grantee_locked(storage, grantee)?
            .unwrap_or_default())
    }

    pub fn maybe_grantee_locked(
        &self,
        storage: &dyn Storage,
        grantee: GranteeAddress,
    ) -> Result<Option<Uint128>, NymPoolContractError> {
        Ok(self.grantees.may_load(storage, grantee.clone())?)
    }

    /// unconditionally attempts to load specified amount of tokens for the particular grantee
    /// it does not validate permissions nor allowances - that's up to the caller
    pub(super) fn lock(
        &self,
        deps: DepsMut,
        grantee: GranteeAddress,
        amount: Uint128,
    ) -> Result<(), NymPoolContractError> {
        let existing_grantee = self.grantee_locked(deps.storage, grantee.clone())?;
        let new_locked_grantee = existing_grantee + amount;

        let existing_total = self.total_locked.load(deps.storage)?;
        let new_locked_total = existing_total + amount;

        self.grantees
            .save(deps.storage, grantee, &new_locked_grantee)?;
        self.total_locked.save(deps.storage, &new_locked_total)?;
        Ok(())
    }

    pub(super) fn unlock(
        &self,
        deps: DepsMut,
        grantee: GranteeAddress,
        amount: Uint128,
    ) -> Result<(), NymPoolContractError> {
        let locked_grantee = self.grantee_locked(deps.storage, grantee.clone())?;
        let total_locked = self.total_locked.load(deps.storage)?;

        if locked_grantee < amount {
            return Err(NymPoolContractError::InsufficientLockedTokens {
                grantee: grantee.to_string(),
                locked: locked_grantee,
                requested: amount,
            });
        }

        let updated_grantee = locked_grantee - amount;

        // if the updated value is zero, just remove the map entry
        if updated_grantee.is_zero() {
            self.grantees.remove(deps.storage, grantee);
        } else {
            self.grantees
                .save(deps.storage, grantee, &updated_grantee)?;
        }

        // we're specifically not using saturating sub here because that operation should ALWAYS be valid
        // if it fails, it means there's a pool inconsistency that has to be resolved
        self.total_locked
            .save(deps.storage, &(total_locked - amount))?;

        Ok(())
    }
}

pub mod retrieval_limits {
    pub const LOCKED_TOKENS_DEFAULT_LIMIT: u32 = 100;
    pub const LOCKED_TOKENS_MAX_LIMIT: u32 = 200;

    pub const GRANTERS_DEFAULT_LIMIT: u32 = 100;
    pub const GRANTERS_MAX_LIMIT: u32 = 200;

    pub const GRANTS_DEFAULT_LIMIT: u32 = 100;
    pub const GRANTS_MAX_LIMIT: u32 = 200;
}

#[cfg(test)]
mod tests {
    use super::*;
}
