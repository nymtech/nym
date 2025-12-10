// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{MeasurementKind, NYM_PERFORMANCE_CONTRACT_STORAGE};
use cosmwasm_std::{Addr, DepsMut, Env, Event, MessageInfo, Response, to_json_binary};
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

pub fn try_define_measurement_kind(
    deps: DepsMut<'_>,
    sender: &Addr,
    measurement_kind: MeasurementKind,
) -> Result<Response, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), sender)?;

    // validation
    if measurement_kind.len() < 2
        || measurement_kind.len() > 20
        || !measurement_kind.is_ascii()
        || measurement_kind.contains(char::is_whitespace)
    {
        return Err(NymPerformanceContractError::InvalidInput(format!(
            "Cannot define {} as measurement kind",
            measurement_kind
        )));
    }

    NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .define_new_measurement_kind(deps.storage, measurement_kind)?;

    Ok(Response::new())
}

pub fn try_retire_measurement_kind(
    deps: DepsMut<'_>,
    sender_addr: &Addr,
    measurement_kind: MeasurementKind,
) -> Result<Response, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), sender_addr)?;

    NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .retire_measurement_kind(deps.storage, measurement_kind)?;

    Ok(Response::new())
}

pub fn try_submit_performance_results(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    epoch_id: EpochId,
    data: NodePerformance,
) -> Result<Response, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE.submit_performance_data(
        deps,
        env,
        &info.sender,
        epoch_id,
        data,
    )?;

    // TODO: emit events
    Ok(Response::new())
}

