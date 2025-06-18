// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use cosmwasm_std::{to_json_binary, DepsMut, Env, MessageInfo, Response};
use nym_performance_contract_common::{
    EpochId, NodeId, NodePerformance, NymPerformanceContractError,
};

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

pub fn try_remove_node_measurements(
    deps: DepsMut<'_>,
    info: MessageInfo,
    epoch_id: EpochId,
    node_id: NodeId,
) -> Result<Response, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE.remove_node_measurements(
        deps,
        &info.sender,
        epoch_id,
        node_id,
    )?;

    Ok(Response::new())
}

pub fn try_remove_epoch_measurements(
    deps: DepsMut<'_>,
    info: MessageInfo,
    epoch_id: EpochId,
) -> Result<Response, NymPerformanceContractError> {
    let res =
        NYM_PERFORMANCE_CONTRACT_STORAGE.remove_epoch_measurements(deps, &info.sender, epoch_id)?;

    Ok(Response::new().set_data(to_json_binary(&res)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::retrieval_limits;
    use crate::testing::{init_contract_tester, PerformanceContractTesterExt};
    use cosmwasm_std::from_json;
    use nym_contracts_common_testing::{AdminExt, ContractOpts};
    use nym_performance_contract_common::RemoveEpochMeasurementsResponse;

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

    #[cfg(test)]
    mod authorising_network_monitor {
        use super::*;
        use crate::testing::init_contract_tester;
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};

        #[test]
        fn requires_valid_address() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let bad_address = "foomp".to_string();
            let good_address = test.generate_account();

            let env = test.env();
            let admin = test.admin_msg();

            assert!(try_authorise_network_monitor(
                test.deps_mut(),
                env.clone(),
                admin.clone(),
                bad_address
            )
            .is_err());
            assert!(try_authorise_network_monitor(
                test.deps_mut(),
                env,
                admin,
                good_address.to_string()
            )
            .is_ok());

            Ok(())
        }
    }

    #[cfg(test)]
    mod retiring_network_monitor {
        use super::*;
        use crate::testing::{init_contract_tester, PerformanceContractTesterExt};
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};

        #[test]
        fn requires_valid_address() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let bad_address = "foomp".to_string();
            let good_address = test.generate_account();
            test.authorise_network_monitor(&good_address)?;

            let env = test.env();
            let admin = test.admin_msg();

            assert!(try_retire_network_monitor(
                test.deps_mut(),
                env.clone(),
                admin.clone(),
                bad_address
            )
            .is_err());
            assert!(try_retire_network_monitor(
                test.deps_mut(),
                env,
                admin,
                good_address.to_string()
            )
            .is_ok());

            Ok(())
        }
    }

    // panics in tests are fine...
    #[allow(clippy::panic)]
    #[test]
    fn removing_epoch_measurements_returns_binary_data() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();

        let nm = tester.addr_make("network-monitor");
        tester.authorise_network_monitor(&nm)?;

        tester.advance_mixnet_epoch()?;
        for i in 0..2 * retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT {
            tester.insert_raw_performance(&nm, (i + 1) as NodeId, "0.42")?;
        }

        let admin = tester.admin_msg();
        let res = try_remove_epoch_measurements(tester.deps_mut(), admin.clone(), 0)?;

        let Some(data) = res.data else {
            panic!("missing binary response");
        };
        let deserialised: RemoveEpochMeasurementsResponse = from_json(&data)?;
        assert!(!deserialised.additional_entries_to_remove_remaining);

        let res = try_remove_epoch_measurements(tester.deps_mut(), admin, 1)?;

        let Some(data) = res.data else {
            panic!("missing binary response");
        };
        let deserialised: RemoveEpochMeasurementsResponse = from_json(&data)?;
        assert!(deserialised.additional_entries_to_remove_remaining);

        Ok(())
    }
}
