// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Deps, DepsMut, Env};
use cw_controllers::Admin;
use nym_network_monitors_contract_common::constants::storage_keys;
use nym_network_monitors_contract_common::NetworkMonitorsContractError;

pub const NETWORK_MONITORS_CONTRACT_STORAGE: NetworkMonitorsStorage = NetworkMonitorsStorage::new();

pub struct NetworkMonitorsStorage {
    pub(crate) contract_admin: Admin,
}

impl NetworkMonitorsStorage {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        NetworkMonitorsStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
        }
    }

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        _env: Env,
        admin: Addr,
    ) -> Result<(), NetworkMonitorsContractError> {
        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;

        Ok(())
    }

    fn is_admin(&self, deps: Deps, addr: &Addr) -> Result<bool, NetworkMonitorsContractError> {
        self.contract_admin.is_admin(deps, addr).map_err(Into::into)
    }

    fn ensure_is_admin(&self, deps: Deps, addr: &Addr) -> Result<(), NetworkMonitorsContractError> {
        self.contract_admin
            .assert_admin(deps, addr)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {

    #[cfg(test)]
    mod network_monitors_storage {
        use crate::storage::NetworkMonitorsStorage;
        use cosmwasm_std::testing::{mock_dependencies, mock_env};

        #[cfg(test)]
        mod initialisation {
            use crate::storage::NetworkMonitorsStorage;
            use cosmwasm_std::testing::{mock_dependencies, mock_env};

            #[test]
            fn sets_contract_admin() -> anyhow::Result<()> {
                let storage = NetworkMonitorsStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin1 = deps.api.addr_make("first-admin");
                let admin2 = deps.api.addr_make("secod-admin");

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
            let storage = NetworkMonitorsStorage::new();
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
            let storage = NetworkMonitorsStorage::new();
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
