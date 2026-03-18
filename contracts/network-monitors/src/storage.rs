// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Deps, DepsMut, Env, StdError, StdResult};
use cw_controllers::Admin;
use cw_storage_plus::{Key, KeyDeserialize, Map, PrimaryKey};
use nym_network_monitors_contract_common::constants::storage_keys;
use nym_network_monitors_contract_common::{
    AuthorisedNetworkMonitor, AuthorisedNetworkMonitorOrchestrator, NetworkMonitorsContractError,
    OrchestratorAddress,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

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
    pub(crate) authorised_agents: Map<AgentStorageKey, AuthorisedNetworkMonitor>,
}

// implement explicit wrapper for the storage key rather than use string representation
// to make use of type safety and avoid accidentally mixing up `IpAddr` and `SocketAddr`
#[derive(Clone, Copy, Debug)]
pub(crate) struct AgentStorageKey(IpAddr);

impl<'a> PrimaryKey<'a> for AgentStorageKey {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&'_ self) -> Vec<Key<'_>> {
        match self.0 {
            IpAddr::V4(ipv4) => vec![Key::Val32(ipv4.octets())],
            IpAddr::V6(ipv6) => vec![Key::Val128(ipv6.octets())],
        }
    }
}

impl KeyDeserialize for AgentStorageKey {
    type Output = AgentStorageKey;
    const KEY_ELEMS: u16 = 1;

    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        // SAFETY: we're using the correct number of bytes for the conversion
        #[allow(clippy::unwrap_used)]
        let ip = match value.len() {
            4 => IpAddr::V4(Ipv4Addr::from(TryInto::<[u8; 4]>::try_into(value).unwrap())),
            16 => IpAddr::V6(Ipv6Addr::from(
                TryInto::<[u8; 16]>::try_into(value).unwrap(),
            )),
            _ => return Err(StdError::generic_err("invalid IP address length")),
        };
        Ok(AgentStorageKey(ip))
    }
}

impl From<IpAddr> for AgentStorageKey {
    fn from(ip: IpAddr) -> Self {
        Self(ip)
    }
}

