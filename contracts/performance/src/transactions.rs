// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use nym_performance_contract_common::{EpochId, NodePerformance, NymPerformanceContractError};

pub fn try_update_contract_admin(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, NymPerformanceContractError> {
    let new_admin = deps.api.addr_validate(&new_admin)?;

    let res = NYM_PERFORMANCE_CONTRACT_STORAGE
        .contract_admin
        .execute_update_admin(deps, info, Some(new_admin))?;

    Ok(res)
}

pub fn try_submit_performance_results(
    deps: DepsMut<'_>,
    info: MessageInfo,
    epoch_id: EpochId,
    data: NodePerformance,
) -> Result<Response, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE.submit_performance_data(deps, &info.sender, epoch_id, data)?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_batch_submit_performance_results(
    deps: DepsMut<'_>,
    info: MessageInfo,
    epoch_id: EpochId,
    data: Vec<NodePerformance>,
) -> Result<Response, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE.batch_submit_performance_results(
        deps,
        &info.sender,
        epoch_id,
        data,
    )?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_authorise_network_monitor(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, NymPerformanceContractError> {
    let address = deps.api.addr_validate(&address)?;

    NYM_PERFORMANCE_CONTRACT_STORAGE.authorise_network_monitor(
        deps,
        &env,
        &info.sender,
        address,
    )?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_retire_network_monitor(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, NymPerformanceContractError> {
    let address = deps.api.addr_validate(&address)?;

    NYM_PERFORMANCE_CONTRACT_STORAGE.retire_network_monitor(deps, env, &info.sender, address)?;

    // TODO: emit events
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
        use nym_performance_contract_common::ExecuteMsg;

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
                NymPerformanceContractError::Admin(AdminError::NotAdmin {})
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
