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
) -> Result<Response, NetworkMonitorsContractError> {
    NETWORK_MONITORS_CONTRACT_STORAGE.authorise_monitor(
        deps,
        &env,
        &info.sender,
        network_monitor_address,
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
}
