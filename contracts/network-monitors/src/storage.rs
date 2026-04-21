// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
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

/// CosmWasm storage key encoding a [`SocketAddr`] as a composite primary key.
///
/// ## On-disk layout
///
/// The key is encoded as two elements (see [`PrimaryKey::key`]):
///   1. IP octets — `Val32` (4 bytes) for IPv4, `Val128` (16 bytes) for IPv6
///   2. Port — `Val16` (2 bytes, big-endian)
///
/// cw-storage-plus prepends each element with a 2-byte big-endian length prefix,
/// so the full byte sequence is `[0, ip_len][ip_octets...][0, 2][port_be]`.
/// This means IPv4 keys naturally sort before IPv6, and within the same IP
/// family keys sort by IP octets then by port.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AgentStorageKey(SocketAddr);

impl PrimaryKey<'_> for AgentStorageKey {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&'_ self) -> Vec<Key<'_>> {
        let port = self.0.port().to_be_bytes();
        match self.0.ip() {
            IpAddr::V4(ipv4) => vec![Key::Val32(ipv4.octets()), Key::Val16(port)],
            IpAddr::V6(ipv6) => vec![Key::Val128(ipv6.octets()), Key::Val16(port)],
        }
    }
}

impl KeyDeserialize for AgentStorageKey {
    type Output = AgentStorageKey;
    const KEY_ELEMS: u16 = 2;

    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        // format: 2-byte length prefix + IP bytes + 2-byte port
        if value.len() < 4 {
            return Err(StdError::generic_err("invalid socket address length"));
        }

        // SAFETY: we're using the correct number of bytes for the conversion
        #[allow(clippy::unwrap_used)]
        let ip_len = u16::from_be_bytes([value[0], value[1]]) as usize;
        let ip_bytes = &value[2..2 + ip_len];
        let port_bytes = &value[2 + ip_len..];

        #[allow(clippy::unwrap_used)]
        let ip = match ip_len {
            4 => IpAddr::V4(Ipv4Addr::from(
                TryInto::<[u8; 4]>::try_into(ip_bytes).unwrap(),
            )),
            16 => IpAddr::V6(Ipv6Addr::from(
                TryInto::<[u8; 16]>::try_into(ip_bytes).unwrap(),
            )),
            _ => return Err(StdError::generic_err("invalid IP address length")),
        };

        let port = u16::from_be_bytes(
            TryInto::<[u8; 2]>::try_into(port_bytes)
                .map_err(|_| StdError::generic_err("invalid port length"))?,
        );

        Ok(AgentStorageKey(SocketAddr::new(ip, port)))
    }
}

