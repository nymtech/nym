// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use cosmwasm_std::{to_json_binary, to_json_string, DepsMut, Env, Event, MessageInfo, Response};
use nym_performance_contract_common::{
    EpochId, NodeId, NodeSubmission, NymPerformanceContractError, Weights,
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
    env: Env,
    info: MessageInfo,
    epoch_id: EpochId,
    data: NodeSubmission,
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
    data: Vec<NodeSubmission>,
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
            ),
    );
    Ok(response)
}

pub fn try_update_weights(
    deps: DepsMut<'_>,
    info: MessageInfo,
    weights: Weights,
) -> Result<Response, NymPerformanceContractError> {
    let effective_from =
        NYM_PERFORMANCE_CONTRACT_STORAGE.update_weights(deps, &info.sender, weights)?;

    Ok(Response::new().add_event(
        Event::new("weights_update")
            .add_attribute("effective_from", effective_from.to_string())
            .add_attribute("weights", to_json_string(&weights)?),
    ))
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
    use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
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
        use nym_contracts_common_testing::{AdminExt, ContractOpts};

        #[test]
        fn requires_valid_address() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let bad_address = "foomp".to_string();
            let good_address = test.new_authorised_network_monitor();

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

    #[cfg(test)]
    mod updating_weights {
        use super::*;
        use crate::testing::{init_contract_tester, p};
        use cosmwasm_std::Attribute;
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, ContractOpts};
        use nym_performance_contract_common::{ExecuteMsg, Weights};

        fn weights(liveness: &str, stress: &str) -> Weights {
            Weights {
                liveness: p(liveness),
                stress: p(stress),
            }
        }

        #[test]
        fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let not_admin = test.addr_make("not-admin");

            let res = test
                .execute_raw(
                    not_admin,
                    ExecuteMsg::UpdateWeights {
                        weights: weights("0.7", "0.3"),
                    },
                )
                .unwrap_err();
            assert_eq!(
                res,
                NymPerformanceContractError::Admin(AdminError::NotAdmin {})
            );

            Ok(())
        }

        #[test]
        fn emits_the_effective_epoch_and_the_weights() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            test.set_mixnet_epoch(10)?;

            let res = test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::UpdateWeights {
                    weights: weights("0.7", "0.3"),
                },
            )?;

            let event = res
                .events
                .iter()
                .find(|event| event.ty == "weights_update")
                .expect("the weights update must be announced");
            assert_eq!(
                event.attributes,
                vec![
                    Attribute::new("effective_from", "11"),
                    Attribute::new("weights", r#"{"liveness":"0.7","stress":"0.3"}"#),
                ]
            );

            // and the storage agrees with the announcement
            assert_eq!(
                NYM_PERFORMANCE_CONTRACT_STORAGE
                    .weights_at(test.deps().storage, 11)?
                    .map(|weights| weights.weights),
                Some(weights("0.7", "0.3"))
            );

            Ok(())
        }
    }

    #[cfg(test)]
    mod batch_submission {
        use super::*;
        use crate::testing::{
            init_contract_tester, liveness_submission, PerformanceContractTesterExt,
        };
        use cosmwasm_std::testing::message_info;
        use cosmwasm_std::Attribute;
        use nym_performance_contract_common::BatchSubmissionResult;

        #[test]
        fn batch_submission_is_observable_from_the_response() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let nodes = test.bond_dummy_nymnodes(2);
            let env = test.env();

            // two bonded nodes and one that does not exist, in ascending order
            let res = try_batch_submit_performance_results(
                test.deps_mut(),
                env,
                message_info(&nm, &[]),
                0,
                vec![
                    liveness_submission(nodes[0], "0.5"),
                    liveness_submission(nodes[1], "0.5"),
                    liveness_submission(999999, "0.5"),
                ],
            )?;

            let data: BatchSubmissionResult =
                from_json(res.data.expect("the result is returned as data"))?;
            assert_eq!(
                data,
                BatchSubmissionResult {
                    accepted_scores: 2,
                    non_existent_nodes: vec![999999],
                }
            );

            let event = res
                .events
                .iter()
                .find(|event| event.ty == "batch_performance_submission")
                .expect("the batch is announced");
            assert_eq!(
                event.attributes,
                vec![
                    Attribute::new("accepted_scores", "2"),
                    Attribute::new("non_existent_nodes", "[999999]"),
                ]
            );

            Ok(())
        }
    }

    // panics in tests are fine...
    #[allow(clippy::panic)]
    #[test]
    fn removing_epoch_measurements_returns_binary_data() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();

        let nm = tester.new_authorised_network_monitor();

        tester.advance_mixnet_epoch()?;
        for _ in 0..2 * retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT {
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_liveness(&nm, node_id, "0.42");
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
