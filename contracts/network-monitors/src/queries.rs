// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{retrieval_limits, NETWORK_MONITORS_CONTRACT_STORAGE};
use cosmwasm_std::{Deps, StdResult};
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_network_monitors_contract_common::{
    AuthorisedNetworkMonitorOrchestratorsResponse, AuthorisedNetworkMonitorsPagedResponse,
    NetworkMonitorsContractError,
};
use std::net::IpAddr;

pub fn query_admin(deps: Deps) -> Result<AdminResponse, NetworkMonitorsContractError> {
    NETWORK_MONITORS_CONTRACT_STORAGE
        .contract_admin
        .query_admin(deps)
        .map_err(Into::into)
}

// no need for pagination as we don't expect even a double digit of those
pub fn query_network_monitor_orchestrators(
    deps: Deps,
) -> Result<AuthorisedNetworkMonitorOrchestratorsResponse, NetworkMonitorsContractError> {
    let authorised = NETWORK_MONITORS_CONTRACT_STORAGE
        .authorised_orchestrators
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|record| record.map(|(_, details)| details))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AuthorisedNetworkMonitorOrchestratorsResponse { authorised })
}

pub fn query_network_monitor_agents(
    deps: Deps,
    start_after: Option<IpAddr>,
    limit: Option<u32>,
) -> Result<AuthorisedNetworkMonitorsPagedResponse, NetworkMonitorsContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::AGENTS_DEFAULT_LIMIT)
        .min(retrieval_limits::AGENTS_MAX_LIMIT) as usize;

    let start = start_after.map(|addr| Bound::exclusive(addr.to_string()));

    let authorised = NETWORK_MONITORS_CONTRACT_STORAGE
        .authorised_agents
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|record| record.map(|(_, details)| details))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = authorised.last().map(|last| last.address);

    Ok(AuthorisedNetworkMonitorsPagedResponse {
        authorised,
        start_next_after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod admin_query {
        use crate::queries::query_admin;
        use crate::testing::init_contract_tester;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
        use nym_network_monitors_contract_common::ExecuteMsg;

        #[test]
        fn returns_current_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let initial_admin = test.admin_unchecked();

            // initial
            let res = query_admin(test.deps())?;
            assert_eq!(res.admin, Some(initial_admin.to_string()));

            let new_admin = test.generate_account();

            // sanity check
            assert_ne!(initial_admin, new_admin);

            // after update
            test.execute_msg(
                initial_admin.clone(),
                &ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                },
            )?;

            let updated_admin = query_admin(test.deps())?;
            assert_eq!(updated_admin.admin, Some(new_admin.to_string()));

            Ok(())
        }
    }

    #[cfg(test)]
    mod network_monitor_orchestrators_query {
        use super::*;
        use crate::testing::{init_contract_tester, NetworkMonitorsContractTesterExt};
        use nym_contracts_common_testing::{AdminExt, ContractOpts};
        use nym_network_monitors_contract_common::ExecuteMsg;

        #[test]
        fn returns_empty_list_when_there_are_no_extra_orchestrators() -> anyhow::Result<()> {
            // make sure to start with an empty state
            let mut test = init_contract_tester();
            test.remove_all_orchestrators();

            let res = query_network_monitor_orchestrators(test.deps())?;

            assert!(res.authorised.is_empty());

            Ok(())
        }

        #[test]
        fn returns_all_authorised_orchestrators() -> anyhow::Result<()> {
            // make sure to start with an empty state
            let mut test = init_contract_tester();
            test.remove_all_orchestrators();

            let orchestrator1 = test.add_orchestrator()?;
            let orchestrator2 = test.add_orchestrator()?;
            let orchestrator3 = test.add_orchestrator()?;

            let res = query_network_monitor_orchestrators(test.deps())?;

            assert_eq!(res.authorised.len(), 3);
            assert!(res.authorised.iter().any(|o| o.address == orchestrator1));
            assert!(res.authorised.iter().any(|o| o.address == orchestrator2));
            assert!(res.authorised.iter().any(|o| o.address == orchestrator3));

            Ok(())
        }

        #[test]
        fn does_not_return_revoked_orchestrators() -> anyhow::Result<()> {
            // make sure to start with an empty state
            let mut test = init_contract_tester();
            test.remove_all_orchestrators();

            let orchestrator1 = test.add_orchestrator()?;
            let orchestrator2 = test.add_orchestrator()?;

            test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                    address: orchestrator1.to_string(),
                },
            )?;

            let res = query_network_monitor_orchestrators(test.deps())?;

            assert!(!res.authorised.iter().any(|o| o.address == orchestrator1));
            assert!(res.authorised.iter().any(|o| o.address == orchestrator2));

            Ok(())
        }

        #[test]
        fn returns_entries_in_ascending_order() -> anyhow::Result<()> {
            // make sure to start with an empty state
            let mut test = init_contract_tester();
            test.remove_all_orchestrators();

            test.add_orchestrator()?;
            test.add_orchestrator()?;
            test.add_orchestrator()?;

            let res = query_network_monitor_orchestrators(test.deps())?;

            assert!(res
                .authorised
                .windows(2)
                .all(|window| window[0].address <= window[1].address));

            Ok(())
        }
    }

    #[cfg(test)]
    mod network_monitor_agents_query {
        use super::*;
        use crate::testing::{
            init_contract_tester, NetworkMonitorsContract, NetworkMonitorsContractTesterExt,
        };
        use nym_contracts_common_testing::{ContractOpts, ContractTester};
        use nym_network_monitors_contract_common::ExecuteMsg;

        fn string_sorted_ips(
            test: &mut ContractTester<NetworkMonitorsContract>,
            n: usize,
        ) -> Vec<IpAddr> {
            let mut ips = Vec::new();
            for _ in 0..n {
                ips.push(test.random_ip().to_string());
            }

            ips.sort_unstable();
            ips.into_iter().map(|ip| ip.parse().unwrap()).collect()
        }

        #[test]
        fn returns_empty_response_when_no_agents_are_authorised() -> anyhow::Result<()> {
            let test = init_contract_tester();

            let res = query_network_monitor_agents(test.deps(), None, None)?;

            assert!(res.authorised.is_empty());
            assert_eq!(res.start_next_after, None);

            Ok(())
        }

        #[test]
        fn returns_all_authorised_agents_below_default_limit() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agents = string_sorted_ips(&mut test, 5);

            for agent in &agents {
                test.execute_raw(
                    orchestrator.clone(),
                    ExecuteMsg::AuthoriseNetworkMonitor { address: *agent },
                )?;
            }

            let res = query_network_monitor_agents(test.deps(), None, None)?;

            assert_eq!(res.authorised.len(), agents.len());
            assert_eq!(res.start_next_after, agents.last().copied());

            for agent in &agents {
                assert!(res.authorised.iter().any(|a| a.address == *agent));
            }

            Ok(())
        }

        #[test]
        fn respects_explicit_limit() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agents = string_sorted_ips(&mut test, 5);

            for agent in &agents {
                test.execute_raw(
                    orchestrator.clone(),
                    ExecuteMsg::AuthoriseNetworkMonitor { address: *agent },
                )?;
            }

            let res = query_network_monitor_agents(test.deps(), None, Some(2))?;

            assert_eq!(res.authorised.len(), 2);
            assert_eq!(res.authorised[0].address, agents[0]);
            assert_eq!(res.authorised[1].address, agents[1]);
            assert_eq!(res.start_next_after, Some(agents[1]));

            Ok(())
        }

        #[test]
        fn respects_start_after_for_pagination() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agents = string_sorted_ips(&mut test, 5);

            for agent in &agents {
                test.execute_raw(
                    orchestrator.clone(),
                    ExecuteMsg::AuthoriseNetworkMonitor { address: *agent },
                )?;
            }

            let res = query_network_monitor_agents(test.deps(), Some(agents[1]), Some(2))?;

            assert_eq!(res.authorised.len(), 2);
            assert_eq!(res.authorised[0].address, agents[2]);
            assert_eq!(res.authorised[1].address, agents[3]);
            assert_eq!(res.start_next_after, Some(agents[3]));

            Ok(())
        }

        #[test]
        fn caps_limit_at_maximum() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let total = retrieval_limits::AGENTS_MAX_LIMIT as usize + 20;
            let agents = string_sorted_ips(&mut test, total);

            for agent in &agents {
                test.execute_raw(
                    orchestrator.clone(),
                    ExecuteMsg::AuthoriseNetworkMonitor { address: *agent },
                )?;
            }

            let res = query_network_monitor_agents(
                test.deps(),
                None,
                Some(retrieval_limits::AGENTS_MAX_LIMIT + 1),
            )?;

            assert_eq!(
                res.authorised.len(),
                retrieval_limits::AGENTS_MAX_LIMIT as usize
            );
            assert_eq!(
                res.start_next_after,
                Some(agents[retrieval_limits::AGENTS_MAX_LIMIT as usize - 1])
            );

            Ok(())
        }

        #[test]
        fn start_next_after_is_none_for_empty_page() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agents = string_sorted_ips(&mut test, 3);

            for agent in &agents {
                test.execute_raw(
                    orchestrator.clone(),
                    ExecuteMsg::AuthoriseNetworkMonitor { address: *agent },
                )?;
            }

            let res = query_network_monitor_agents(test.deps(), Some(agents[2]), Some(10))?;

            assert!(res.authorised.is_empty());
            assert_eq!(res.start_next_after, None);

            Ok(())
        }

        #[test]
        fn returns_entries_in_ascending_order() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agents = string_sorted_ips(&mut test, 6);

            for agent in &agents {
                test.execute_raw(
                    orchestrator.clone(),
                    ExecuteMsg::AuthoriseNetworkMonitor { address: *agent },
                )?;
            }

            let res = query_network_monitor_agents(test.deps(), None, None)?;

            assert!(res
                .authorised
                .windows(2)
                .all(|window| window[0].address.to_string() <= window[1].address.to_string()));

            Ok(())
        }
    }
}
