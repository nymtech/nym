// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NETWORK_MONITORS_CONTRACT_STORAGE;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use nym_network_monitors_contract_common::NetworkMonitorsContractError;
use std::net::SocketAddr;

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

/// Update the announced ed25519 identity key of the orchestrator submitting the transaction.
///
/// The sender must already be an authorised orchestrator - this is enforced by
/// [`NetworkMonitorsStorage::update_orchestrator_identity_key`] via the `NotAnOrchestrator` error
/// when no entry exists for the sender.
///
/// Only shape-level validation is performed on `identity_key` (valid base58 encoding a 32-byte
/// ed25519 public key). The key is not verified to be a valid curve point, as doing so on-chain
/// is disproportionately expensive relative to the downstream risk - a malformed key will simply
/// fail signature verification when used.
pub fn try_update_orchestrator_identity_key(
    deps: DepsMut<'_>,
    info: MessageInfo,
    identity_key: String,
) -> Result<Response, NetworkMonitorsContractError> {
    // perform basic validation of the key, i.e. is it valid base58 and is it 32 bytes (i.e. ed25519)?
    let mut public_key = [0u8; 32];
    let used = bs58::decode(&identity_key)
        .onto(&mut public_key)
        .map_err(|err| {
            NetworkMonitorsContractError::MalformedEd25519OrchestratorIdentityKey(err.to_string())
        })?;

    if used != 32 {
        return Err(
            NetworkMonitorsContractError::MalformedEd25519OrchestratorIdentityKey(
                "Too few bytes provided for the public key".into(),
            ),
        );
    }

    NETWORK_MONITORS_CONTRACT_STORAGE.update_orchestrator_identity_key(
        deps,
        &info.sender,
        identity_key,
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
    network_monitor_address: SocketAddr,
    bs58_x25519_noise: String,
    noise_version: u8,
) -> Result<Response, NetworkMonitorsContractError> {
    // perform basic validation of the key, i.e. is it valid base58 and is it 32 bytes (i.e. x25519)?
    let mut public_key = [0u8; 32];
    let used = bs58::decode(&bs58_x25519_noise)
        .onto(&mut public_key)
        .map_err(|err| {
            NetworkMonitorsContractError::MalformedX25519AgentNoiseKey(err.to_string())
        })?;

    if used != 32 {
        return Err(NetworkMonitorsContractError::MalformedX25519AgentNoiseKey(
            "Too few bytes provided for the public key".into(),
        ));
    }

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
    network_monitor_address: SocketAddr,
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

    // bs58 encoding of 32 zero bytes — a syntactically valid x25519 key for tests
    const TEST_NOISE_KEY: &str = "11111111111111111111111111111111";

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
    mod updating_orchestrator_identity_key {
        use super::*;

        /// Base58 encoding of 32 bytes - a valid ed25519 key shape.
        fn valid_identity_key() -> String {
            bs58::encode([7u8; 32]).into_string()
        }

        #[test]
        fn can_only_be_performed_by_authorised_orchestrator() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let non_orchestrator = test.generate_account();

            let res = test
                .execute_raw(
                    non_orchestrator.clone(),
                    ExecuteMsg::UpdateOrchestratorIdentityKey {
                        key: valid_identity_key(),
                    },
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
                ExecuteMsg::UpdateOrchestratorIdentityKey {
                    key: valid_identity_key(),
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn rejects_key_that_is_not_valid_base58() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;

            // '0', 'O', 'I', 'l' are not in the bitcoin alphabet used by bs58
            let res = test
                .execute_raw(
                    orchestrator,
                    ExecuteMsg::UpdateOrchestratorIdentityKey {
                        key: "not_valid_base58_0OIl".to_string(),
                    },
                )
                .unwrap_err();
            assert!(matches!(
                res,
                NetworkMonitorsContractError::MalformedEd25519OrchestratorIdentityKey(_)
            ));

            Ok(())
        }

        #[test]
        fn rejects_key_that_is_too_short() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;

            // 16 bytes, not 32
            let too_short = bs58::encode([1u8; 16]).into_string();
            let res = test
                .execute_raw(
                    orchestrator,
                    ExecuteMsg::UpdateOrchestratorIdentityKey { key: too_short },
                )
                .unwrap_err();
            assert!(matches!(
                res,
                NetworkMonitorsContractError::MalformedEd25519OrchestratorIdentityKey(_)
            ));

            Ok(())
        }

        #[test]
        fn rejects_key_that_is_too_long() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;

            // 33 bytes, not 32 - decoder should bail out because the destination buffer is too small
            let too_long = bs58::encode([1u8; 33]).into_string();
            let res = test
                .execute_raw(
                    orchestrator,
                    ExecuteMsg::UpdateOrchestratorIdentityKey { key: too_long },
                )
                .unwrap_err();
            assert!(matches!(
                res,
                NetworkMonitorsContractError::MalformedEd25519OrchestratorIdentityKey(_)
            ));

            Ok(())
        }

        #[test]
        fn stores_provided_key_against_orchestrator_entry() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;

            // freshly authorised orchestrator has no announced identity key
            let info = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .load(test.storage(), &orchestrator)?;
            assert!(info.identity_key.is_none());

            let key = valid_identity_key();
            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::UpdateOrchestratorIdentityKey { key: key.clone() },
            )?;

            let updated = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .load(test.storage(), &orchestrator)?;
            assert_eq!(updated.identity_key.as_deref(), Some(key.as_str()));

            Ok(())
        }

        #[test]
        fn overwrites_previously_announced_key() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;

            let first_key = bs58::encode([1u8; 32]).into_string();
            let second_key = bs58::encode([2u8; 32]).into_string();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::UpdateOrchestratorIdentityKey {
                    key: first_key.clone(),
                },
            )?;
            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::UpdateOrchestratorIdentityKey {
                    key: second_key.clone(),
                },
            )?;

            let info = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .load(test.storage(), &orchestrator)?;
            assert_eq!(info.identity_key.as_deref(), Some(second_key.as_str()));

            Ok(())
        }

        #[test]
        fn updating_one_orchestrator_key_does_not_affect_others() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator_a = test.add_orchestrator()?;
            let orchestrator_b = test.add_orchestrator()?;

            let key_a = bs58::encode([10u8; 32]).into_string();
            test.execute_raw(
                orchestrator_a.clone(),
                ExecuteMsg::UpdateOrchestratorIdentityKey { key: key_a.clone() },
            )?;

            let info_a = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .load(test.storage(), &orchestrator_a)?;
            let info_b = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_orchestrators
                .load(test.storage(), &orchestrator_b)?;

            assert_eq!(info_a.identity_key.as_deref(), Some(key_a.as_str()));
            assert!(info_b.identity_key.is_none());

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
            let agent = test.random_socket();

            let res = test
                .execute_raw(
                    non_orchestrator.clone(),
                    ExecuteMsg::AuthoriseNetworkMonitor {
                        mixnet_address: agent,
                        bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                        noise_version: 1,
                    },
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
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn inserts_new_entry_for_fresh_agents() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_socket();

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.into())?
                .is_none());

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            )?;

            let info = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .load(test.storage(), agent.into())?;

            assert_eq!(info.mixnet_address, agent);
            assert_eq!(info.authorised_by, orchestrator);

            Ok(())
        }

        #[test]
        fn renews_existing_agent_authorisation() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_socket();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            )?;

            let initial = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .load(test.storage(), agent.into())?;

            test.advance_day_of_blocks();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            )?;

            let updated = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .load(test.storage(), agent.into())?;

            assert_eq!(updated.mixnet_address, agent);
            assert_eq!(updated.authorised_by, orchestrator);
            assert!(updated.authorised_at > initial.authorised_at);

            Ok(())
        }
    }

    #[cfg(test)]
    mod revoking_network_monitor {
        use super::*;

        #[test]
        fn can_be_performed_by_orchestrator() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_socket();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            )?;

            let res = test.execute_raw(
                orchestrator,
                ExecuteMsg::RevokeNetworkMonitor { address: agent },
            );
            assert!(res.is_ok());

            Ok(())
        }

        #[test]
        fn can_be_performed_by_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let admin = test.admin_unchecked();
            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_socket();

            test.execute_raw(
                orchestrator,
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            )?;

            let res = test.execute_raw(admin, ExecuteMsg::RevokeNetworkMonitor { address: agent });
            assert!(res.is_ok());

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.into())?
                .is_none());

            Ok(())
        }

        #[test]
        fn rejects_non_privileged_accounts() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let orchestrator = test.add_orchestrator()?;
            let non_privileged = test.generate_account();
            let agent = test.random_socket();

            test.execute_raw(
                orchestrator,
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            )?;

            let res = test
                .execute_raw(
                    non_privileged,
                    ExecuteMsg::RevokeNetworkMonitor { address: agent },
                )
                .unwrap_err();

            assert_eq!(res, NetworkMonitorsContractError::Unauthorized);

            Ok(())
        }

        #[test]
        fn deletes_entry_from_storage() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_socket();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            )?;

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.into())?
                .is_some());

            test.execute_raw(
                orchestrator,
                ExecuteMsg::RevokeNetworkMonitor { address: agent },
            )?;

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.into())?
                .is_none());

            Ok(())
        }

        #[test]
        fn is_noop_for_non_existent_entries() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;
            let agent = test.random_socket();

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.into())?
                .is_none());

            let res = test.execute_raw(
                orchestrator,
                ExecuteMsg::RevokeNetworkMonitor { address: agent },
            );
            assert!(res.is_ok());

            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent.into())?
                .is_none());

            Ok(())
        }

        #[test]
        fn revoking_one_agent_preserves_other_on_same_host() -> anyhow::Result<()> {
            use std::net::{IpAddr, Ipv4Addr, SocketAddr};

            let mut test = init_contract_tester();
            let orchestrator = test.add_orchestrator()?;

            // two agents on the same IP but different ports
            let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
            let agent_a = SocketAddr::new(ip, 1000);
            let agent_b = SocketAddr::new(ip, 2000);

            // two syntactically valid (32-byte) bs58 x25519 keys
            let noise_key_a = TEST_NOISE_KEY.to_string();
            let noise_key_b = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi".to_string();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent_a,
                    bs58_x25519_noise: noise_key_a,
                    noise_version: 1,
                },
            )?;
            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent_b,
                    bs58_x25519_noise: noise_key_b.clone(),
                    noise_version: 1,
                },
            )?;

            // both exist
            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent_a.into())?
                .is_some());
            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent_b.into())?
                .is_some());

            // revoke agent_a
            test.execute_raw(
                orchestrator,
                ExecuteMsg::RevokeNetworkMonitor { address: agent_a },
            )?;

            // agent_a gone, agent_b still present
            assert!(NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .may_load(test.storage(), agent_a.into())?
                .is_none());
            let remaining = NETWORK_MONITORS_CONTRACT_STORAGE
                .authorised_agents
                .load(test.storage(), agent_b.into())?;
            assert_eq!(remaining.mixnet_address, agent_b);
            assert_eq!(
                remaining.bs58_x25519_noise,
                "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi"
            );

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

            let agent1 = test.random_socket();
            let agent2 = test.random_socket();
            let agent3 = test.random_socket();

            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent1,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            )?;
            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent2,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
            )?;
            test.execute_raw(
                orchestrator.clone(),
                ExecuteMsg::AuthoriseNetworkMonitor {
                    mixnet_address: agent3,
                    bs58_x25519_noise: TEST_NOISE_KEY.to_string(),
                    noise_version: 1,
                },
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
                    .may_load(test.storage(), agent.into())?
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
                    .may_load(test.storage(), agent.into())?
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

            test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            )?;

            // snapshot after revocation (cascade-delete has already run); the failed
            // call below must not mutate this set
            let post_revoke_agents = test.all_agents();

            let res = test
                .execute_raw(orchestrator, ExecuteMsg::RevokeAllNetworkMonitors)
                .unwrap_err();

            assert_eq!(res, NetworkMonitorsContractError::Unauthorized);
            assert_eq!(test.all_agents(), post_revoke_agents);

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
                    .may_load(test.storage(), agent.into())?
                    .is_none());
            }

            assert!(test.all_agents().is_empty());

            Ok(())
        }
    }
}
