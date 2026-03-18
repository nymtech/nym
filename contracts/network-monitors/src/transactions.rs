// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NETWORK_MONITORS_CONTRACT_STORAGE;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use nym_network_monitors_contract_common::NetworkMonitorsContractError;
use std::net::IpAddr;

pub fn try_update_contract_admin(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, NetworkMonitorsContractError> {
    let new_admin = deps.api.addr_validate(&new_admin)?;

    let res = NETWORK_MONITORS_CONTRACT_STORAGE
        .contract_admin
        .execute_update_admin(deps, info, Some(new_admin))?;

    Ok(res)
}

pub fn try_authorise_network_monitor_orchestrator(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    orchestrator_address: String,
) -> Result<Response, NetworkMonitorsContractError> {
    let orchestrator_address = deps.api.addr_validate(&orchestrator_address)?;
    NETWORK_MONITORS_CONTRACT_STORAGE.authorise_orchestrator(
        deps,
        &env,
        &info.sender,
        orchestrator_address,
    )?;

    Ok(Response::new())
}

pub fn try_revoke_network_monitor_orchestrator(
    deps: DepsMut<'_>,
    info: MessageInfo,
    orchestrator_address: String,
) -> Result<Response, NetworkMonitorsContractError> {
    let orchestrator_address = deps.api.addr_validate(&orchestrator_address)?;

    NETWORK_MONITORS_CONTRACT_STORAGE.remove_orchestrator_authorisation(
        deps,
        &info.sender,
        orchestrator_address,
    )?;

    Ok(Response::new())
}

pub fn try_authorise_network_monitor(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    network_monitor_address: IpAddr,
    bs58_x25519_noise: String,
    noise_version: u8,
) -> Result<Response, NetworkMonitorsContractError> {
    NETWORK_MONITORS_CONTRACT_STORAGE.authorise_monitor(
        deps,
        &env,
        &info.sender,
        network_monitor_address,
        bs58_x25519_noise,
        noise_version,
    )?;

    Ok(Response::new())
}

pub fn try_revoke_network_monitor(
    deps: DepsMut<'_>,
    info: MessageInfo,
    network_monitor_address: IpAddr,
) -> Result<Response, NetworkMonitorsContractError> {
    NETWORK_MONITORS_CONTRACT_STORAGE.remove_monitor_authorisation(
        deps,
        &info.sender,
        network_monitor_address,
    )?;
    Ok(Response::new())
}

pub fn try_revoke_all_network_monitors(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, NetworkMonitorsContractError> {
    NETWORK_MONITORS_CONTRACT_STORAGE.remove_all_monitors(deps, &info.sender)?;
    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{init_contract_tester, NetworkMonitorsContractTesterExt};
    use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};
    use nym_network_monitors_contract_common::ExecuteMsg;

    #[cfg(test)]
    mod updating_contract_admin {
        use super::*;
        use crate::testing::init_contract_tester;
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};
        use nym_network_monitors_contract_common::ExecuteMsg;

        #[test]
        fn can_only_be_performed_by_current_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let random_acc = test.generate_account();
            let new_admin = test.generate_account();
            let res = test
                .execute_raw(
                    random_acc,
                    ExecuteMsg::UpdateAdmin {
                        admin: new_admin.to_string(),
                    },
                )
                .unwrap_err();

            assert_eq!(
                res,
                NetworkMonitorsContractError::Admin(AdminError::NotAdmin {})
            );

            let actual_admin = test.admin_unchecked();
            let res = test.execute_raw(
                actual_admin.clone(),
                ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                },
            );
            assert!(res.is_ok());

            let updated_admin = test.admin_unchecked();
            assert_eq!(new_admin, updated_admin);

            Ok(())
        }

        #[test]
        fn requires_providing_valid_address() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let bad_account = "definitely-not-valid-account";
            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::UpdateAdmin {
                    admin: bad_account.to_string(),
                },
            );

            assert!(res.is_err());

            let empty_account = "";
            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::UpdateAdmin {
                    admin: empty_account.to_string(),
                },
            );

            assert!(res.is_err());

            Ok(())
        }
    }

    #[cfg(test)]
    mod authorising_network_monitor_orchestrator {
        use super::*;
        use cw_controllers::AdminError;

        #[test]
        fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let non_admin = test.generate_account();
            let orchestrator = test.generate_account();

            let res = test
                .execute_raw(
                    non_admin.clone(),
                    ExecuteMsg::AuthoriseNetworkMonitorOrchestrator {
                        address: orchestrator.to_string(),
                    },
                )
                .unwrap_err();

            assert_eq!(
                res,
                NetworkMonitorsContractError::Admin(AdminError::NotAdmin {})
            );

            let admin = test.admin_unchecked();
            let res = test.execute_raw(
                admin,
                ExecuteMsg::AuthoriseNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn requires_providing_valid_orchestrator_address() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let bad_address = "definitely-not-valid-account";
            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::AuthoriseNetworkMonitorOrchestrator {
                    address: bad_address.to_string(),
                },
            );
            assert!(res.is_err());

            let good_address = test.generate_account();
            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::AuthoriseNetworkMonitorOrchestrator {
                    address: good_address.to_string(),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn inserts_new_entry_for_fresh_accounts() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.generate_account();

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .may_load(test.storage(), &orchestrator)?
                .is_none());

            test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::AuthoriseNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            )?;

            let info = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .load(test.storage(), &orchestrator)?;

            assert_eq!(info.address, orchestrator);

            Ok(())
        }

        #[test]
        fn is_noop_for_already_authorised_accounts() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.generate_account();
            let admin = test.admin_unchecked();

            test.execute_raw(
                admin.clone(),
                ExecuteMsg::AuthoriseNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            )?;

            let info = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .load(test.storage(), &orchestrator)?;

            test.execute_raw(
                admin,
                ExecuteMsg::AuthoriseNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            )?;

            let updated = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .load(test.storage(), &orchestrator)?;

            assert_eq!(info, updated);

            Ok(())
        }
    }

    #[cfg(test)]
    mod revoking_network_monitor_orchestrator {
        use super::*;
        use cw_controllers::AdminError;

        #[test]
        fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let orchestrator = test.add_orchestrator()?;
            let non_admin = test.generate_account();

            let res = test
                .execute_raw(
                    non_admin.clone(),
                    ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                        address: orchestrator.to_string(),
                    },
                )
                .unwrap_err();

            assert_eq!(
                res,
                NetworkMonitorsContractError::Admin(AdminError::NotAdmin {})
            );

            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn requires_providing_valid_orchestrator_address() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                    address: "definitely-not-valid-account".to_string(),
                },
            );
            assert!(res.is_err());

            let valid_but_missing = test.generate_account();
            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                    address: valid_but_missing.to_string(),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn deletes_entry_from_storage() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .may_load(test.storage(), &orchestrator)?
                .is_some());

            test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            )?;

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .may_load(test.storage(), &orchestrator)?
                .is_none());

            Ok(())
        }
    }

    #[cfg(test)]
    mod authorising_network_monitor {
        use super::*;
        use nym_contracts_common_testing::ChainOpts;

        #[test]
        fn can_only_be_performed_by_orchestrator() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let non_orchestrator = test.generate_account();
            let agent = test.random_ip();

            let res = test
                .execute_raw(
                    non_orchestrator.clone(),
                    ExecuteMsg::AuthoriseNetworkMonitor { address: agent, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
                )
                .unwrap_err();

            assert_eq!(
                res,
                NetworkMonitorsContractError::NotAnOrchestrator {
                    addr: non_orchestrator
                }
            );

            let orchestrator = test.add_orchestrator()?;
            let res = test.execute_raw(
                orchestrator,
                ExecuteMsg::AuthoriseNetworkMonitor { address: agent, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn inserts_new_entry_for_fresh_agents() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_ip();

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.to_string())?
                .is_none());

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor { address: agent, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
            )?;

            let info = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .load(test.storage(), agent.to_string())?;

            assert_eq!(info.address, agent);
            assert_eq!(info.authorised_by, orchestrator);

            Ok(())
        }

        #[test]
        fn renews_existing_agent_authorisation() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_ip();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor { address: agent, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
            )?;

            let initial = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .load(test.storage(), agent.to_string())?;

            test.advance_day_of_blocks();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor { address: agent, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
            )?;

            let updated = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .load(test.storage(), agent.to_string())?;

            assert_eq!(updated.address, agent);
            assert_eq!(updated.authorised_by, orchestrator);
            assert!(updated.authorised_at > initial.authorised_at);

            Ok(())
        }
    }

    #[cfg(test)]
    mod revoking_network_monitor {
        use super::*;

        #[test]
        fn can_only_be_performed_by_orchestrator() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let orchestrator = test.add_orchestrator()?;
            let non_orchestrator = test.generate_account();
            let agent = test.random_ip();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor { address: agent, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
            )?;

            let res = test
                .execute_raw(
                    non_orchestrator.clone(),
                    ExecuteMsg::RevokeNetworkMonitor { address: agent },
                )
                .unwrap_err();

            assert_eq!(
                res,
                NetworkMonitorsContractError::NotAnOrchestrator {
                    addr: non_orchestrator
                }
            );

            let res = test.execute_raw(
                orchestrator,
                ExecuteMsg::RevokeNetworkMonitor { address: agent },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn deletes_entry_from_storage() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_ip();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor { address: agent, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
            )?;

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.to_string())?
                .is_some());

            test.execute_raw(
                orchestrator,
                ExecuteMsg::RevokeNetworkMonitor { address: agent },
            )?;

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.to_string())?
                .is_none());

            Ok(())
        }

        #[test]
        fn is_noop_for_non_existent_entries() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_ip();

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.to_string())?
                .is_none());

            let res = test.execute_raw(
                orchestrator,
                ExecuteMsg::RevokeNetworkMonitor { address: agent },
            );
            assert!(res.is_ok());

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.to_string())?
                .is_none());

            Ok(())
        }
    }

    #[cfg(test)]
    mod revoking_all_network_monitors {
        use super::*;

        fn setup_prepopulated_tester() -> anyhow::Result<(
            nym_contracts_common_testing::ContractTester<crate::testing::NetworkMonitorsContract>,
            cosmwasm_std::Addr,
        )> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;

            let agent1 = test.random_ip();
            let agent2 = test.random_ip();
            let agent3 = test.random_ip();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor { address: agent1, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
            )?;
            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor { address: agent2, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
            )?;
            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor { address: agent3, bs58_x25519_noise: "test_noise_key".to_string(), noise_version: 1 },
            )?;

            Ok((test, orchestrator))
        }

        #[test]
        fn can_be_performed_by_admin() -> anyhow::Result<()> {
            let (mut test, _) = setup_prepopulated_tester()?;
            let agents = test.all_agents();

            test.execute_raw(test.admin_unchecked(), ExecuteMsg::RevokeAllNetworkMonitors)?;

            for agent in agents {
                assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                    .authorised_agents
                    .may_load(test.storage(), agent.to_string())?
                    .is_none());
            }

            assert!(test.all_agents().is_empty());

            Ok(())
        }

        #[test]
        fn can_be_performed_by_orchestrator() -> anyhow::Result<()> {
            let (mut test, orchestrator) = setup_prepopulated_tester()?;
            let agents = test.all_agents();

            test.execute_raw(orchestrator, ExecuteMsg::RevokeAllNetworkMonitors)?;

            for agent in agents {
                assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                    .authorised_agents
                    .may_load(test.storage(), agent.to_string())?
                    .is_none());
            }

            assert!(test.all_agents().is_empty());

            Ok(())
        }

        #[test]
        fn cannot_be_performed_by_non_privileged_account() -> anyhow::Result<()> {
            let (mut test, _) = setup_prepopulated_tester()?;
            let agents = test.all_agents();
            let random_acc = test.generate_account();

            let res = test
                .execute_raw(random_acc, ExecuteMsg::RevokeAllNetworkMonitors)
                .unwrap_err();

            assert_eq!(res, NetworkMonitorsContractError::Unauthorized);
            assert_eq!(test.all_agents(), agents);

            Ok(())
        }

        #[test]
        fn cannot_be_performed_by_revoked_orchestrator() -> anyhow::Result<()> {
            let (mut test, orchestrator) = setup_prepopulated_tester()?;
            let agents = test.all_agents();

            test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            )?;

            let res = test
                .execute_raw(orchestrator, ExecuteMsg::RevokeAllNetworkMonitors)
                .unwrap_err();

            assert_eq!(res, NetworkMonitorsContractError::Unauthorized);
            assert_eq!(test.all_agents(), agents);

            Ok(())
        }

        #[test]
        fn clears_all_agents() -> anyhow::Result<()> {
            let (mut test, _) = setup_prepopulated_tester()?;
            let agents = test.all_agents();

            test.execute_raw(test.admin_unchecked(), ExecuteMsg::RevokeAllNetworkMonitors)?;

            for agent in agents {
                assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                    .authorised_agents
                    .may_load(test.storage(), agent.to_string())?
                    .is_none());
            }

            assert!(test.all_agents().is_empty());

            Ok(())
        }
    }
}
