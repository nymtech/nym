// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use cosmwasm_std::{Addr, Order};
use nym_contracts_common_testing::{
    mock_dependencies, AdminExt, ChainOpts, CommonStorageKeys, ContractFn, ContractOpts,
    ContractTester, DenomExt, PermissionedFn, QueryFn, RandExt, Rng, RngCore, TestableNymContract,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::storage::NetworkMonitorsStorage;
use nym_network_monitors_contract_common::constants::storage_keys;
use nym_network_monitors_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NetworkMonitorsContractError, QueryMsg,
};

pub struct NetworkMonitorsContract;

impl TestableNymContract for NetworkMonitorsContract {
    const NAME: &'static str = "nym-network-monitors-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = NetworkMonitorsContractError;

    fn instantiate() -> ContractFn<Self::InitMsg, Self::ContractError> {
        instantiate
    }

    fn execute() -> ContractFn<Self::ExecuteMsg, Self::ContractError> {
        execute
    }

    fn query() -> QueryFn<Self::QueryMsg, Self::ContractError> {
        query
    }

    fn migrate() -> PermissionedFn<Self::MigrateMsg, Self::ContractError> {
        migrate
    }

    fn base_init_msg() -> Self::InitMsg {
        let deps = mock_dependencies();
        InstantiateMsg {
            orchestrator_address: deps.api.addr_make("initial-dummy-orchestrator").to_string(),
        }
    }
}

pub fn init_contract_tester() -> ContractTester<NetworkMonitorsContract> {
    NetworkMonitorsContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN)
}

pub trait NetworkMonitorsContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = NetworkMonitorsContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
{
    fn add_orchestrator(&mut self) -> Result<Addr, NetworkMonitorsContractError> {
        let admin = self.admin_unchecked();
        let addr = self.generate_account();
        self.execute_raw(
            admin,
            ExecuteMsg::AuthoriseNetworkMonitorOrchestrator {
                address: addr.to_string(),
            },
        )?;
        Ok(addr)
    }

    fn remove_all_orchestrators(&mut self) {
        let orchestrators = self.all_orchestrators();
        for orchestrator in orchestrators {
            self.execute_raw(
                self.admin_unchecked(),
                ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            )
            .unwrap();
        }
    }

    fn add_dummy_agent(&mut self, agent: SocketAddr) {
        let orchestrators = self.all_orchestrators();
        let orchestrator = match orchestrators.first() {
            Some(orchestrator) => orchestrator.clone(),
            None => self.add_orchestrator().unwrap().clone(),
        };

        self.execute_raw(
            orchestrator,
            ExecuteMsg::AuthoriseNetworkMonitor {
                mixnet_address: agent,
                bs58_x25519_noise: "11111111111111111111111111111111".to_string(),
                noise_version: 1,
            },
        )
        .unwrap();
    }

    fn random_ipv4(&mut self) -> IpAddr {
        let rng = self.raw_rng();
        IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
    }

    fn random_ipv6(&mut self) -> IpAddr {
        let rng = self.raw_rng();
        IpAddr::V6(Ipv6Addr::new(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ))
    }

    fn random_ip(&mut self) -> IpAddr {
        let rng = self.raw_rng();

        // toss a coin, if even => ipv4, if odd => ipv6
        if rng.next_u32() % 2 == 0 {
            self.random_ipv4()
        } else {
            self.random_ipv6()
        }
    }

    fn random_socket_ipv4(&mut self) -> SocketAddr {
        let port = self.raw_rng().gen();
        SocketAddr::new(self.random_ipv4(), port)
    }

    fn random_socket_ipv6(&mut self) -> SocketAddr {
        let port = self.raw_rng().gen();
        SocketAddr::new(self.random_ipv6(), port)
    }

    fn random_socket(&mut self) -> SocketAddr {
        let port = self.raw_rng().gen();
        SocketAddr::new(self.random_ip(), port)
    }

    fn all_agents(&self) -> Vec<SocketAddr> {
        NetworkMonitorsStorage::new()
            .authorised_agents
            .range(self.storage(), None, None, Order::Ascending)
            .map(|record| record.unwrap().1.mixnet_address)
            .collect()
    }

    fn all_orchestrators(&self) -> Vec<Addr> {
        NetworkMonitorsStorage::new()
            .authorised_orchestrators
            .range(self.storage(), None, None, Order::Ascending)
            .map(|record| record.unwrap().0)
            .collect()
    }
}

impl NetworkMonitorsContractTesterExt for ContractTester<NetworkMonitorsContract> {}

/// Compare SocketAddrs in the same order as the storage key encoding.
///
/// Storage keys are: `[0, ip_len] [ip_octets...] [port_be_bytes]`
/// This means IPv4 (len=4) always sorts before IPv6 (len=16),
/// within the same type keys sort by IP octets then by port.
pub(crate) fn storage_socket_comp(a: SocketAddr, b: SocketAddr) -> std::cmp::Ordering {
    let ip_ord = match (a.ip(), b.ip()) {
        (IpAddr::V4(a), IpAddr::V4(b)) => a.octets().cmp(&b.octets()),
        (IpAddr::V6(a), IpAddr::V6(b)) => a.octets().cmp(&b.octets()),
        // length prefix [0, 4] < [0, 16] so all IPv4 sorts before all IPv6
        (IpAddr::V4(_), IpAddr::V6(_)) => std::cmp::Ordering::Less,
        (IpAddr::V6(_), IpAddr::V4(_)) => std::cmp::Ordering::Greater,
    };
    ip_ord.then(a.port().cmp(&b.port()))
}
