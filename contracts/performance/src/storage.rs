// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Deps, DepsMut, Env};
use cw_controllers::Admin;
use nym_performance_contract_common::constants::storage_keys;
use nym_performance_contract_common::NymPerformanceContractError;

pub const NYM_PERFORMANCE_CONTRACT_STORAGE: NymPerformanceContractStorage =
    NymPerformanceContractStorage::new();

pub struct NymPerformanceContractStorage {
    pub(crate) contract_admin: Admin,
}

impl NymPerformanceContractStorage {
    #[allow(clippy::new_without_default)]
    const fn new() -> Self {
        NymPerformanceContractStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
        }
    }

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        env: Env,
        admin: Addr,
    ) -> Result<(), NymPerformanceContractError> {
        let _ = deps;
        let _ = env;

        // set the contract admin
        self.contract_admin.set(deps.branch(), Some(admin))?;

        Ok(())
    }

    fn is_admin(&self, deps: Deps, addr: &Addr) -> Result<bool, NymPerformanceContractError> {
        self.contract_admin.is_admin(deps, addr).map_err(Into::into)
    }

    fn ensure_is_admin(&self, deps: Deps, addr: &Addr) -> Result<(), NymPerformanceContractError> {
        self.contract_admin
            .assert_admin(deps, addr)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod performance_contract_storage {
        use super::*;
        use cosmwasm_std::testing::{mock_dependencies, mock_env};

        #[cfg(test)]
        mod initialisation {
            use super::*;
            use cosmwasm_std::testing::{mock_dependencies, mock_env};

            #[test]
            fn sets_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin1 = deps.api.addr_make("first-admin");
                let admin2 = deps.api.addr_make("second-admin");

                storage.initialise(deps.as_mut(), env.clone(), admin1.clone())?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin1).is_ok());

                let mut deps = mock_dependencies();
                storage.initialise(deps.as_mut(), env.clone(), admin2.clone())?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin2).is_ok());

                Ok(())
            }
        }

        #[test]
        fn checking_for_admin() -> anyhow::Result<()> {
            let storage = NymPerformanceContractStorage::new();
            let mut deps = mock_dependencies();
            let env = mock_env();
            let admin = deps.api.addr_make("admin");
            let non_admin = deps.api.addr_make("non-admin");

            storage.initialise(deps.as_mut(), env, admin.clone())?;
            assert!(storage.is_admin(deps.as_ref(), &admin)?);
            assert!(!storage.is_admin(deps.as_ref(), &non_admin)?);

            Ok(())
        }

        #[test]
        fn ensuring_admin_privileges() -> anyhow::Result<()> {
            let storage = NymPerformanceContractStorage::new();
            let mut deps = mock_dependencies();
            let env = mock_env();
            let admin = deps.api.addr_make("admin");
            let non_admin = deps.api.addr_make("non-admin");

            storage.initialise(deps.as_mut(), env, admin.clone())?;
            assert!(storage.ensure_is_admin(deps.as_ref(), &admin).is_ok());
            assert!(storage.ensure_is_admin(deps.as_ref(), &non_admin).is_err());

            Ok(())
        }
    }
}
