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

/// The mixnet contract's callback when `node_id` begins unbonding: takes the node's latest entry
/// out of the digest for good.
pub fn try_handle_node_unbonding(
    deps: DepsMut<'_>,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NymPerformanceContractError> {
    let mixnet_contract = NYM_PERFORMANCE_CONTRACT_STORAGE
        .mixnet_contract_address
        .load(deps.storage)?;
    if info.sender != mixnet_contract {
        return Err(NymPerformanceContractError::UnauthorisedMixnetCallback {
            sender: info.sender,
        });
    }

    NYM_PERFORMANCE_CONTRACT_STORAGE.remove_latest_entry(deps.storage, node_id)?;
    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_json;
    use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
    use nym_contracts_common_testing::ContractOpts;

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

    #[cfg(test)]
    mod node_unbonding {
        use super::*;
        use crate::queries::{
            query_node_performance, query_node_performance_paged, query_rewarding_inputs,
            query_rewarding_score,
        };
        use crate::testing::{
            digest_of, init_contract_tester, p, scored_entry, scored_submission,
            PerformanceContractTesterExt,
        };
        use cosmwasm_std::testing::message_info;
        use nym_performance_contract_common::constants::{
            storage_keys, MAX_FALLBACK_LOOKBACK_EPOCHS,
        };

        #[test]
        fn only_the_mixnet_contract_may_call_back() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;
            test.submit_at_epoch(&nm, 10, scored_submission(node_id, "0.9", "1"));
            let digest = test.digest();

            let impostor = test.addr_make("impostor");
            let res =
                try_handle_node_unbonding(test.deps_mut(), message_info(&impostor, &[]), node_id)
                    .unwrap_err();
            assert_eq!(
                res,
                NymPerformanceContractError::UnauthorisedMixnetCallback { sender: impostor }
            );

            assert_eq!(test.digest(), digest);
            assert_eq!(test.last_known_epoch(node_id), Some(10));

            Ok(())
        }

        #[test]
        fn a_never_measured_node_changes_nothing() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let measured = test.bond_dummy_nymnode()?;
            let never_measured = test.bond_dummy_nymnode()?;

            // nothing is written, not even the empty accumulator
            test.deliver_unbond_callback(never_measured);
            assert!(test
                .deps()
                .storage
                .get(storage_keys::DIGEST_STATE.as_bytes())
                .is_none());

            test.submit_at_epoch(&nm, 10, scored_submission(measured, "0.9", "1"));
            let digest = test.digest();
            test.deliver_unbond_callback(never_measured);
            assert_eq!(test.digest(), digest);
            assert_eq!(test.last_known_epoch(never_measured), None);

            Ok(())
        }

        #[test]
        fn the_callback_retires_the_entry_and_leaves_the_bundles() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;
            let other = test.bond_dummy_nymnode()?;

            test.submit_at_epoch(&nm, 9, scored_submission(node_id, "0.9", "1"));
            test.submit_at_epoch(&nm, 10, scored_submission(node_id, "0.8", "1"));
            test.submit_for_epoch(&nm, 10, scored_submission(other, "1", "1"));

            let epochs = 0..=10 + MAX_FALLBACK_LOOKBACK_EPOCHS + 1;
            let rewarding_before = epochs
                .clone()
                .map(|epoch_id| query_rewarding_inputs(test.deps(), epoch_id, node_id))
                .collect::<Result<Vec<_>, _>>()?;

            test.deliver_unbond_callback(node_id);

            // the entry leaves the digest and the routing view, and the pointer goes with it
            assert_eq!(test.digest(), digest_of(&[scored_entry(other, 10, "1")]));
            assert_eq!(
                test.all_latest_performance(),
                vec![scored_entry(other, 10, "1")]
            );
            assert_eq!(test.last_known_epoch(node_id), None);
            let history = query_node_performance_paged(test.deps(), node_id, None, None)?;
            assert!(history.performance.is_empty());

            // its bundles stay readable per epoch
            let at_10 = query_node_performance(test.deps(), 10, node_id)?.performance;
            assert_eq!(
                at_10.map(|performance| performance.medians.liveness),
                Some(Some(p("0.8")))
            );

            // and rewarding resolves exactly as before, through the walk now that the pointer
            // is gone
            for (epoch_id, before) in epochs.zip(rewarding_before) {
                assert_eq!(
                    query_rewarding_inputs(test.deps(), epoch_id, node_id)?,
                    before
                );
                assert_eq!(
                    query_rewarding_score(test.deps(), epoch_id, node_id)?.score,
                    before.score
                );
            }

            Ok(())
        }
    }

    // a real `UnbondNymNode` executed on the mixnet contract, whose callback is a sub-message
    // that only an app-level execution runs
    #[cfg(test)]
    mod real_unbonding {
        use super::*;
        use crate::queries::query_rewarding_inputs;
        use crate::testing::{
            init_contract_tester, p, scored_entry, scored_submission, PerformanceContractTesterExt,
        };
        use mixnet_contract::testable_mixnet_contract::MixnetContractSiblings;
        use nym_performance_contract_common::ExecuteMsg;

        #[test]
        fn a_real_unbond_removes_the_node_from_the_digest() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;
            let other = test.bond_dummy_nymnode()?;
            test.submit_at_epoch(&nm, 10, scored_submission(node_id, "0.9", "1"));
            test.submit_for_epoch(&nm, 10, scored_submission(other, "1", "1"));
            let bundle = test.read_bundle(10, node_id);

            test.begin_unbonding(node_id);

            assert_eq!(
                test.all_latest_performance(),
                vec![scored_entry(other, 10, "1")]
            );
            test.assert_digest_is_refold();
            assert_eq!(test.last_known_epoch(node_id), None);
            assert_eq!(test.read_bundle(10, node_id), bundle);

            Ok(())
        }

        #[test]
        fn an_unbonded_node_cannot_return() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let first = test.new_authorised_network_monitor();
            let second = test.new_authorised_network_monitor();
            let third = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;
            test.submit_at_epoch(&first, 10, scored_submission(node_id, "0.9", "1"));
            test.begin_unbonding(node_id);
            let digest = test.digest();

            // within the same epoch, the mixnet contract already reports the node as not bonded
            let res = test
                .execute_raw(
                    second,
                    ExecuteMsg::Submit {
                        epoch: 10,
                        data: scored_submission(node_id, "0.8", "1"),
                    },
                )
                .unwrap_err();
            assert_eq!(res, NymPerformanceContractError::NodeNotBonded { node_id });

            let env = test.env();
            let res = NYM_PERFORMANCE_CONTRACT_STORAGE.batch_submit_performance_results(
                test.deps_mut(),
                env,
                &third,
                10,
                vec![scored_submission(node_id, "0.8", "1")],
            )?;
            assert_eq!(res.non_existent_nodes, vec![node_id]);

            assert_eq!(test.digest(), digest);
            assert_eq!(test.last_known_epoch(node_id), None);

            Ok(())
        }

        #[test]
        fn a_nodes_last_epoch_resolves_after_it_began_unbonding() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;
            let other = test.bond_dummy_nymnode()?;
            test.submit_at_epoch(&nm, 9, scored_submission(node_id, "0.9", "1"));

            // other nodes are measured in epoch 10, and this one begins unbonding during it
            // before any monitor has reported it
            test.submit_at_epoch(&nm, 10, scored_submission(other, "1", "1"));
            test.begin_unbonding(node_id);

            let res = query_rewarding_inputs(test.deps(), 10, node_id)?;
            assert_eq!(res.source.map(|source| source.epoch_id), Some(9));
            assert_eq!(res.score, Some(p("0.9")));

            Ok(())
        }

        #[test]
        fn without_the_address_no_callback_is_sent() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            test.set_mixnet_sibling_contracts(MixnetContractSiblings::default().with_clear_all())?;
            let nm = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;
            test.submit_at_epoch(&nm, 10, scored_submission(node_id, "0.9", "1"));

            // the unbond itself succeeds, and the committed set still holds the node
            test.begin_unbonding(node_id);
            assert_eq!(
                test.all_latest_performance(),
                vec![scored_entry(node_id, 10, "0.9")]
            );
            assert_eq!(test.last_known_epoch(node_id), Some(10));

            Ok(())
        }
    }
}
