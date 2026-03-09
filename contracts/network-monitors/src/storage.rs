// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Deps, DepsMut, Env};
use cw_controllers::Admin;
use cw_storage_plus::Map;
use nym_network_monitors_contract_common::constants::storage_keys;
use nym_network_monitors_contract_common::{
    AuthorisedNetworkMonitor, AuthorisedNetworkMonitorOrchestrator, InstantiateMsg,
    NetworkMonitorAddress, NetworkMonitorsContractError, OrchestratorAddress,
};
use std::net::IpAddr;

pub const NETWORK_MONITORS_CONTRACT_STORAGE: NetworkMonitorsStorage = NetworkMonitorsStorage::new();

/// The storage has an authorisation hierarchy:
/// - At the top there's the contract admin (controlled by Nymtech SA multisig, later by governance)
///   which has ultimate control over the contract and is permitted to authorise new network monitors orchestrators
/// - This is followed by network monitor orchestrators which are permitted to make changes to the set of allowed agents
/// - Finally, at the bottom, authorised network monitor agents which are permitted to send test mixnet packets to Nym nodes
pub struct NetworkMonitorsStorage {
    pub(crate) contract_admin: Admin,
    pub(crate) authorised_orchestrators:
        Map<&'static OrchestratorAddress, AuthorisedNetworkMonitorOrchestrator>,
    pub(crate) authorised_agents: Map<NetworkMonitorAddress, AuthorisedNetworkMonitor>,
}

impl NetworkMonitorsStorage {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        NetworkMonitorsStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            authorised_orchestrators: Map::new(storage_keys::AUTHORISED_ORCHESTRATORS),
            authorised_agents: Map::new(storage_keys::AUTHORISED_NETWORK_MONITORS),
        }
    }

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        env: Env,
        admin: Addr,
        msg: InstantiateMsg,
    ) -> Result<(), NetworkMonitorsContractError> {
        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;

        let orchestrator = deps.api.addr_validate(&msg.orchestrator_address)?;

        // set the initial orchestrator authorisation
        self.authorised_orchestrators.save(
            deps.storage,
            &orchestrator,
            &AuthorisedNetworkMonitorOrchestrator {
                address: orchestrator.clone(),
                authorised_at: env.block.time,
            },
        )?;

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

    fn is_orchestrator(
        &self,
        deps: Deps,
        addr: &Addr,
    ) -> Result<bool, NetworkMonitorsContractError> {
        Ok(self
            .authorised_orchestrators
            .may_load(deps.storage, addr)?
            .is_some())
    }

    fn ensure_is_orchestrator(
        &self,
        deps: Deps,
        addr: &Addr,
    ) -> Result<(), NetworkMonitorsContractError> {
        self.contract_admin
            .assert_admin(deps, addr)
            .map_err(Into::into)
    }

    pub fn authorise_orchestrator(
        &self,
        deps: DepsMut,
        env: &Env,
        sender: &Addr,
        orchestrator_address: OrchestratorAddress,
    ) -> Result<(), NetworkMonitorsContractError> {
        // only contract admin can authorise new orchestrators
        self.ensure_is_admin(deps.as_ref(), sender)?;

        // if orchestrator is already authorised, it's a no-op
        if self.is_orchestrator(deps.as_ref(), &orchestrator_address)? {
            return Ok(());
        }

        self.authorised_orchestrators.save(
            deps.storage,
            &orchestrator_address,
            &AuthorisedNetworkMonitorOrchestrator {
                address: orchestrator_address.clone(),
                authorised_at: env.block.time,
            },
        )?;
        Ok(())
    }

    pub fn remove_orchestrator_authorisation(
        &self,
        deps: DepsMut,
        sender: &Addr,
        orchestrator_address: OrchestratorAddress,
    ) -> Result<(), NetworkMonitorsContractError> {
        self.ensure_is_admin(deps.as_ref(), sender)?;

        self.authorised_orchestrators
            .remove(deps.storage, &orchestrator_address);
        Ok(())
    }

    pub fn authorise_monitor(
        &self,
        deps: DepsMut,
        env: &Env,
        sender: &Addr,
        monitor_address: IpAddr,
    ) -> Result<(), NetworkMonitorsContractError> {
        // only orchestrators can authorise new monitors
        self.ensure_is_orchestrator(deps.as_ref(), sender)?;

        self.authorised_agents.save(
            deps.storage,
            monitor_address.to_string(),
            &AuthorisedNetworkMonitor {
                address: monitor_address,
                authorised_by: sender.clone(),
                authorised_at: env.block.time,
            },
        )?;
        Ok(())
    }

    pub fn remove_monitor_authorisation(
        &self,
        deps: DepsMut,
        sender: &Addr,
        monitor_address: IpAddr,
    ) -> Result<(), NetworkMonitorsContractError> {
        self.ensure_is_orchestrator(deps.as_ref(), sender)?;
        self.authorised_agents
            .remove(deps.storage, monitor_address.to_string());
        Ok(())
    }

    pub fn remove_all_monitors(
        &self,
        deps: DepsMut,
        sender: &Addr,
    ) -> Result<(), NetworkMonitorsContractError> {
        // only the contract admin or an authorised orchestrator can remove all monitors
        if !self.is_admin(deps.as_ref(), sender)? && !self.is_orchestrator(deps.as_ref(), sender)? {
            return Err(NetworkMonitorsContractError::Unauthorized);
        }

        self.authorised_agents.clear(deps.storage);
        Ok(())
    }
}

pub mod retrieval_limits {
    pub const AGENTS_DEFAULT_LIMIT: u32 = 100;
    pub const AGENTS_MAX_LIMIT: u32 = 200;
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