pub fn try_batch_submit_performance_results(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    epoch_id: EpochId,
    data: Vec<NodePerformance>,
) -> Result<Response, NymPerformanceContractError> {
    let res = NYM_PERFORMANCE_CONTRACT_STORAGE.batch_submit_performance_results(
        deps,
        env,
        &info.sender,
        epoch_id,
        data,
    )?;

    let response = Response::new().set_data(to_json_binary(&res)?).add_event(
        Event::new("batch_performance_submission")
            .add_attribute("accepted_scores", res.accepted_scores.to_string())
            .add_attribute(
                "non_existent_nodes",
                format!("{:?}", res.non_existent_nodes),
            )
            .add_attribute(
                "non_existent_measurement_kinds",
                format!("{:?}", res.non_existent_measurement_kind),
            ),
    );
    Ok(response)
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
    use crate::testing::{PerformanceContractTesterExt, init_contract_tester};
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

            assert!(
                try_authorise_network_monitor(
                    test.deps_mut(),
                    env.clone(),
                    admin.clone(),
                    bad_address
                )
                .is_err()
            );
            assert!(
                try_authorise_network_monitor(
                    test.deps_mut(),
                    env,
                    admin,
                    good_address.to_string()
                )
                .is_ok()
            );

            Ok(())
        }
    }

    #[cfg(test)]
    mod retiring_network_monitor {
        use super::*;
        use crate::testing::{PerformanceContractTesterExt, init_contract_tester};
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};

        #[test]
        fn requires_valid_address() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let bad_address = "foomp".to_string();
            let good_address = test.generate_account();
            test.authorise_network_monitor(&good_address)?;

            let env = test.env();
            let admin = test.admin_msg();

            assert!(
                try_retire_network_monitor(
                    test.deps_mut(),
                    env.clone(),
                    admin.clone(),
                    bad_address
                )
                .is_err()
            );
            assert!(
                try_retire_network_monitor(test.deps_mut(), env, admin, good_address.to_string())
                    .is_ok()
            );

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
        tester.define_dummy_measurement_kind()?;

        tester.advance_mixnet_epoch()?;

        let measurement_kind = tester.dummy_measurement_kind();
        for _ in 0..2 * retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT {
            let node_id = tester.bond_dummy_nymnode()?;
            tester.insert_raw_performance(&nm, node_id, measurement_kind.clone(), "0.42")?;
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

    mod measurement_kind_authorization {
        use cosmwasm_std::testing::message_info;
        use nym_contracts_common_testing::{AdminExt, ContractOpts};
        use nym_performance_contract_common::NymPerformanceContractError;

        use crate::{
            storage::MeasurementKind,
            testing::{PerformanceContractTesterExt, init_contract_tester},
            transactions::{
                try_define_measurement_kind, try_retire_measurement_kind,
                try_submit_performance_results,
            },
        };

        #[allow(clippy::panic)]
        #[test]
        fn add_requires_admin() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();
            let new_measurement = MeasurementKind::from("new-measurement");

            assert!(
                try_define_measurement_kind(
                    tester.deps_mut(),
                    &admin.sender,
                    new_measurement.clone()
                )
                .is_ok()
            );
        }

        #[allow(clippy::panic)]
        #[test]
        fn retire_requires_admin() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();
            let new_measurement = MeasurementKind::from("new-measurement");

            try_define_measurement_kind(tester.deps_mut(), &admin.sender, new_measurement.clone())
                .unwrap();

            let unauthorized_addr = tester.addr_make("unauthorized-addr");
            let unauthorized = try_retire_measurement_kind(
                tester.deps_mut(),
                &unauthorized_addr,
                new_measurement.clone(),
            );
            assert!(matches!(
                unauthorized,
                Err(NymPerformanceContractError::Admin { .. })
            ));

            let authorized = try_retire_measurement_kind(
                tester.deps_mut(),
                &admin.sender,
                new_measurement.clone(),
            );
            assert!(matches!(authorized, Ok(..)));
        }

        #[allow(clippy::panic)]
        #[test]
        fn cannot_add_existing() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();
            let new_measurement = MeasurementKind::from("new-measurement");

            let first_attempt = try_define_measurement_kind(
                tester.deps_mut(),
                &admin.sender,
                new_measurement.clone(),
            );
            assert!(matches!(first_attempt, Ok(..)));

            let second_attempt =
                try_define_measurement_kind(tester.deps_mut(), &admin.sender, new_measurement);
            assert!(matches!(
                second_attempt,
                Err(NymPerformanceContractError::InvalidInput(_))
            ));
        }

        #[allow(clippy::panic)]
        #[test]
        fn cannot_retire_nonexistent() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();
            let nonexistent = MeasurementKind::from("nonexistent");

            let err = try_retire_measurement_kind(tester.deps_mut(), &admin.sender, nonexistent);

            assert!(matches!(
                err,
                Err(NymPerformanceContractError::InvalidInput(_))
            ));
        }

        #[allow(clippy::panic)]
        #[test]
        fn cannot_submit_undefined() {
            let mut tester = init_contract_tester();
            let env = tester.env();
            let admin = tester.admin_msg();
            let dummy_perf = tester.dummy_node_performance();
            let nm = tester.addr_make("network-monitor");
            tester.authorise_network_monitor(&nm).unwrap();

            let dummy_measurement = dummy_perf.measurement_kind.clone();

            let first_attempt = try_submit_performance_results(
                tester.deps_mut(),
                env.clone(),
                // network monitor submits
                message_info(&nm, &[]),
                0,
                dummy_perf.clone(),
            );
            assert!(matches!(
                first_attempt,
                Err(NymPerformanceContractError::UnsupportedMeasurementKind { .. })
            ));

            try_define_measurement_kind(
                tester.deps_mut(),
                // admin defines
                &admin.sender,
                dummy_measurement.clone(),
            )
            .unwrap();
            let second_attempt = try_submit_performance_results(
                tester.deps_mut(),
                env,
                // network monitor submits
                message_info(&nm, &[]),
                0,
                dummy_perf,
            );
            assert!(matches!(second_attempt, Ok(..)));
        }

        #[allow(clippy::panic)]
        #[test]
        fn cannot_submit_retired() {
            let mut tester = init_contract_tester();
            let env = tester.env();
            let admin = tester.admin_msg();
            let dummy_perf = tester.dummy_node_performance();
            let nm = tester.addr_make("network-monitor");
            tester.authorise_network_monitor(&nm).unwrap();

            let dummy_measurement = dummy_perf.measurement_kind.clone();

            try_define_measurement_kind(
                tester.deps_mut(),
                // admin defines
                &admin.sender,
                dummy_measurement.clone(),
            )
            .unwrap();
            let defined_ok = try_submit_performance_results(
                tester.deps_mut(),
                env.clone(),
                // network monitor submits
                message_info(&nm, &[]),
                0,
                dummy_perf.clone(),
            );
            assert!(matches!(defined_ok, Ok(..)));

            // can't submit for the same node in the same epoch again
            tester.advance_mixnet_epoch().unwrap();

            try_retire_measurement_kind(
                tester.deps_mut(),
                // admin defines
                &admin.sender,
                dummy_measurement.clone(),
            )
            .unwrap();

            let retired_err = try_submit_performance_results(
                tester.deps_mut(),
                env,
                // network monitor submits
                message_info(&nm, &[]),
                1,
                dummy_perf,
            );
            println!("{:#?}", retired_err);
            assert!(matches!(
                retired_err,
                Err(NymPerformanceContractError::UnsupportedMeasurementKind { .. })
            ));
        }
    }
}