impl From<SocketAddr> for AgentStorageKey {
    fn from(addr: SocketAddr) -> Self {
        Self(addr.ip())
    }
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
        orchestrator: Addr,
    ) -> Result<(), NetworkMonitorsContractError> {
        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;

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

    pub(crate) fn is_orchestrator(
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
        if !self.is_orchestrator(deps, addr)? {
            return Err(NetworkMonitorsContractError::NotAnOrchestrator { addr: addr.clone() });
        }
        Ok(())
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
        monitor_address: SocketAddr,
        bs58_x25519_noise: String,
        noise_version: u8,
    ) -> Result<(), NetworkMonitorsContractError> {
        // only orchestrators can authorise new monitors
        self.ensure_is_orchestrator(deps.as_ref(), sender)?;

        self.authorised_agents.save(
            deps.storage,
            monitor_address.into(),
            &AuthorisedNetworkMonitor {
                mixnet_address: monitor_address,
                authorised_by: sender.clone(),
                authorised_at: env.block.time,
                bs58_x25519_noise,
                noise_version,
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
            .remove(deps.storage, monitor_address.into());
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
                let admin2 = deps.api.addr_make("second-admin");
                let orchestrator = deps.api.addr_make("orchestrator");

                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin1.clone(),
                    orchestrator.clone().clone(),
                )?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin1).is_ok());

                let mut deps = mock_dependencies();
                storage.initialise(deps.as_mut(), env.clone(), admin2.clone(), orchestrator)?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin2).is_ok());

                Ok(())
            }

            #[test]
            fn sets_the_initial_orchestrator() -> anyhow::Result<()> {
                let storage = NetworkMonitorsStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin = deps.api.addr_make("admin");
                let orchestrator1 = deps.api.addr_make("orchestrator");
                let orchestrator2 = deps.api.addr_make("orchestrator");

                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    orchestrator1.clone(),
                )?;
                assert!(storage
                    .ensure_is_orchestrator(deps.as_ref(), &orchestrator1)
                    .is_ok());

                let mut deps = mock_dependencies();
                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    orchestrator2.clone(),
                )?;
                assert!(storage
                    .ensure_is_orchestrator(deps.as_ref(), &orchestrator2)
                    .is_ok());

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
            let orchestrator = deps.api.addr_make("orchestrator");

            storage.initialise(deps.as_mut(), env, admin.clone(), orchestrator)?;
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
            let orchestrator = deps.api.addr_make("orchestrator");

            storage.initialise(deps.as_mut(), env, admin.clone(), orchestrator)?;
            assert!(storage.ensure_is_admin(deps.as_ref(), &admin).is_ok());
            assert!(storage.ensure_is_admin(deps.as_ref(), &non_admin).is_err());

            Ok(())
        }

        #[test]
        fn checking_for_orchestrator() -> anyhow::Result<()> {
            let storage = NetworkMonitorsStorage::new();
            let mut deps = mock_dependencies();
            let env = mock_env();
            let admin = deps.api.addr_make("admin");
            let non_orchestrator = deps.api.addr_make("non-orchestrator");
            let orchestrator = deps.api.addr_make("orchestrator");

            storage.initialise(deps.as_mut(), env, admin, orchestrator.clone())?;
            assert!(storage.is_orchestrator(deps.as_ref(), &orchestrator)?);
            assert!(!storage.is_orchestrator(deps.as_ref(), &non_orchestrator)?);

            Ok(())
        }

        #[test]
        fn ensuring_orchestrator_privileges() -> anyhow::Result<()> {
            let storage = NetworkMonitorsStorage::new();
            let mut deps = mock_dependencies();
            let env = mock_env();
            let admin = deps.api.addr_make("admin");
            let non_orchestrator = deps.api.addr_make("non-orchestrator");
            let orchestrator = deps.api.addr_make("orchestrator");

            storage.initialise(deps.as_mut(), env, admin, orchestrator.clone())?;
            assert!(storage
                .ensure_is_orchestrator(deps.as_ref(), &orchestrator)
                .is_ok());
            assert!(storage
                .ensure_is_orchestrator(deps.as_ref(), &non_orchestrator)
                .is_err());

            Ok(())
        }

        #[cfg(test)]
        mod authorising_orchestrator {
            use super::*;
            use crate::testing::init_contract_tester;
            use cw_controllers::AdminError;
            use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
            use nym_network_monitors_contract_common::NetworkMonitorsContractError;

            #[test]
            fn can_only_be_done_by_admin() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let non_admin = tester.generate_account();
                let orchestrator = tester.generate_account();

                let env = tester.env();
                let deps = tester.deps_mut();
                let res = storage
                    .authorise_orchestrator(deps, &env, &non_admin, orchestrator.clone())
                    .unwrap_err();
                assert_eq!(
                    NetworkMonitorsContractError::Admin(AdminError::NotAdmin {}),
                    res
                );

                let env = tester.env();
                let deps = tester.deps_mut();
                let res2 = storage.authorise_orchestrator(deps, &env, &admin, orchestrator.clone());
                assert_eq!(res2, Ok(()));

                Ok(())
            }

            #[test]
            fn inserts_new_entry_for_fresh_accounts() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let orchestrator = tester.generate_account();

                let env = tester.env();
                let deps = tester.deps_mut();

                assert!(storage
                    .authorised_orchestrators
                    .may_load(deps.storage, &orchestrator)?
                    .is_none());
                storage.authorise_orchestrator(deps, &env, &admin, orchestrator.clone())?;

                let info = storage
                    .authorised_orchestrators
                    .load(&tester, &orchestrator)?;

                assert_eq!(info.address, orchestrator);
                assert_eq!(info.authorised_at, env.block.time);

                Ok(())
            }

            #[test]
            fn no_op_for_older_accounts() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let orchestrator = tester.generate_account();

                let env = tester.env();
                let deps = tester.deps_mut();

                storage.authorise_orchestrator(deps, &env, &admin, orchestrator.clone())?;
                let info = storage
                    .authorised_orchestrators
                    .load(&tester, &orchestrator)?;

                tester.advance_day_of_blocks();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_orchestrator(deps, &env, &admin, orchestrator.clone())?;

                let updated_info = storage
                    .authorised_orchestrators
                    .load(&tester, &orchestrator)?;

                assert_eq!(info, updated_info);

                Ok(())
            }
        }

        #[cfg(test)]
        mod removing_orchestrator_authorisation {
            use super::*;
            use crate::testing::init_contract_tester;
            use cw_controllers::AdminError;
            use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};
            use nym_network_monitors_contract_common::NetworkMonitorsContractError;

            #[test]
            fn can_only_be_done_by_admin() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let non_admin = tester.generate_account();
                let orchestrator = tester.generate_account();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_orchestrator(deps, &env, &admin, orchestrator.clone())?;

                let deps = tester.deps_mut();
                let res = storage
                    .remove_orchestrator_authorisation(deps, &non_admin, orchestrator.clone())
                    .unwrap_err();
                assert_eq!(
                    NetworkMonitorsContractError::Admin(AdminError::NotAdmin {}),
                    res
                );

                let deps = tester.deps_mut();
                let res2 = storage.remove_orchestrator_authorisation(deps, &admin, orchestrator);
                assert_eq!(res2, Ok(()));

                Ok(())
            }

            #[test]
            fn deletes_entry_from_storage() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let orchestrator = tester.generate_account();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_orchestrator(deps, &env, &admin, orchestrator.clone())?;

                assert!(storage
                    .authorised_orchestrators
                    .may_load(&tester, &orchestrator)?
                    .is_some());

                let deps = tester.deps_mut();
                storage.remove_orchestrator_authorisation(deps, &admin, orchestrator.clone())?;

                assert!(storage
                    .authorised_orchestrators
                    .may_load(&tester, &orchestrator)?
                    .is_none());

                Ok(())
            }

            #[test]
            fn no_op_for_non_existent_entries() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let orchestrator = tester.generate_account();

                assert!(storage
                    .authorised_orchestrators
                    .may_load(&tester, &orchestrator)?
                    .is_none());

                let deps = tester.deps_mut();
                let res =
                    storage.remove_orchestrator_authorisation(deps, &admin, orchestrator.clone());
                assert_eq!(res, Ok(()));

                assert!(storage
                    .authorised_orchestrators
                    .may_load(&tester, &orchestrator)?
                    .is_none());

                Ok(())
            }
        }

        #[cfg(test)]
        mod authorising_monitors {
            use super::*;
            use crate::testing::{init_contract_tester, NetworkMonitorsContractTesterExt};
            use nym_contracts_common_testing::{ChainOpts, ContractOpts, RandExt};
            use nym_network_monitors_contract_common::NetworkMonitorsContractError;

            #[test]
            fn can_only_be_done_by_an_orchestrator() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let orchestrator = tester.add_orchestrator()?;
                let non_orchestrator = tester.generate_account();
                let agent = tester.random_socket();

                let env = tester.env();
                let deps = tester.deps_mut();
                let res = storage
                    .authorise_monitor(
                        deps,
                        &env,
                        &non_orchestrator,
                        agent,
                        "test_noise_key".to_string(),
                        1,
                    )
                    .unwrap_err();
                assert_eq!(
                    NetworkMonitorsContractError::NotAnOrchestrator {
                        addr: non_orchestrator.clone()
                    },
                    res
                );

                let env = tester.env();
                let deps = tester.deps_mut();
                let res2 = storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                );
                assert_eq!(res2, Ok(()));

                Ok(())
            }

            #[test]
            fn inserts_new_entry_for_fresh_accounts() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                // IPV4:
                let orchestrator = tester.add_orchestrator()?;
                let agent = tester.random_socket_ipv4();

                let env = tester.env();
                let deps = tester.deps_mut();

                assert!(storage
                    .authorised_agents
                    .may_load(deps.storage, agent.into())?
                    .is_none());
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let info = storage.authorised_agents.load(&tester, agent.into())?;

                assert_eq!(info.mixnet_address, agent);
                assert_eq!(info.authorised_by, orchestrator);
                assert_eq!(info.authorised_at, env.block.time);

                tester.advance_day_of_blocks();

                // IPV6:
                let agent = tester.random_socket_ipv6();

                let env = tester.env();
                let deps = tester.deps_mut();

                assert!(storage
                    .authorised_agents
                    .may_load(deps.storage, agent.into())?
                    .is_none());
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let info = storage.authorised_agents.load(&tester, agent.into())?;

                assert_eq!(info.mixnet_address, agent);
                assert_eq!(info.authorised_by, orchestrator);
                assert_eq!(info.authorised_at, env.block.time);

                Ok(())
            }

            #[test]
            fn updates_timestamp_for_older_accounts() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                // IPV4:
                let orchestrator = tester.add_orchestrator()?;
                let agent = tester.random_socket_ipv4();

                let env = tester.env();
                let deps = tester.deps_mut();

                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let initial_time = env.block.time;
                tester.advance_day_of_blocks();
                let new_expected_time = tester.env().block.time;

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let updated_info = storage.authorised_agents.load(&tester, agent.into())?;

                assert_eq!(updated_info.mixnet_address, agent);
                assert_eq!(updated_info.authorised_by, orchestrator);
                assert_ne!(updated_info.authorised_at, initial_time);
                assert_eq!(updated_info.authorised_at, new_expected_time);

                tester.advance_day_of_blocks();

                // IPV6:
                let agent = tester.random_socket_ipv6();

                let env = tester.env();
                let deps = tester.deps_mut();

                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let initial_time = env.block.time;
                tester.advance_day_of_blocks();
                let new_expected_time = tester.env().block.time;

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let updated_info = storage.authorised_agents.load(&tester, agent.into())?;

                assert_eq!(updated_info.mixnet_address, agent);
                assert_eq!(updated_info.authorised_by, orchestrator);
                assert_ne!(updated_info.authorised_at, initial_time);
                assert_eq!(updated_info.authorised_at, new_expected_time);

                Ok(())
            }
        }

        #[cfg(test)]
        mod removing_monitor_authorisation {
            use super::*;
            use crate::testing::{init_contract_tester, NetworkMonitorsContractTesterExt};
            use nym_contracts_common_testing::{ChainOpts, ContractOpts, RandExt};
            use nym_network_monitors_contract_common::NetworkMonitorsContractError;

            #[test]
            fn can_only_be_done_by_an_orchestrator() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let orchestrator = tester.add_orchestrator()?;
                let non_orchestrator = tester.generate_account();
                let agent = tester.random_socket();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let deps = tester.deps_mut();
                let res = storage
                    .remove_monitor_authorisation(deps, &non_orchestrator, agent.ip())
                    .unwrap_err();
                assert_eq!(
                    NetworkMonitorsContractError::NotAnOrchestrator {
                        addr: non_orchestrator.clone()
                    },
                    res
                );

                let deps = tester.deps_mut();
                let res2 = storage.remove_monitor_authorisation(deps, &orchestrator, agent.ip());
                assert_eq!(res2, Ok(()));

                Ok(())
            }

            #[test]
            fn deletes_entry_from_storage() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                // IPV4:
                let orchestrator = tester.add_orchestrator()?;
                let agent = tester.random_socket_ipv4();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                )?;

                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent.into())?
                    .is_some());

                let deps = tester.deps_mut();
                storage.remove_monitor_authorisation(deps, &orchestrator, agent.ip())?;

                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent.into())?
                    .is_none());

                tester.advance_day_of_blocks();

                // IPV6:
                let agent = tester.random_socket_ipv6();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent,
                    "test_noise_key".to_string(),
                    1,
                )?;

                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent.into())?
                    .is_some());

                let deps = tester.deps_mut();
                storage.remove_monitor_authorisation(deps, &orchestrator, agent.ip())?;

                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent.into())?
                    .is_none());

                Ok(())
            }

            #[test]
            fn no_op_for_non_existent_entries() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let orchestrator = tester.add_orchestrator()?;
                let agent = tester.random_socket();

                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent.into())?
                    .is_none());

                let deps = tester.deps_mut();
                let res = storage.remove_monitor_authorisation(deps, &orchestrator, agent.ip());
                assert_eq!(res, Ok(()));

                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent.into())?
                    .is_none());

                Ok(())
            }
        }

        #[cfg(test)]
        mod removing_all_monitors {
            use super::*;
            use crate::testing::{
                init_contract_tester, NetworkMonitorsContract, NetworkMonitorsContractTesterExt,
            };
            use cosmwasm_std::Addr;
            use nym_contracts_common_testing::{AdminExt, ContractOpts, ContractTester, RandExt};
            use nym_network_monitors_contract_common::NetworkMonitorsContractError;

            fn setup_prepopulated_tester() -> (ContractTester<NetworkMonitorsContract>, Addr) {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();
                let orchestrator = tester.add_orchestrator().unwrap();

                // Prepopulate with several agents
                let agent1 = tester.random_socket();
                let agent2 = tester.random_socket();
                let agent3 = tester.random_socket();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage
                    .authorise_monitor(
                        deps,
                        &env,
                        &orchestrator,
                        agent1,
                        "test_noise_key".to_string(),
                        1,
                    )
                    .unwrap();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage
                    .authorise_monitor(
                        deps,
                        &env,
                        &orchestrator,
                        agent2,
                        "test_noise_key".to_string(),
                        1,
                    )
                    .unwrap();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage
                    .authorise_monitor(
                        deps,
                        &env,
                        &orchestrator,
                        agent3,
                        "test_noise_key".to_string(),
                        1,
                    )
                    .unwrap();

                // sanity check to make sure all agents got added
                let all_agents = tester.all_agents();
                assert_eq!(all_agents.len(), 3);
                assert!(all_agents.contains(&agent1));
                assert!(all_agents.contains(&agent2));
                assert!(all_agents.contains(&agent3));

                (tester, orchestrator)
            }

            #[test]
            fn can_be_done_by_admin() -> anyhow::Result<()> {
                let storage = NetworkMonitorsStorage::new();

                let (mut tester, _) = setup_prepopulated_tester();
                let admin = tester.admin_unchecked();

                let all_agents = tester.all_agents();

                // Admin can call this method
                let deps = tester.deps_mut();
                storage.remove_all_monitors(deps, &admin)?;

                // Verify all agents are cleared
                for agent in all_agents {
                    assert!(storage
                        .authorised_agents
                        .may_load(&tester, agent.into())?
                        .is_none());
                }

                assert!(tester.all_agents().is_empty());

                Ok(())
            }

            #[test]
            fn can_be_done_by_orchestrator() -> anyhow::Result<()> {
                let storage = NetworkMonitorsStorage::new();
                let (mut tester, orchestrator) = setup_prepopulated_tester();

                let all_agents = tester.all_agents();

                let deps = tester.deps_mut();
                storage.remove_all_monitors(deps, &orchestrator)?;

                // Verify all agents are cleared
                for agent in all_agents {
                    assert!(storage
                        .authorised_agents
                        .may_load(&tester, agent.into())?
                        .is_none());
                }

                assert!(tester.all_agents().is_empty());

                Ok(())
            }

            #[test]
            fn cannot_be_done_by_non_privileged_account() -> anyhow::Result<()> {
                let storage = NetworkMonitorsStorage::new();
                let (mut tester, _) = setup_prepopulated_tester();

                let non_privileged = tester.generate_account();

                // Non-privileged account cannot call this method
                let deps = tester.deps_mut();
                let res = storage
                    .remove_all_monitors(deps, &non_privileged)
                    .unwrap_err();
                assert_eq!(NetworkMonitorsContractError::Unauthorized, res);

                Ok(())
            }

            #[test]
            fn cannot_be_done_by_revoked_orchestrator() -> anyhow::Result<()> {
                let storage = NetworkMonitorsStorage::new();
                let (mut tester, orchestrator) = setup_prepopulated_tester();

                let admin = tester.admin_unchecked();
                let pre_all_agents = tester.all_agents();

                let deps = tester.deps_mut();

                // Revoke orchestrator privileges
                storage.remove_orchestrator_authorisation(deps, &admin, orchestrator.clone())?;

                // Verify revoked orchestrator cannot call remove_all_monitors
                let deps = tester.deps_mut();
                let res = storage
                    .remove_all_monitors(deps, &orchestrator)
                    .unwrap_err();
                assert_eq!(NetworkMonitorsContractError::Unauthorized, res);

                // Verify agent is still present after failed attempt
                assert_eq!(tester.all_agents(), pre_all_agents);

                Ok(())
            }

            #[test]
            fn clears_all_agents() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let orchestrator = tester.add_orchestrator()?;

                // Prepopulate with multiple agents
                let agent1 = tester.random_socket();
                let agent2 = tester.random_socket();
                let agent3 = tester.random_socket();
                let agent4 = tester.random_socket();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent1,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent2,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent3,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator,
                    agent4,
                    "test_noise_key".to_string(),
                    1,
                )?;

                // Verify agents are present
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent1.into())?
                    .is_some());
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent2.into())?
                    .is_some());
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent3.into())?
                    .is_some());
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent4.into())?
                    .is_some());

                // Remove all monitors
                let deps = tester.deps_mut();
                storage.remove_all_monitors(deps, &admin)?;

                // Verify all agents are cleared
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent1.into())?
                    .is_none());
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent2.into())?
                    .is_none());
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent3.into())?
                    .is_none());
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent4.into())?
                    .is_none());

                Ok(())
            }
        }
    }
}
