// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::validate_usage_coin;
use cosmwasm_std::{coin, Addr, Coin, Deps, DepsMut, Env, Storage, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use nym_pool_contract_common::constants::storage_keys;
use nym_pool_contract_common::{
    Allowance, Grant, GranteeAddress, GranterAddress, GranterInformation, NymPoolContractError,
};
use std::cmp::max;
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
        // set the denom
        self.pool_denomination.save(deps.storage, pool_denom)?;

        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;

        // set the admin to be a whitelisted granter
        self.add_new_granter(deps.branch(), &env, &admin, &admin)?;

        // initialise the locked storage (with the total of 0)
        self.locked.initialise(deps.branch())?;

        let included_grants = !initial_grants.is_empty();

        // add all initial grants
        let mut required_amount = Uint128::zero();
        for (grantee, allowance) in initial_grants {
            let grantee = deps.api.addr_validate(&grantee)?;
            if let Some(ref limit) = allowance.basic().spend_limit {
                required_amount += limit.amount;
            }
            self.insert_new_grant(deps.branch(), &env, &admin, &grantee, allowance)?;
        }

        // special case: during initialisation, even if we're inserting unlimited grants,
        // we have to have _some_ tokens available
        if included_grants {
            let balance = self.contract_balance(deps.as_ref(), &env)?;
            if required_amount > balance.amount || balance.amount.is_zero() {
                return Err(NymPoolContractError::InsufficientTokens {
                    required: coin(max(required_amount.u128(), 1), &balance.denom),
                    available: balance,
                });
            }
        }

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
        env: &Env,
        sender: &Addr,
        granter: &GranterAddress,
    ) -> Result<(), NymPoolContractError> {
        // currently only the admin is permitted to add new granters
        self.ensure_is_admin(deps.as_ref(), sender)?;

        if self
            .granters
            .may_load(deps.storage, granter.clone())?
            .is_some()
        {
            return Err(NymPoolContractError::AlreadyAGranter);
        }

        self.granters.save(
            deps.storage,
            granter.clone(),
            &GranterInformation {
                created_by: sender.clone(),
                created_at_height: env.block.height,
            },
        )?;

        Ok(())
    }

    pub fn remove_granter(
        &self,
        deps: DepsMut,
        admin: &Addr,
        granter: &GranterAddress,
    ) -> Result<(), NymPoolContractError> {
        // only admin is permitted to remove granters
        self.ensure_is_admin(deps.as_ref(), admin)?;

        // the granter has to be, well, an actual granter
        self.ensure_is_whitelisted_granter(deps.as_ref(), granter)?;

        self.granters.remove(deps.storage, granter.clone());

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
        grantee: &GranteeAddress,
        mut allowance: Allowance,
    ) -> Result<(), NymPoolContractError> {
        // the granter should be permitted to add new grants
        self.ensure_is_whitelisted_granter(deps.as_ref(), granter)?;

        // check for existing grant
        if let Some(existing_grant) = self.try_load_grant(deps.as_ref(), grantee)? {
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
                    required: spend_limit.clone(),
                });
            }
        }

        // set initial state based on the env
        allowance.set_initial_state(env);

        self.grants.save(
            deps.storage,
            grantee.clone(),
            &Grant {
                granter: granter.clone(),
                grantee: grantee.clone(),
                granted_at_height: env.block.height,
                allowance,
            },
        )?;

        Ok(())
    }

    pub fn try_spend_part_of_grant(
        &self,
        deps: DepsMut,
        env: &Env,
        grantee_address: &GranteeAddress,
        amount: &Coin,
    ) -> Result<(), NymPoolContractError> {
        let mut grant = self.load_grant(deps.as_ref(), grantee_address)?;
        grant.allowance.try_spend(env, amount)?;

        let locked = self.locked.grantee_locked(deps.storage, grantee_address)?;

        // if we used up all allowance and have no locked tokens, we can just remove the grant from storage
        if grant.allowance.is_used_up() && locked.is_zero() {
            self.grants.remove(deps.storage, grantee_address.clone())
        } else {
            self.grants
                .save(deps.storage, grantee_address.clone(), &grant)?;
        }

        Ok(())
    }

    pub fn remove_grant(
        &self,
        deps: DepsMut,
        grantee_address: &GranteeAddress,
    ) -> Result<(), NymPoolContractError> {
        self.grants.remove(deps.storage, grantee_address.clone());

        // if there are any tokens still locked associated with this grantee, unlock them
        if let Some(grantee_locked) = self
            .locked
            .maybe_grantee_locked(deps.storage, grantee_address)?
        {
            self.locked.unlock(deps, grantee_address, grantee_locked)?;
        }

        Ok(())
    }

    pub fn revoke_grant(
        &self,
        deps: DepsMut,
        grantee_address: &GranteeAddress,
        revoker: &Addr,
    ) -> Result<(), NymPoolContractError> {
        let grant = self.load_grant(deps.as_ref(), grantee_address)?;
        let original_granter = grant.granter;

        let is_admin = self.is_admin(deps.as_ref(), revoker)?;

        // grant can only be revoked by the granter who has originally granted it (assuming it's still whitelisted)
        // or by the admin
        if revoker != original_granter && !is_admin {
            // request came from a random sender - neither the original granter nor the current admin
            return Err(NymPoolContractError::UnauthorizedGrantRevocation);
        }

        // at this point we know the request must have come from either the original granter or contract admin,
        // however, if it was the former, we still need to verify whether it's still whitelisted
        // (if the granter was removed, it shouldn't have any permissions to modify old grants anymore)
        if !is_admin && !self.is_whitelisted_granter(deps.as_ref(), revoker)? {
            return Err(NymPoolContractError::UnauthorizedGrantRevocation);
        }

        self.remove_grant(deps, grantee_address)
    }

    pub fn lock_part_of_allowance(
        &self,
        mut deps: DepsMut,
        env: &Env,
        grantee: &GranteeAddress,
        amount: Coin,
    ) -> Result<(), NymPoolContractError> {
        // ensure correct coin has been specified
        validate_usage_coin(deps.storage, &amount)?;

        // keep track of the locked coins
        self.locked.lock(deps.branch(), grantee, amount.amount)?;

        // attempt to deduct the coins from the allowance
        self.try_spend_part_of_grant(deps, env, grantee, &amount)?;

        Ok(())
    }

    pub fn unlock_part_of_allowance(
        &self,
        deps: DepsMut,
        grantee: &GranteeAddress,
        amount: &Coin,
    ) -> Result<(), NymPoolContractError> {
        // ensure correct coin has been specified
        validate_usage_coin(deps.storage, amount)?;

        // update the underlying spend limit of the grant
        let mut grant = self.load_grant(deps.as_ref(), grantee)?;
        // note: this will only increase the basic spend limit and will not change any periodic allowances
        grant.allowance.increase_spend_limit(amount.amount);
        self.grants.save(deps.storage, grantee.clone(), &grant)?;

        // keep track of the locked coins (also checks whether sufficient tokens are locked, etc.)
        self.locked.unlock(deps, grantee, amount.amount)
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
        grantee: &GranteeAddress,
    ) -> Result<Uint128, NymPoolContractError> {
        Ok(self
            .maybe_grantee_locked(storage, grantee)?
            .unwrap_or_default())
    }

    pub fn maybe_grantee_locked(
        &self,
        storage: &dyn Storage,
        grantee: &GranteeAddress,
    ) -> Result<Option<Uint128>, NymPoolContractError> {
        Ok(self.grantees.may_load(storage, grantee.clone())?)
    }

    /// unconditionally attempts to load specified amount of tokens for the particular grantee
    /// it does not validate permissions nor allowances - that's up to the caller
    fn lock(
        &self,
        deps: DepsMut,
        grantee: &GranteeAddress,
        amount: Uint128,
    ) -> Result<(), NymPoolContractError> {
        let existing_grantee = self.grantee_locked(deps.storage, grantee)?;
        let new_locked_grantee = existing_grantee + amount;

        let existing_total = self.total_locked.load(deps.storage)?;
        let new_locked_total = existing_total + amount;

        self.grantees
            .save(deps.storage, grantee.clone(), &new_locked_grantee)?;
        self.total_locked.save(deps.storage, &new_locked_total)?;
        Ok(())
    }

    fn unlock(
        &self,
        deps: DepsMut,
        grantee: &GranteeAddress,
        amount: Uint128,
    ) -> Result<(), NymPoolContractError> {
        let locked_grantee = self.grantee_locked(deps.storage, grantee)?;
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
            self.grantees.remove(deps.storage, grantee.clone());
        } else {
            self.grantees
                .save(deps.storage, grantee.clone(), &updated_grantee)?;
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
    use nym_pool_contract_common::BasicAllowance;

    fn dummy_allowance() -> Allowance {
        Allowance::Basic(BasicAllowance::unlimited())
    }

    #[cfg(test)]
    mod nympool_storage {
        use super::*;
        use crate::testing::{init_contract_tester, NymPoolContractTesterExt, TEST_DENOM};
        use cosmwasm_std::testing::{
            mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage,
        };
        use cosmwasm_std::{coin, coins, Empty, OwnedDeps};
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};
        use nym_pool_contract_common::BasicAllowance;

        #[cfg(test)]
        mod initialisation {
            use super::*;
            use crate::testing::TEST_DENOM;
            use cosmwasm_std::testing::{mock_dependencies, mock_env};
            use cosmwasm_std::{coin, Order};
            use nym_contracts_common_testing::deps_with_balance;
            use nym_pool_contract_common::BasicAllowance;

            fn all_grants(storage: &dyn Storage) -> HashMap<GranteeAddress, Grant> {
                NYM_POOL_STORAGE
                    .grants
                    .range(storage, None, None, Order::Ascending)
                    .collect::<Result<HashMap<_, _>, _>>()
                    .unwrap()
            }

            #[test]
            fn requires_some_tokens_for_unlimited_initial_grants() -> anyhow::Result<()> {
                let storage = NymPoolStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin = deps.api.addr_make("admin");

                let mut grants = HashMap::new();
                grants.insert(
                    deps.api.addr_make("gr1").to_string(),
                    Allowance::Basic(BasicAllowance::unlimited()),
                );
                let res = storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    &TEST_DENOM.to_string(),
                    grants.clone(),
                );
                // we haven't got any tokens - this should have failed
                assert!(res.is_err());

                let address = &env.contract.address;
                let mem_storage = MockStorage::default();
                let api = MockApi::default();

                let querier: MockQuerier<Empty> =
                    MockQuerier::new(&[(address.as_str(), coins(1, TEST_DENOM).as_slice())]);

                let mut deps: OwnedDeps<_, _, _, Empty> = OwnedDeps {
                    storage: mem_storage,
                    api,
                    querier,
                    custom_query_type: Default::default(),
                };

                // while we don't have a lot, we have some tokens which should allow this tx to proceed
                let res = storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    &TEST_DENOM.to_string(),
                    grants.clone(),
                );
                assert!(res.is_ok());

                Ok(())
            }

            #[test]
            fn requires_specified_amount_of_tokens_for_bounded_grants() -> anyhow::Result<()> {
                fn bounded_allowance() -> Allowance {
                    Allowance::Basic(BasicAllowance {
                        spend_limit: Some(coin(100, TEST_DENOM)),
                        expiration_unix_timestamp: None,
                    })
                }

                let storage = NymPoolStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin = deps.api.addr_make("admin");

                let mut grants = HashMap::new();
                grants.insert(deps.api.addr_make("gr1").to_string(), bounded_allowance());
                grants.insert(deps.api.addr_make("gr2").to_string(), bounded_allowance());
                grants.insert(deps.api.addr_make("gr3").to_string(), bounded_allowance());
                grants.insert(deps.api.addr_make("gr4").to_string(), bounded_allowance());
                let res = storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    &TEST_DENOM.to_string(),
                    grants.clone(),
                );
                // we haven't got any tokens - this should have failed
                assert!(res.is_err());

                let address = &env.contract.address;
                let mem_storage = MockStorage::default();
                let api = MockApi::default();

                let querier: MockQuerier<Empty> =
                    MockQuerier::new(&[(address.as_str(), coins(399, TEST_DENOM).as_slice())]);

                let mut deps: OwnedDeps<_, _, _, Empty> = OwnedDeps {
                    storage: mem_storage,
                    api,
                    querier,
                    custom_query_type: Default::default(),
                };

                // we haven't got enough tokens (we need at least 400) - still a failure!
                let res = storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    &TEST_DENOM.to_string(),
                    grants.clone(),
                );
                assert!(res.is_err());

                // finally, with at least 400, it should work
                let mem_storage = MockStorage::default();
                let api = MockApi::default();

                let querier: MockQuerier<Empty> =
                    MockQuerier::new(&[(address.as_str(), coins(400, TEST_DENOM).as_slice())]);

                let mut deps: OwnedDeps<_, _, _, Empty> = OwnedDeps {
                    storage: mem_storage,
                    api,
                    querier,
                    custom_query_type: Default::default(),
                };

                let res = storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    &TEST_DENOM.to_string(),
                    grants,
                );
                assert!(res.is_ok());

                Ok(())
            }

            #[test]
            fn inserts_all_initial_grants() -> anyhow::Result<()> {
                let storage = NymPoolStorage::new();
                let env = mock_env();

                let mut deps = deps_with_balance(&env);

                let admin = deps.api.addr_make("admin");
                let denom = &TEST_DENOM.to_string();

                // no grants
                let grants = HashMap::new();
                storage.initialise(deps.as_mut(), env.clone(), admin.clone(), denom, grants)?;
                assert!(all_grants(&deps.storage).is_empty());

                // one grant
                let mut deps = deps_with_balance(&env);
                let mut grants = HashMap::new();
                grants.insert(deps.api.addr_make("gr1").to_string(), dummy_allowance());
                storage.initialise(deps.as_mut(), env.clone(), admin.clone(), denom, grants)?;
                let all = all_grants(&deps.storage);
                assert_eq!(all.len(), 1);
                let grant = all.get(&deps.api.addr_make("gr1")).unwrap();
                assert_eq!(grant.grantee, deps.api.addr_make("gr1"));
                assert_eq!(grant.allowance, dummy_allowance());

                // multiple grants
                let mut deps = deps_with_balance(&env);
                let mut grants = HashMap::new();
                grants.insert(deps.api.addr_make("gr1").to_string(), dummy_allowance());
                grants.insert(deps.api.addr_make("gr2").to_string(), dummy_allowance());
                grants.insert(deps.api.addr_make("gr3").to_string(), dummy_allowance());
                grants.insert(deps.api.addr_make("gr4").to_string(), dummy_allowance());
                storage.initialise(deps.as_mut(), env.clone(), admin.clone(), denom, grants)?;
                let all = all_grants(&deps.storage);
                assert_eq!(all.len(), 4);
                let grant = all.get(&deps.api.addr_make("gr1")).unwrap();
                assert_eq!(grant.grantee, deps.api.addr_make("gr1"));
                let grant = all.get(&deps.api.addr_make("gr3")).unwrap();
                assert_eq!(grant.grantee, deps.api.addr_make("gr3"));

                // fails on invalid grantee address
                let mut deps = deps_with_balance(&env);
                let mut grants = HashMap::new();
                grants.insert(deps.api.addr_make("gr1").to_string(), dummy_allowance());
                grants.insert("invalid_address".to_string(), dummy_allowance());
                assert!(storage
                    .initialise(deps.as_mut(), env.clone(), admin.clone(), denom, grants)
                    .is_err());

                // fails on invalid allowance
                let mut deps = deps_with_balance(&env);
                let mut grants = HashMap::new();
                grants.insert(
                    deps.api.addr_make("gr1").to_string(),
                    Allowance::Basic(BasicAllowance {
                        spend_limit: Some(coin(0, TEST_DENOM)),
                        expiration_unix_timestamp: None,
                    }),
                );
                assert!(storage
                    .initialise(deps.as_mut(), env.clone(), admin, denom, grants)
                    .is_err());
                Ok(())
            }

            #[test]
            fn sets_pool_denomination() -> anyhow::Result<()> {
                let storage = NymPoolStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin = deps.api.addr_make("admin");

                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    &"somedenom".to_string(),
                    HashMap::new(),
                )?;
                assert_eq!(
                    storage.pool_denomination.load(deps.as_ref().storage)?,
                    "somedenom".to_string()
                );

                let mut deps = mock_dependencies();
                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    &"anotherdenom".to_string(),
                    HashMap::new(),
                )?;
                assert_eq!(
                    storage.pool_denomination.load(deps.as_ref().storage)?,
                    "anotherdenom".to_string()
                );

                Ok(())
            }

            #[test]
            fn sets_contract_admin() -> anyhow::Result<()> {
                let storage = NymPoolStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin1 = deps.api.addr_make("first-admin");
                let admin2 = deps.api.addr_make("secod-admin");

                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin1.clone(),
                    &TEST_DENOM.to_string(),
                    HashMap::new(),
                )?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin1).is_ok());

                let mut deps = mock_dependencies();
                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin2.clone(),
                    &TEST_DENOM.to_string(),
                    HashMap::new(),
                )?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin2).is_ok());

                Ok(())
            }

            #[test]
            fn initialises_locked_storage() -> anyhow::Result<()> {
                let storage = NymPoolStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin = deps.api.addr_make("admin");
                let denom = &TEST_DENOM.to_string();

                storage.initialise(deps.as_mut(), env, admin, denom, HashMap::new())?;
                assert!(storage
                    .locked
                    .total_locked
                    .load(deps.as_ref().storage)?
                    .is_zero());

                Ok(())
            }

            #[test]
            fn adds_admin_to_the_granters_set() -> anyhow::Result<()> {
                let storage = NymPoolStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin1 = deps.api.addr_make("first-admin");
                let admin2 = deps.api.addr_make("second-admin");

                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin1.clone(),
                    &TEST_DENOM.to_string(),
                    HashMap::new(),
                )?;
                assert_eq!(
                    storage
                        .granters
                        .load(&deps.storage, admin1.clone())?
                        .created_by,
                    admin1
                );

                let mut deps = mock_dependencies();
                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin2.clone(),
                    &TEST_DENOM.to_string(),
                    HashMap::new(),
                )?;
                assert_eq!(
                    storage
                        .granters
                        .load(&deps.storage, admin2.clone())?
                        .created_by,
                    admin2
                );

                Ok(())
            }
        }

        #[test]
        fn getting_contract_balance() -> anyhow::Result<()> {
            // it's a simple as running the bank query against the address set in the current env
            // using the pool denom
            let env = mock_env();
            let address = &env.contract.address;

            let storage = MockStorage::default();
            let api = MockApi::default();

            let not_contract = api.addr_make("unrelated-address");
            let pool_denom = TEST_DENOM;

            // set some initial balances
            let querier: MockQuerier<Empty> = MockQuerier::new(&[
                (
                    address.as_str(),
                    vec![coin(1000, pool_denom), coin(2000, "anotherdenom")].as_slice(),
                ),
                (not_contract.as_str(), coins(1234, pool_denom).as_slice()),
            ]);

            let mut deps: OwnedDeps<_, _, _, Empty> = OwnedDeps {
                storage,
                api,
                querier,
                custom_query_type: Default::default(),
            };
            let storage = NymPoolStorage::new();
            let admin = deps.api.addr_make("admin");

            // regardless of other denoms and other accounts, the balance query only returns target denom
            storage.initialise(
                deps.as_mut(),
                env.clone(),
                admin,
                &pool_denom.to_string(),
                HashMap::new(),
            )?;
            assert_eq!(
                storage.contract_balance(deps.as_ref(), &env)?,
                coin(1000, pool_denom)
            );

            Ok(())
        }

        #[test]
        fn checking_for_admin() -> anyhow::Result<()> {
            let storage = NymPoolStorage::new();
            let mut deps = mock_dependencies();
            let env = mock_env();
            let admin = deps.api.addr_make("admin");
            let non_admin = deps.api.addr_make("non-admin");
            let denom = &TEST_DENOM.to_string();

            storage.initialise(deps.as_mut(), env, admin.clone(), denom, HashMap::new())?;
            assert!(storage.is_admin(deps.as_ref(), &admin)?);
            assert!(!storage.is_admin(deps.as_ref(), &non_admin)?);

            Ok(())
        }

        #[test]
        fn ensuring_admin_privileges() -> anyhow::Result<()> {
            let storage = NymPoolStorage::new();
            let mut deps = mock_dependencies();
            let env = mock_env();
            let admin = deps.api.addr_make("admin");
            let non_admin = deps.api.addr_make("non-admin");
            let denom = &TEST_DENOM.to_string();

            storage.initialise(deps.as_mut(), env, admin.clone(), denom, HashMap::new())?;
            assert!(storage.ensure_is_admin(deps.as_ref(), &admin).is_ok());
            assert!(storage.ensure_is_admin(deps.as_ref(), &non_admin).is_err());

            Ok(())
        }

        #[test]
        fn loading_granter_information() -> anyhow::Result<()> {
            let storage = NymPoolStorage::new();
            let mut test = init_contract_tester();

            let granter = test.generate_account();

            // not granter
            let info = storage.try_load_granter(test.deps(), &granter)?;
            assert!(info.is_none());

            // granter
            let admin = test.admin_unchecked();
            test.add_granter(&granter);

            let info = storage.try_load_granter(test.deps(), &granter)?;
            assert_eq!(
                info,
                Some(GranterInformation {
                    created_by: admin,
                    created_at_height: test.env().block.height,
                })
            );

            Ok(())
        }

        #[test]
        fn checking_granter_permission() -> anyhow::Result<()> {
            let storage = NymPoolStorage::new();
            let mut test = init_contract_tester();

            let granter = test.generate_account();
            test.add_granter(&granter);
            let not_granter = test.generate_account();

            let deps = test.deps();
            assert!(storage.is_whitelisted_granter(deps, &granter)?);
            assert!(!storage.is_whitelisted_granter(deps, &not_granter)?);

            Ok(())
        }

        #[test]
        fn ensuring_granter_permission() -> anyhow::Result<()> {
            let storage = NymPoolStorage::new();
            let mut test = init_contract_tester();

            let granter = test.generate_account();
            test.add_granter(&granter);
            let not_granter = test.generate_account();

            let deps = test.deps();
            assert!(storage
                .ensure_is_whitelisted_granter(deps, &granter)
                .is_ok());
            assert!(storage
                .ensure_is_whitelisted_granter(deps, &not_granter)
                .is_err());

            Ok(())
        }

        #[test]
        fn checking_available_tokens() -> anyhow::Result<()> {
            // initialise the contract with some tokens
            let env = mock_env();
            let address = &env.contract.address;

            let storage = MockStorage::default();
            let api = MockApi::default();
            let pool_denom = TEST_DENOM;

            // set some initial balances
            let querier: MockQuerier<Empty> =
                MockQuerier::new(&[(address.as_str(), coins(1000, pool_denom).as_slice())]);

            let mut deps: OwnedDeps<_, _, _, Empty> = OwnedDeps {
                storage,
                api,
                querier,
                custom_query_type: Default::default(),
            };
            let storage = NymPoolStorage::new();
            let admin = deps.api.addr_make("admin");

            storage.initialise(
                deps.as_mut(),
                env.clone(),
                admin,
                &pool_denom.to_string(),
                HashMap::new(),
            )?;

            // if there are no locked tokens, the available equals to the balance
            assert_eq!(
                storage.available_tokens(deps.as_ref(), &env)?,
                coin(1000, pool_denom)
            );

            // however, once we start locking them, it becomes the difference between those

            // some locked
            let dummy_grantee = deps.api.addr_make("grantee");
            storage
                .locked
                .lock(deps.as_mut(), &dummy_grantee, Uint128::new(100))?;

            assert_eq!(
                storage.available_tokens(deps.as_ref(), &env)?,
                coin(900, pool_denom)
            );

            // all locked
            storage
                .locked
                .lock(deps.as_mut(), &dummy_grantee, Uint128::new(900))?;
            assert_eq!(
                storage.available_tokens(deps.as_ref(), &env)?,
                coin(0, pool_denom)
            );

            // locked beyond balance (to check for overflow errors)
            storage
                .locked
                .lock(deps.as_mut(), &dummy_grantee, Uint128::new(1000000))?;
            assert_eq!(
                storage.available_tokens(deps.as_ref(), &env)?,
                coin(0, pool_denom)
            );

            Ok(())
        }

        #[test]
        fn attempting_to_load_grant() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let storage = NymPoolStorage::new();

            // doesn't exist...
            let grantee = test.generate_account();
            assert_eq!(storage.try_load_grant(test.deps(), &grantee)?, None);

            // exists
            test.add_dummy_grant_for(&grantee);
            assert_eq!(
                storage.try_load_grant(test.deps(), &grantee)?,
                Some(Grant {
                    granter: test.admin_unchecked(),
                    grantee,
                    granted_at_height: test.env().block.height,
                    allowance: Allowance::Basic(BasicAllowance::unlimited()),
                })
            );
            Ok(())
        }

        #[test]
        fn loading_grant() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let storage = NymPoolStorage::new();

            // doesn't exist...
            let grantee = test.generate_account();
            assert!(storage.load_grant(test.deps(), &grantee).is_err());

            // exists
            test.add_dummy_grant_for(&grantee);
            assert_eq!(
                storage.load_grant(test.deps(), &grantee)?,
                Grant {
                    granter: test.admin_unchecked(),
                    grantee,
                    granted_at_height: test.env().block.height,
                    allowance: Allowance::Basic(BasicAllowance::unlimited()),
                }
            );
            Ok(())
        }

        #[cfg(test)]
        mod adding_new_granter {
            use super::*;
            use crate::testing::init_contract_tester;
            use cw_controllers::AdminError;

            #[test]
            fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let not_admin = test.generate_account();

                let granter = test.generate_account();

                let (deps, env) = test.deps_mut_env();
                let res = storage
                    .add_new_granter(deps, &env, &not_admin, &granter)
                    .unwrap_err();
                assert_eq!(res, NymPoolContractError::Admin(AdminError::NotAdmin {}));

                let (deps, env) = test.deps_mut_env();
                let res = storage.add_new_granter(deps, &env, &admin, &granter);
                assert!(res.is_ok());

                Ok(())
            }

            #[test]
            fn can_only_be_performed_if_account_is_not_already_a_granter() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let granter = test.generate_account();

                // adding it for the first time...
                let (deps, env) = test.deps_mut_env();
                storage.add_new_granter(deps, &env, &admin, &granter)?;

                // it's already a granter
                let (deps, env) = test.deps_mut_env();
                let res = storage
                    .add_new_granter(deps, &env, &admin, &granter)
                    .unwrap_err();
                assert_eq!(res, NymPoolContractError::AlreadyAGranter);

                Ok(())
            }

            #[test]
            fn saves_basic_metadata() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let granter = test.generate_account();

                // no metadata
                let info = storage.granters.may_load(test.storage(), granter.clone())?;
                assert!(info.is_none());

                let (deps, env) = test.deps_mut_env();
                storage.add_new_granter(deps, &env, &admin, &granter)?;

                let info = storage.granters.may_load(test.storage(), granter.clone())?;
                // it was added by the admin at the current height
                assert_eq!(
                    info,
                    Some(GranterInformation {
                        created_by: admin.clone(),
                        created_at_height: env.block.height,
                    })
                );

                // after changing admin, new address is set as the creator
                let new_admin = test.generate_account();
                // sanity check:
                assert_ne!(admin, new_admin);
                test.change_admin(&new_admin);

                let new_granter = test.generate_account();
                let (deps, env) = test.deps_mut_env();
                storage.add_new_granter(deps, &env, &new_admin, &new_granter)?;
                let info = storage
                    .granters
                    .may_load(test.storage(), new_granter.clone())?;
                // it was added by the new admin at the current height
                assert_eq!(
                    info,
                    Some(GranterInformation {
                        created_by: new_admin.clone(),
                        created_at_height: env.block.height,
                    })
                );

                Ok(())
            }
        }

        #[cfg(test)]
        mod removing_granter {
            use super::*;
            use crate::testing::init_contract_tester;

            #[test]
            fn requires_granter_to_exist() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let granter = test.generate_account();

                assert!(storage
                    .remove_granter(test.deps_mut(), &admin, &granter)
                    .is_err());

                test.add_granter(&granter);
                assert!(storage
                    .remove_granter(test.deps_mut(), &admin, &granter)
                    .is_ok());

                Ok(())
            }

            #[test]
            fn can_only_be_performed_by_admin() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let random_address = test.generate_account();
                let granter = test.generate_account();
                let admin = test.admin_unchecked();
                test.add_granter(&granter);

                // can't be removed by the granter itself
                assert!(storage
                    .remove_granter(test.deps_mut(), &granter, &granter)
                    .is_err());

                // not by some random address
                assert!(storage
                    .remove_granter(test.deps_mut(), &random_address, &granter)
                    .is_err());

                // admin can do it though!
                assert!(storage
                    .remove_granter(test.deps_mut(), &admin, &granter)
                    .is_ok());

                test.add_granter(&granter);
                let new_admin = test.generate_account();
                test.change_admin(&new_admin);

                // old admin can't do anything : (
                assert!(storage
                    .remove_granter(test.deps_mut(), &admin, &granter)
                    .is_err());

                // but new admin can!
                assert!(storage
                    .remove_granter(test.deps_mut(), &new_admin, &granter)
                    .is_ok());

                Ok(())
            }

            #[test]
            fn removes_it_from_granter_list() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let granter = test.generate_account();
                test.add_granter(&granter);

                assert!(storage
                    .granters
                    .may_load(test.storage(), granter.clone())?
                    .is_some());

                storage.remove_granter(test.deps_mut(), &admin, &granter)?;

                assert!(storage
                    .granters
                    .may_load(test.storage(), granter.clone())?
                    .is_none());
                Ok(())
            }
        }

        #[cfg(test)]
        mod adding_new_grant {
            use super::*;
            use crate::testing::init_contract_tester;
            use nym_pool_contract_common::ClassicPeriodicAllowance;

            #[test]
            fn can_only_be_done_by_whitelisted_granter() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let not_valid_granter = test.generate_account();
                let granter = test.generate_account();
                test.add_granter(&granter);

                let grantee = test.generate_account();

                let (deps, env) = test.deps_mut_env();
                let res = storage
                    .insert_new_grant(deps, &env, &not_valid_granter, &grantee, dummy_allowance())
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPoolContractError::InvalidGranter {
                        addr: not_valid_granter.to_string(),
                    }
                );

                let (deps, env) = test.deps_mut_env();
                let res =
                    storage.insert_new_grant(deps, &env, &granter, &grantee, dummy_allowance());

                assert!(res.is_ok());
                Ok(())
            }

            #[test]
            fn cant_be_done_if_grant_already_existed() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let grantee = test.add_dummy_grant().grantee;

                let (deps, env) = test.deps_mut_env();
                let res = storage
                    .insert_new_grant(deps, &env, &admin, &grantee, dummy_allowance())
                    .unwrap_err();

                assert!(matches!(
                    res,
                    NymPoolContractError::GrantAlreadyExist { .. }
                ));

                Ok(())
            }

            #[test]
            fn only_accepts_valid_allowances() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                // allowance with 0 limit and wrong denom
                let bad_allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(coin(0, "invalid-denom")),
                    expiration_unix_timestamp: None,
                });

                let admin = test.admin_unchecked();
                let grantee = test.generate_account();

                let (deps, env) = test.deps_mut_env();
                let res = storage
                    .insert_new_grant(deps, &env, &admin, &grantee, bad_allowance)
                    .unwrap_err();

                assert!(matches!(res, NymPoolContractError::InvalidDenom { .. }));

                Ok(())
            }

            #[test]
            fn explicit_limit_cant_be_larger_than_available_tokens() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let grantee = test.generate_account();

                let available = storage.available_tokens(test.deps(), &test.env())?;

                // just above the available
                let mut limit = available.clone();
                limit.amount += Uint128::new(1);
                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(limit),
                    expiration_unix_timestamp: None,
                });

                let (deps, env) = test.deps_mut_env();
                let res = storage
                    .insert_new_grant(deps, &env, &admin, &grantee, allowance)
                    .unwrap_err();

                assert!(matches!(
                    res,
                    NymPoolContractError::InsufficientTokens { .. }
                ));

                // equal to the available
                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(available.clone()),
                    expiration_unix_timestamp: None,
                });

                let (deps, env) = test.deps_mut_env();
                let res = storage.insert_new_grant(deps, &env, &admin, &grantee, allowance);
                assert!(res.is_ok());

                // and below the available
                let mut test = init_contract_tester();
                let mut limit = available.clone();
                limit.amount -= Uint128::new(1);
                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(limit),
                    expiration_unix_timestamp: None,
                });

                let (deps, env) = test.deps_mut_env();
                let res = storage.insert_new_grant(deps, &env, &admin, &grantee, allowance);
                assert!(res.is_ok());

                Ok(())
            }

            #[test]
            fn updates_allowances_initial_state_and_saves_it_to_storage() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let grantee = test.generate_account();

                let inner = ClassicPeriodicAllowance {
                    basic: BasicAllowance::unlimited(),
                    period_duration_secs: 3600,
                    period_spend_limit: coin(100, TEST_DENOM),
                    period_can_spend: None,
                    period_reset_unix_timestamp: 0,
                };
                let allowance = Allowance::ClassicPeriodic(inner.clone());

                let (deps, env) = test.deps_mut_env();
                let res = storage.insert_new_grant(deps, &env, &admin, &grantee, allowance);
                assert!(res.is_ok());

                let stored_grant = storage.load_grant(test.deps(), &grantee)?;
                let mut expected_inner = inner;
                expected_inner.period_can_spend = Some(expected_inner.period_spend_limit.clone());
                expected_inner.period_reset_unix_timestamp =
                    env.block.time.seconds() + expected_inner.period_duration_secs;
                let expected = Allowance::ClassicPeriodic(expected_inner);

                assert_eq!(stored_grant.allowance, expected);
                assert_eq!(stored_grant.grantee, grantee);
                assert_eq!(stored_grant.granter, admin);
                assert_eq!(stored_grant.granted_at_height, env.block.height);

                Ok(())
            }
        }

        #[cfg(test)]
        mod spending_part_of_grant {
            use super::*;
            use crate::testing::init_contract_tester;

            #[test]
            fn requires_grant_to_exist() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let grantee = test.generate_account();

                let (deps, env) = test.deps_mut_env();
                let res = storage
                    .try_spend_part_of_grant(deps, &env, &grantee, &coin(100, TEST_DENOM))
                    .unwrap_err();

                assert!(matches!(res, NymPoolContractError::GrantNotFound { .. }));

                test.add_dummy_grant_for(&grantee);
                assert!(storage
                    .try_spend_part_of_grant(
                        test.deps_mut(),
                        &env,
                        &grantee,
                        &coin(100, TEST_DENOM)
                    )
                    .is_ok());
                Ok(())
            }

            #[test]
            fn requires_grant_to_be_spendable() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let grantee = test.generate_account();

                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(coin(100, TEST_DENOM)),
                    expiration_unix_timestamp: None,
                });

                let (deps, env) = test.deps_mut_env();
                storage.insert_new_grant(deps, &env, &admin, &grantee, allowance)?;

                let res = storage
                    .try_spend_part_of_grant(
                        test.deps_mut(),
                        &env,
                        &grantee,
                        &coin(200, TEST_DENOM),
                    )
                    .unwrap_err();

                assert_eq!(res, NymPoolContractError::SpendingAboveAllowance);
                Ok(())
            }

            #[test]
            fn updates_stored_grant() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let grantee = test.generate_account();

                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(coin(100, TEST_DENOM)),
                    expiration_unix_timestamp: None,
                });
                let (deps, env) = test.deps_mut_env();
                storage.insert_new_grant(deps, &env, &admin, &grantee, allowance)?;

                storage.try_spend_part_of_grant(
                    test.deps_mut(),
                    &env,
                    &grantee,
                    &coin(40, TEST_DENOM),
                )?;

                let stored_grant = storage.load_grant(test.deps(), &grantee)?;
                assert_eq!(
                    stored_grant.allowance,
                    Allowance::Basic(BasicAllowance {
                        spend_limit: Some(coin(60, TEST_DENOM)),
                        expiration_unix_timestamp: None,
                    })
                );

                Ok(())
            }

            #[test]
            fn removes_grant_from_storage_if_its_used_up() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let grantee1 = test.generate_account();
                let grantee2 = test.generate_account();

                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(coin(100, TEST_DENOM)),
                    expiration_unix_timestamp: None,
                });
                let (deps, env) = test.deps_mut_env();
                storage.insert_new_grant(deps, &env, &admin, &grantee1, allowance.clone())?;

                let (deps, env) = test.deps_mut_env();
                storage.insert_new_grant(deps, &env, &admin, &grantee2, allowance)?;

                // use whole allowance with no locked tokens
                storage.try_spend_part_of_grant(
                    test.deps_mut(),
                    &env,
                    &grantee1,
                    &coin(100, TEST_DENOM),
                )?;
                assert!(storage.try_load_grant(test.deps(), &grantee1)?.is_none());

                // use whole allowance with some locked tokens
                test.lock_allowance(grantee2.as_str(), Uint128::new(50));
                storage.try_spend_part_of_grant(
                    test.deps_mut(),
                    &env,
                    &grantee2,
                    &coin(50, TEST_DENOM),
                )?;

                let stored_grant = storage.load_grant(test.deps(), &grantee2)?;
                assert_eq!(
                    stored_grant.allowance,
                    Allowance::Basic(BasicAllowance {
                        spend_limit: Some(coin(0, TEST_DENOM)),
                        expiration_unix_timestamp: None,
                    })
                );

                // unlock and attempt to spend again
                storage.unlock_part_of_allowance(
                    test.deps_mut(),
                    &grantee2,
                    &coin(50, TEST_DENOM),
                )?;
                storage.try_spend_part_of_grant(
                    test.deps_mut(),
                    &env,
                    &grantee2,
                    &coin(50, TEST_DENOM),
                )?;
                assert!(storage.try_load_grant(test.deps(), &grantee2)?.is_none());

                Ok(())
            }
        }

        #[test]
        fn removing_grant() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let storage = NymPoolStorage::new();

            let grantee = test.generate_account();

            // no-op if doesn't exist
            assert!(storage.remove_grant(test.deps_mut(), &grantee).is_ok());

            // removes the actual entry from the map
            test.add_dummy_grant_for(&grantee);

            assert!(storage
                .grants
                .may_load(test.storage(), grantee.clone())?
                .is_some());

            assert!(storage.remove_grant(test.deps_mut(), &grantee).is_ok());

            assert!(storage
                .grants
                .may_load(test.storage(), grantee.clone())?
                .is_none());

            // if applicable, unlocks any locked tokens
            // (all the details of unlocking are already tested in different unit test(s),
            // so it's sufficient to check any of those occurred)
            let grantee2 = test.add_dummy_grant().grantee;
            test.lock_allowance(grantee2.as_str(), Uint128::new(50));

            assert!(storage.remove_grant(test.deps_mut(), &grantee2).is_ok());

            assert!(storage
                .locked
                .grantees
                .may_load(test.storage(), grantee2)?
                .is_none());
            assert!(storage.locked.total_locked.load(test.storage())?.is_zero());

            Ok(())
        }

        #[cfg(test)]
        mod revoking_grant {
            use super::*;
            use crate::testing::init_contract_tester;

            #[test]
            fn requires_grant_to_exist() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let grantee = test.generate_account();

                assert_eq!(
                    storage
                        .revoke_grant(test.deps_mut(), &grantee, &admin)
                        .unwrap_err(),
                    NymPoolContractError::GrantNotFound {
                        grantee: grantee.to_string(),
                    }
                );

                test.add_dummy_grant_for(&grantee);
                assert!(storage
                    .revoke_grant(test.deps_mut(), &grantee, &admin)
                    .is_ok());

                Ok(())
            }

            #[test]
            fn can_always_be_called_by_current_admin() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let grantee = test.add_dummy_grant().grantee;

                // current admin
                let admin = test.admin_unchecked();
                assert!(storage
                    .revoke_grant(test.deps_mut(), &grantee, &admin)
                    .is_ok());

                // new admin
                let new_admin = test.generate_account();
                let grantee = test.add_dummy_grant().grantee;
                test.change_admin(&new_admin);
                assert!(storage
                    .revoke_grant(test.deps_mut(), &grantee, &new_admin)
                    .is_ok());

                // old admin
                let grantee = test.add_dummy_grant().grantee;
                assert_eq!(
                    storage
                        .revoke_grant(test.deps_mut(), &grantee, &admin)
                        .unwrap_err(),
                    NymPoolContractError::UnauthorizedGrantRevocation
                );

                Ok(())
            }

            #[test]
            fn can_be_called_by_original_granter_if_its_still_whitelisted() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let granter = test.generate_account();
                let grantee1 = test.generate_account();
                let grantee2 = test.generate_account();

                test.add_granter(&granter);
                let env = test.env();
                storage.insert_new_grant(
                    test.deps_mut(),
                    &env,
                    &granter,
                    &grantee1,
                    dummy_allowance(),
                )?;
                storage.insert_new_grant(
                    test.deps_mut(),
                    &env,
                    &granter,
                    &grantee2,
                    dummy_allowance(),
                )?;

                // still whitelisted
                assert!(storage
                    .revoke_grant(test.deps_mut(), &grantee1, &granter)
                    .is_ok());

                // not whitelisted anymore
                storage.remove_granter(test.deps_mut(), &admin, &granter)?;
                assert_eq!(
                    storage
                        .revoke_grant(test.deps_mut(), &grantee2, &granter)
                        .unwrap_err(),
                    NymPoolContractError::UnauthorizedGrantRevocation
                );

                Ok(())
            }

            #[test]
            fn removes_the_underlying_grant() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                let admin = test.admin_unchecked();
                let grantee = test.add_dummy_grant().grantee;

                storage.revoke_grant(test.deps_mut(), &grantee, &admin)?;
                assert!(storage
                    .grants
                    .may_load(test.storage(), grantee.clone())?
                    .is_none());

                Ok(())
            }
        }

        #[cfg(test)]
        mod locking_part_of_allowance {
            use super::*;
            use crate::testing::init_contract_tester;
            use nym_contracts_common_testing::DenomExt;

            #[test]
            fn requires_providing_valid_coin() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let grantee = test.add_dummy_grant().grantee;

                let bad_amount = coin(0, "invalid-denom");
                let good_amount = test.coin(100);

                let env = test.env();

                assert!(storage
                    .lock_part_of_allowance(test.deps_mut(), &env, &grantee, bad_amount)
                    .is_err());
                assert!(storage
                    .lock_part_of_allowance(test.deps_mut(), &env, &grantee, good_amount)
                    .is_ok());

                Ok(())
            }

            #[test]
            fn requires_grant_to_exist() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let grantee = test.generate_account();
                let env = test.env();

                let amount = test.coin(100);

                // doesn't exist
                assert!(storage
                    .lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount.clone())
                    .is_err());

                // does exist
                test.add_dummy_grant_for(&grantee);
                assert!(storage
                    .lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount)
                    .is_ok());
                Ok(())
            }

            #[test]
            fn does_not_allow_locking_more_than_spend_limit() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let admin = test.admin_unchecked();
                let env = test.env();

                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(test.coin(100)),
                    expiration_unix_timestamp: None,
                });
                let grantee = test.generate_account();
                storage.insert_new_grant(test.deps_mut(), &env, &admin, &grantee, allowance)?;

                let amount = test.coin(101);
                assert!(storage
                    .lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount)
                    .is_err());

                let amount = test.coin(100);
                assert!(storage
                    .lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount)
                    .is_ok());

                Ok(())
            }

            #[test]
            fn deducts_locked_amount_from_the_allowance() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let admin = test.admin_unchecked();
                let env = test.env();

                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(test.coin(100)),
                    expiration_unix_timestamp: None,
                });
                let grantee = test.generate_account();
                storage.insert_new_grant(test.deps_mut(), &env, &admin, &grantee, allowance)?;

                let amount = test.coin(40);
                storage.lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount)?;

                let allowance = storage
                    .grants
                    .load(test.storage(), grantee.clone())?
                    .allowance;
                assert_eq!(allowance.basic().spend_limit, Some(test.coin(60)));

                // no-op if there's no limit
                let grantee = test.generate_account();
                let unlimited = Allowance::Basic(BasicAllowance::unlimited());
                storage.insert_new_grant(test.deps_mut(), &env, &admin, &grantee, unlimited)?;

                let amount = test.coin(40);
                storage.lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount)?;
                let allowance = storage
                    .grants
                    .load(test.storage(), grantee.clone())?
                    .allowance;
                assert_eq!(allowance.basic(), &BasicAllowance::unlimited());

                Ok(())
            }

            #[test]
            fn preserves_grant_even_if_resultant_allowance_is_zero() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let admin = test.admin_unchecked();
                let env = test.env();

                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(test.coin(100)),
                    expiration_unix_timestamp: None,
                });
                let grantee = test.generate_account();
                storage.insert_new_grant(test.deps_mut(), &env, &admin, &grantee, allowance)?;

                let amount = test.coin(100);
                storage.lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount)?;

                let allowance = storage
                    .grants
                    .load(test.storage(), grantee.clone())?
                    .allowance;
                assert_eq!(allowance.basic().spend_limit, Some(test.coin(0)));

                Ok(())
            }

            #[test]
            fn updates_internal_locked_counter() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let env = test.env();
                let grantee = test.add_dummy_grant().grantee;
                let amount1 = test.coin(100);
                let amount2 = test.coin(200);

                storage.lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount1)?;
                assert_eq!(
                    storage.locked.grantee_locked(test.storage(), &grantee)?,
                    Uint128::new(100)
                );
                assert_eq!(
                    storage.locked.total_locked.load(test.storage())?,
                    Uint128::new(100)
                );

                // more locked by same grantee
                storage.lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount2)?;
                assert_eq!(
                    storage.locked.grantee_locked(test.storage(), &grantee)?,
                    Uint128::new(300)
                );
                assert_eq!(
                    storage.locked.total_locked.load(test.storage())?,
                    Uint128::new(300)
                );

                Ok(())
            }
        }

        #[cfg(test)]
        mod unlocking_part_of_allowance {
            use super::*;
            use crate::testing::{init_contract_tester, NymPoolContract};
            use nym_contracts_common_testing::{ContractTester, DenomExt};

            fn setup_locked_grant(test: &mut ContractTester<NymPoolContract>) -> Addr {
                let grantee = test.add_dummy_grant().grantee;
                test.lock_allowance(&grantee, Uint128::new(100));
                grantee
            }

            #[test]
            fn requires_providing_valid_coin() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let grantee = setup_locked_grant(&mut test);

                let bad_amount = coin(0, "invalid-denom");
                let good_amount = test.coin(100);

                assert!(storage
                    .unlock_part_of_allowance(test.deps_mut(), &grantee, &bad_amount)
                    .is_err());
                assert!(storage
                    .unlock_part_of_allowance(test.deps_mut(), &grantee, &good_amount)
                    .is_ok());

                Ok(())
            }

            #[test]
            fn does_not_allow_unlocking_more_than_currently_locked() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let grantee = setup_locked_grant(&mut test);

                let amount = test.coin(101);
                assert!(storage
                    .unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)
                    .is_err());

                let amount = test.coin(100);
                assert!(storage
                    .unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)
                    .is_ok());
                Ok(())
            }

            #[test]
            fn requires_grant_to_exist() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let grantee = test.generate_account();

                let amount = test.coin(100);

                assert!(storage
                    .unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)
                    .is_err());
                test.add_dummy_grant_for(&grantee);
                test.lock_allowance(&grantee, Uint128::new(100));
                assert!(storage
                    .unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)
                    .is_ok());
                Ok(())
            }

            #[test]
            fn requires_having_locked_coins() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let grantee = test.add_dummy_grant().grantee;

                let amount = test.coin(100);

                assert!(storage
                    .unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)
                    .is_err());
                test.lock_allowance(&grantee, Uint128::new(100));
                assert!(storage
                    .unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)
                    .is_ok());
                Ok(())
            }

            #[test]
            fn increases_internal_grant_spend_limit() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();
                let admin = test.admin_unchecked();
                let env = test.env();

                let allowance = Allowance::Basic(BasicAllowance {
                    spend_limit: Some(test.coin(100)),
                    expiration_unix_timestamp: None,
                });
                let grantee = test.generate_account();
                storage.insert_new_grant(test.deps_mut(), &env, &admin, &grantee, allowance)?;

                let amount = test.coin(40);
                storage.lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount)?;

                let amount = test.coin(20);
                storage.unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)?;

                let allowance = storage
                    .grants
                    .load(test.storage(), grantee.clone())?
                    .allowance;
                assert_eq!(allowance.basic().spend_limit, Some(test.coin(80)));

                // no-op if there's no limit
                let grantee = test.generate_account();
                let unlimited = Allowance::Basic(BasicAllowance::unlimited());
                storage.insert_new_grant(test.deps_mut(), &env, &admin, &grantee, unlimited)?;

                let amount = test.coin(40);
                storage.lock_part_of_allowance(test.deps_mut(), &env, &grantee, amount)?;

                let amount = test.coin(20);
                storage.unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)?;

                let allowance = storage
                    .grants
                    .load(test.storage(), grantee.clone())?
                    .allowance;
                assert_eq!(allowance.basic(), &BasicAllowance::unlimited());

                Ok(())
            }

            #[test]
            fn updates_internal_locked_counter() -> anyhow::Result<()> {
                let mut test = init_contract_tester();
                let storage = NymPoolStorage::new();

                // 100tokens locked
                let grantee = setup_locked_grant(&mut test);

                let amount = test.coin(20);
                storage.unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)?;
                assert_eq!(
                    storage.locked.grantee_locked(test.storage(), &grantee)?,
                    Uint128::new(80)
                );
                assert_eq!(
                    storage.locked.total_locked.load(test.storage())?,
                    Uint128::new(80)
                );

                let amount = test.coin(80);
                storage.unlock_part_of_allowance(test.deps_mut(), &grantee, &amount)?;
                assert!(storage
                    .locked
                    .grantees
                    .may_load(test.storage(), grantee)?
                    .is_none(),);
                assert!(storage.locked.total_locked.load(test.storage())?.is_zero());

                Ok(())
            }
        }
    }

    #[cfg(test)]
    mod locked_storage {
        use super::*;
        use crate::testing::{init_contract_tester, NymPoolContractTesterExt};
        use cosmwasm_std::testing::mock_dependencies;
        use nym_contracts_common_testing::{ContractOpts, RandExt};

        #[test]
        fn is_initialised_with_zero_total_locked() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let storage = LockedStorage::new();

            // by default, when created, the `total_locked` is inaccessible
            assert!(storage.total_locked.load(&deps.storage).is_err());

            storage.initialise(deps.as_mut())?;
            // but after proper initialisation, it's correctly set to 0
            assert_eq!(storage.total_locked.load(&deps.storage)?, Uint128::zero());

            Ok(())
        }

        #[test]
        fn getting_grantee_locked() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let grantee = test.generate_account();

            let storage = LockedStorage::new();

            // returns zero when there's nothing
            assert!(storage
                .grantee_locked(test.deps().storage, &grantee)?
                .is_zero());

            // even when a grant is created (but with nothing locked!)
            test.add_dummy_grant_for(&grantee);
            assert!(storage
                .grantee_locked(test.deps().storage, &grantee)?
                .is_zero());
            let to_lock = Uint128::new(100);

            // lock some tokens...
            test.lock_allowance(&grantee, to_lock);

            // now we're talking!
            assert_eq!(
                storage.grantee_locked(test.deps().storage, &grantee)?,
                to_lock
            );

            Ok(())
        }

        #[test]
        fn getting_maybe_grantee_locked() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let grantee = test.generate_account();

            let storage = LockedStorage::new();

            // returns None when there's nothing
            assert!(storage
                .maybe_grantee_locked(test.deps().storage, &grantee)?
                .is_none());

            // even when a grant is created (but with nothing locked!)
            test.add_dummy_grant_for(&grantee);
            assert!(storage
                .maybe_grantee_locked(test.deps().storage, &grantee)?
                .is_none());
            let to_lock = Uint128::new(100);

            // lock some tokens...
            test.lock_allowance(&grantee, to_lock);

            // now we're talking!
            assert_eq!(
                storage.maybe_grantee_locked(test.deps().storage, &grantee)?,
                Some(to_lock)
            );

            Ok(())
        }

        #[test]
        fn locking_tokens() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let storage = LockedStorage::new();

            let grantee1 = test.generate_account();
            let grantee2 = test.generate_account();

            let first = Uint128::new(100);
            let second = Uint128::new(200);
            let third = Uint128::new(500);

            let all = test.full_locked_map();
            assert!(all.is_empty());

            // fresh one creates entry for grantee with the amount
            // and updates the total
            let deps = test.deps_mut();
            storage.lock(deps, &grantee1, first)?;

            let deps = test.deps();
            assert_eq!(storage.total_locked.load(deps.storage)?, first);
            assert_eq!(storage.grantee_locked(deps.storage, &grantee1)?, first);

            let all = test.full_locked_map();
            assert_eq!(all.len(), 1);

            // another one updates existing entries (and doesn't overwrite them!)
            let deps = test.deps_mut();
            storage.lock(deps, &grantee1, second)?;

            let deps = test.deps();
            assert_eq!(storage.total_locked.load(deps.storage)?, first + second);
            assert_eq!(
                storage.grantee_locked(deps.storage, &grantee1)?,
                first + second
            );

            let all = test.full_locked_map();
            assert_eq!(all.len(), 1);

            // if we do it for a new grantee, another entry is created, but the same total is updated
            let deps = test.deps_mut();
            storage.lock(deps, &grantee2, third)?;

            let deps = test.deps();
            assert_eq!(
                storage.total_locked.load(deps.storage)?,
                first + second + third
            );
            assert_eq!(
                storage.grantee_locked(deps.storage, &grantee1)?,
                first + second
            );
            assert_eq!(storage.grantee_locked(deps.storage, &grantee2)?, third);

            let all = test.full_locked_map();
            assert_eq!(all.len(), 2);
            Ok(())
        }

        #[test]
        fn unlocking_tokens() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let storage = LockedStorage::new();

            let grantee1 = test.generate_account();
            let grantee2 = test.generate_account();

            test.add_dummy_grant_for(&grantee1);
            test.add_dummy_grant_for(&grantee2);

            let first = Uint128::new(100);
            let second = Uint128::new(200);
            let third = Uint128::new(500);

            let all = test.full_locked_map();
            assert!(all.is_empty());

            let deps = test.deps_mut();

            // can't unlock anything if there's nothing locked
            assert!(matches!(
                storage.unlock(deps, &grantee1, first).unwrap_err(),
                NymPoolContractError::InsufficientLockedTokens { .. }
            ));

            test.lock_allowance(&grantee1, first);

            // can't unlock more than the total locked amount
            let deps = test.deps_mut();
            assert!(matches!(
                storage.unlock(deps, &grantee1, first + second).unwrap_err(),
                NymPoolContractError::InsufficientLockedTokens { .. }
            ));
            test.lock_allowance(&grantee1, second);
            test.lock_allowance(&grantee2, third);
            let all = test.full_locked_map();
            assert_eq!(all.len(), 2);

            // unlocking partial amount correctly updates entries
            let deps = test.deps_mut();
            assert!(storage.unlock(deps, &grantee1, first).is_ok());

            let deps = test.deps_mut();
            assert_eq!(storage.total_locked.load(deps.storage)?, second + third);
            assert_eq!(storage.grantee_locked(deps.storage, &grantee1)?, second);
            let all = test.full_locked_map();
            assert_eq!(all.len(), 2);

            // unlocking the remaining amount will remove the entry
            let deps = test.deps_mut();
            assert!(storage.unlock(deps, &grantee1, second).is_ok());
            let deps = test.deps_mut();
            assert_eq!(storage.total_locked.load(deps.storage)?, third);
            assert!(storage.grantee_locked(deps.storage, &grantee1)?.is_zero());
            let all = test.full_locked_map();
            assert_eq!(all.len(), 1);

            // similarly if the full amount is unlocked immediately
            let deps = test.deps_mut();
            assert!(storage.unlock(deps, &grantee2, third).is_ok());
            let deps = test.deps_mut();
            assert!(storage.total_locked.load(deps.storage)?.is_zero());
            assert!(storage.grantee_locked(deps.storage, &grantee2)?.is_zero());

            let all = test.full_locked_map();
            assert!(all.is_empty());

            Ok(())
        }
    }
}
