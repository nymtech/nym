// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Deps, DepsMut, Env};
use cw_controllers::Admin;
use nym_offline_signers_common::constants::storage_keys;
use nym_offline_signers_common::NymOfflineSignersContractError;

pub const NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE: NymOfflineSignersStorage =
    NymOfflineSignersStorage::new();

pub struct NymOfflineSignersStorage {
    pub(crate) contract_admin: Admin,
}

impl NymOfflineSignersStorage {
    #[allow(clippy::new_without_default)]
    pub(crate) const fn new() -> Self {
        NymOfflineSignersStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
        }
    }

    fn is_admin(&self, deps: Deps, addr: &Addr) -> Result<bool, NymOfflineSignersContractError> {
        self.contract_admin.is_admin(deps, addr).map_err(Into::into)
    }

    fn ensure_is_admin(
        &self,
        deps: Deps,
        addr: &Addr,
    ) -> Result<(), NymOfflineSignersContractError> {
        self.contract_admin
            .assert_admin(deps, addr)
            .map_err(Into::into)
    }

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        env: Env,
        admin: Addr,
    ) -> Result<(), NymOfflineSignersContractError> {
        let _ = deps;
        let _ = env;

        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;
        Ok(())
    }
}

pub mod retrieval_limits {
    //
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod performance_contract_storage {
        use super::*;

        #[cfg(test)]
        mod initialisation {
            use super::*;
            use cosmwasm_std::testing::mock_env;
            use nym_contracts_common_testing::mock_dependencies;

            #[test]
            fn sets_contract_admin() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin1 = deps.api.addr_make("first-admin");
                let admin2 = deps.api.addr_make("second-admin");

                storage.initialise(deps.as_mut(), env.clone(), admin1.clone())?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin1).is_ok());

                storage.initialise(deps.as_mut(), env.clone(), admin2.clone())?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin2).is_ok());

                Ok(())
            }
        }
    }
}