impl From<SocketAddr> for AgentStorageKey {
    fn from(addr: SocketAddr) -> Self {
        Self(addr)
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
                identity_key: None,
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
                identity_key: None,
                authorised_at: env.block.time,
            },
        )?;
        Ok(())
    }

    /// Overwrite the announced identity key for the orchestrator at `sender`.
    ///
    /// The orchestrator must already be authorised - the existence of the storage entry is used as
    /// the authorisation check itself, avoiding a second load. `identity_key` is stored verbatim
    /// (callers are expected to have validated its shape beforehand).
    pub fn update_orchestrator_identity_key(
        &self,
        deps: DepsMut,
        sender: &Addr,
        identity_key: String,
    ) -> Result<(), NetworkMonitorsContractError> {
        // ensure the sender is actually a valid orchestrator
        // by checking if there is any data stored behind its address
        let Some(mut orchestrator_info) = self
            .authorised_orchestrators
            .may_load(deps.storage, &sender)?
        else {
            return Err(NetworkMonitorsContractError::NotAnOrchestrator {
                addr: sender.clone(),
            });
        };

        orchestrator_info.identity_key = Some(identity_key);
        self.authorised_orchestrators
            .save(deps.storage, &sender, &orchestrator_info)?;

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

        // cascade-remove agents authorised by the removed orchestrator
        // TODO: optimise it in the future in case there are more agents than could be handled in a single block
        let agents_to_remove = self
            .authorised_agents
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .filter(|r| {
                r.as_ref()
                    .map(|(_, v)| v.authorised_by == orchestrator_address)
                    .unwrap_or(true)
            })
            .map(|r| r.map(|(k, _)| k))
            .collect::<cosmwasm_std::StdResult<Vec<_>>>()?;
        for agent in agents_to_remove {
            self.authorised_agents.remove(deps.storage, agent);
        }

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
        monitor_address: SocketAddr,
    ) -> Result<(), NetworkMonitorsContractError> {
        // the contract admin or an authorised orchestrator may revoke a monitor
        if !self.is_admin(deps.as_ref(), sender)? && !self.is_orchestrator(deps.as_ref(), sender)? {
            return Err(NetworkMonitorsContractError::Unauthorized);
        }

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
    mod agent_storage_key {
        use super::super::AgentStorageKey;
        use cw_storage_plus::{KeyDeserialize, Map, PrimaryKey};
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

        #[test]
        fn ipv4_key_roundtrips_through_joined_key_and_from_vec() {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)), 8080);
            let key = AgentStorageKey::from(addr);
            let joined = key.joined_key();
            let recovered = AgentStorageKey::from_vec(joined).unwrap();
            assert_eq!(recovered.0, addr);
        }

        #[test]
        fn ipv6_key_roundtrips_through_joined_key_and_from_vec() {
            let addr = SocketAddr::new(
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
                443,
            );
            let key = AgentStorageKey::from(addr);
            let joined = key.joined_key();
            let recovered = AgentStorageKey::from_vec(joined).unwrap();
            assert_eq!(recovered.0, addr);
        }

        #[test]
        fn port_zero_roundtrips() {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 0);
            let key = AgentStorageKey::from(addr);
            let joined = key.joined_key();
            let recovered = AgentStorageKey::from_vec(joined).unwrap();
            assert_eq!(recovered.0, addr);
        }

        #[test]
        fn port_max_roundtrips() {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), u16::MAX);
            let key = AgentStorageKey::from(addr);
            let joined = key.joined_key();
            let recovered = AgentStorageKey::from_vec(joined).unwrap();
            assert_eq!(recovered.0, addr);
        }

        #[test]
        fn same_ip_different_ports_produce_different_keys() {
            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
            let a = AgentStorageKey::from(SocketAddr::new(ip, 1000));
            let b = AgentStorageKey::from(SocketAddr::new(ip, 2000));
            assert_ne!(a.joined_key(), b.joined_key());
        }

        #[test]
        fn ipv4_keys_sort_by_ip_then_port() {
            let addrs = [
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 2000),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 1000),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 500),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 9999),
            ];

            let mut keys: Vec<_> = addrs
                .iter()
                .map(|a| (AgentStorageKey::from(*a).joined_key(), *a))
                .collect();
            keys.sort_by(|a, b| a.0.cmp(&b.0));

            let sorted_addrs: Vec<_> = keys.iter().map(|(_, a)| *a).collect();
            assert_eq!(
                sorted_addrs,
                vec![
                    // 1.2.3.4 < 10.0.0.1 < 10.0.0.2
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 9999),
                    // same IP, port 1000 < 2000
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 1000),
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 2000),
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 500),
                ]
            );
        }

        #[test]
        fn ipv4_sorts_before_ipv6() {
            let v4 = AgentStorageKey::from(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
                65535,
            ));
            let v6 = AgentStorageKey::from(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                1,
            ));
            assert!(v4.joined_key() < v6.joined_key());
        }

        #[test]
        fn map_save_and_load_roundtrip_ipv4() {
            use cosmwasm_std::testing::MockStorage;

            let map: Map<AgentStorageKey, String> = Map::new("test");
            let mut storage = MockStorage::new();

            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
            map.save(&mut storage, addr.into(), &"agent-v4".to_string())
                .unwrap();

            let loaded = map.load(&storage, addr.into()).unwrap();
            assert_eq!(loaded, "agent-v4");
        }

        #[test]
        fn map_save_and_load_roundtrip_ipv6() {
            use cosmwasm_std::testing::MockStorage;

            let map: Map<AgentStorageKey, String> = Map::new("test");
            let mut storage = MockStorage::new();

            let addr = SocketAddr::new(
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
                443,
            );
            map.save(&mut storage, addr.into(), &"agent-v6".to_string())
                .unwrap();

            let loaded = map.load(&storage, addr.into()).unwrap();
            assert_eq!(loaded, "agent-v6");
        }

        #[test]
        fn map_stores_same_ip_different_ports_independently() {
            use cosmwasm_std::testing::MockStorage;

            let map: Map<AgentStorageKey, String> = Map::new("test");
            let mut storage = MockStorage::new();

            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
            let agent_a = SocketAddr::new(ip, 1000);
            let agent_b = SocketAddr::new(ip, 2000);

            map.save(&mut storage, agent_a.into(), &"agent-a".to_string())
                .unwrap();
            map.save(&mut storage, agent_b.into(), &"agent-b".to_string())
                .unwrap();

            assert_eq!(map.load(&storage, agent_a.into()).unwrap(), "agent-a");
            assert_eq!(map.load(&storage, agent_b.into()).unwrap(), "agent-b");
        }

        #[test]
        fn map_remove_one_agent_preserves_other_on_same_ip() {
            use cosmwasm_std::testing::MockStorage;

            let map: Map<AgentStorageKey, String> = Map::new("test");
            let mut storage = MockStorage::new();

            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
            let agent_a = SocketAddr::new(ip, 1000);
            let agent_b = SocketAddr::new(ip, 2000);

            map.save(&mut storage, agent_a.into(), &"agent-a".to_string())
                .unwrap();
            map.save(&mut storage, agent_b.into(), &"agent-b".to_string())
                .unwrap();

            map.remove(&mut storage, agent_a.into());

            assert!(map.may_load(&storage, agent_a.into()).unwrap().is_none());
            assert_eq!(map.load(&storage, agent_b.into()).unwrap(), "agent-b");
        }

        #[test]
        fn map_range_returns_correct_order_with_mixed_ips_and_ports() {
            use cosmwasm_std::testing::MockStorage;
            use cosmwasm_std::{Order, StdResult};

            let map: Map<AgentStorageKey, String> = Map::new("test");
            let mut storage = MockStorage::new();

            let addrs = [
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 2000),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 1000),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 80),
                SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 443),
            ];

            for addr in &addrs {
                map.save(&mut storage, (*addr).into(), &addr.to_string())
                    .unwrap();
            }

            let all: Vec<SocketAddr> = map
                .range(&storage, None, None, Order::Ascending)
                .map(|r: StdResult<(AgentStorageKey, String)>| r.unwrap().0 .0)
                .collect();

            assert_eq!(
                all,
                vec![
                    // IPv4 sorted by octets, then port
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 80),
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 1000),
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 2000),
                    // IPv6 after all IPv4
                    SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 443),
                ]
            );
        }

        #[test]
        fn map_range_with_exclusive_bound_skips_exact_match() {
            use cosmwasm_std::testing::MockStorage;
            use cosmwasm_std::{Order, StdResult};
            use cw_storage_plus::Bound;

            let map: Map<AgentStorageKey, String> = Map::new("test");
            let mut storage = MockStorage::new();

            let addrs = [
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 1000),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 2000),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 3000),
            ];

            for addr in &addrs {
                map.save(&mut storage, (*addr).into(), &addr.to_string())
                    .unwrap();
            }

            // paginate after the second entry
            let start = Bound::exclusive(AgentStorageKey::from(addrs[1]));
            let page: Vec<SocketAddr> = map
                .range(&storage, Some(start), None, Order::Ascending)
                .map(|r: StdResult<(AgentStorageKey, String)>| r.unwrap().0 .0)
                .collect();

            assert_eq!(page, vec![addrs[2]]);
        }
    }

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
                    orchestrator.clone(),
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
            use crate::testing::{init_contract_tester, NetworkMonitorsContractTesterExt};
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

            #[test]
            fn removes_agents_authorised_by_the_removed_orchestrator() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let orchestrator = tester.add_orchestrator()?;

                let agent1 = tester.random_socket();
                let agent2 = tester.random_socket();

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

                // sanity: both agents present
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent1.into())?
                    .is_some());
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent2.into())?
                    .is_some());

                let deps = tester.deps_mut();
                storage.remove_orchestrator_authorisation(deps, &admin, orchestrator.clone())?;

                // orchestrator is gone
                assert!(storage
                    .authorised_orchestrators
                    .may_load(&tester, &orchestrator)?
                    .is_none());

                // its agents are cascade-removed
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent1.into())?
                    .is_none());
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent2.into())?
                    .is_none());

                Ok(())
            }

            #[test]
            fn does_not_remove_agents_authorised_by_other_orchestrators() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let orchestrator_a = tester.add_orchestrator()?;
                let orchestrator_b = tester.add_orchestrator()?;

                let agent_a = tester.random_socket();
                let agent_b = tester.random_socket();

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator_a,
                    agent_a,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let env = tester.env();
                let deps = tester.deps_mut();
                storage.authorise_monitor(
                    deps,
                    &env,
                    &orchestrator_b,
                    agent_b,
                    "test_noise_key".to_string(),
                    1,
                )?;

                let deps = tester.deps_mut();
                storage.remove_orchestrator_authorisation(deps, &admin, orchestrator_a.clone())?;

                // orchestrator_a's agent is gone
                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent_a.into())?
                    .is_none());

                // orchestrator_b's agent is untouched
                let remaining = storage.authorised_agents.load(&tester, agent_b.into())?;
                assert_eq!(remaining.mixnet_address, agent_b);
                assert_eq!(remaining.authorised_by, orchestrator_b);

                // orchestrator_b itself is untouched
                assert!(storage
                    .authorised_orchestrators
                    .may_load(&tester, &orchestrator_b)?
                    .is_some());

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
            use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
            use nym_network_monitors_contract_common::NetworkMonitorsContractError;

            #[test]
            fn rejects_non_privileged_accounts() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let orchestrator = tester.add_orchestrator()?;
                let non_privileged = tester.generate_account();
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
                    .remove_monitor_authorisation(deps, &non_privileged, agent)
                    .unwrap_err();
                assert_eq!(NetworkMonitorsContractError::Unauthorized, res);

                let deps = tester.deps_mut();
                let res2 = storage.remove_monitor_authorisation(deps, &orchestrator, agent);
                assert_eq!(res2, Ok(()));

                Ok(())
            }

            #[test]
            fn can_be_done_by_admin() -> anyhow::Result<()> {
                let mut tester = init_contract_tester();
                let storage = NetworkMonitorsStorage::new();

                let admin = tester.admin_unchecked();
                let orchestrator = tester.add_orchestrator()?;
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
                storage.remove_monitor_authorisation(deps, &admin, agent)?;

                assert!(storage
                    .authorised_agents
                    .may_load(&tester, agent.into())?
                    .is_none());

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
                storage.remove_monitor_authorisation(deps, &orchestrator, agent)?;

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
                storage.remove_monitor_authorisation(deps, &orchestrator, agent)?;

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
                let res = storage.remove_monitor_authorisation(deps, &orchestrator, agent);
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

                let deps = tester.deps_mut();

                // Revoke orchestrator privileges (cascade-removes its agents)
                storage.remove_orchestrator_authorisation(deps, &admin, orchestrator.clone())?;

                // snapshot the post-revocation agent set so we can assert the failed
                // remove_all_monitors call below does not further mutate storage
                let post_revoke_agents = tester.all_agents();

                // Verify revoked orchestrator cannot call remove_all_monitors
                let deps = tester.deps_mut();
                let res = storage
                    .remove_all_monitors(deps, &orchestrator)
                    .unwrap_err();
                assert_eq!(NetworkMonitorsContractError::Unauthorized, res);

                // Verify the failed attempt did not mutate the agent set
                assert_eq!(tester.all_agents(), post_revoke_agents);

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
