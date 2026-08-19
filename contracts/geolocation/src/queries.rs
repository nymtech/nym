// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::{build_storage_key, retrieval_limits, GEOLOCATION_CONTRACT_STORAGE};
use cosmwasm_std::Deps;
use cw_controllers::AdminResponse;
use nym_geolocation_contract_common::{
    AllRecordsPagedResponse, ConfigResponse, DigestResponse, EntryResponse,
    GeolocationContractError, GeolocationRecord, LocationEntry, RecordKey, Source, SourceEntry,
    Subject, SubjectEntriesResponse, WhitelistResponse,
};
use nym_mixnet_contract_common::NodeId;

pub fn query_admin(deps: Deps) -> Result<AdminResponse, GeolocationContractError> {
    GEOLOCATION_CONTRACT_STORAGE
        .contract_admin
        .query_admin(deps)
        .map_err(Into::into)
}

/// The contract's tunables, together with the mixnet contract address they operate against.
///
/// The address rides along rather than getting a query of its own: it is fixed at instantiation,
/// and a reader needs to know which deployment node identity keys were resolved against before
/// it can judge what a self-declared entry proves.
pub fn query_config(deps: Deps) -> Result<ConfigResponse, GeolocationContractError> {
    Ok(ConfigResponse {
        mixnet_contract_address: GEOLOCATION_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(deps.storage)?,
        config: GEOLOCATION_CONTRACT_STORAGE.config.load(deps.storage)?,
    })
}

/// The whole agent whitelist, in ascending address order.
///
/// Unpaginated: the set is small and NYM-controlled. It is also exactly what a client needs in
/// full before it can decide which measured entries to honour, so paging it would move the
/// reassembly into every caller rather than saving anyone work.
pub fn query_whitelist(deps: Deps) -> Result<WhitelistResponse, GeolocationContractError> {
    Ok(WhitelistResponse {
        agents: GEOLOCATION_CONTRACT_STORAGE.all_whitelisted_agents(deps.storage)?,
    })
}

/// A single entry, by its full key. An empty slot is `None` rather than an error: a caller
/// asking whether a source has written anything for a subject is asking a question, not making
/// a claim that it has.
pub fn query_entry(
    deps: Deps,
    subject: Subject,
    source: Source,
) -> Result<EntryResponse, GeolocationContractError> {
    let entry = GEOLOCATION_CONTRACT_STORAGE.may_load_entry(deps.storage, &subject, &source)?;
    Ok(EntryResponse { entry })
}

/// Everything held for one subject, across all sources, in ascending source order.
pub fn query_subject_entries(
    deps: Deps,
    subject: Subject,
) -> Result<SubjectEntriesResponse, GeolocationContractError> {
    let entries = GEOLOCATION_CONTRACT_STORAGE.subject_entries(deps.storage, &subject)?;
    Ok(subject_entries_response(subject, entries))
}

/// [`query_subject_entries`] for the one subject class that exists today, saving callers that
/// work in `NodeId`s from assembling a [`Subject`] themselves.
pub fn query_nym_node_entries(
    deps: Deps,
    node_id: NodeId,
) -> Result<SubjectEntriesResponse, GeolocationContractError> {
    query_subject_entries(deps, Subject::new_nym_node(node_id))
}

/// Only the measured entries for one subject, dropping any self-declaration and override.
pub fn query_subject_measurements(
    deps: Deps,
    subject: Subject,
) -> Result<SubjectEntriesResponse, GeolocationContractError> {
    let entries = GEOLOCATION_CONTRACT_STORAGE.subject_measurements(deps.storage, &subject)?;
    Ok(subject_entries_response(subject, entries))
}

fn subject_entries_response(
    subject: Subject,
    entries: Vec<(Source, LocationEntry)>,
) -> SubjectEntriesResponse {
    SubjectEntriesResponse {
        subject,
        entries: entries
            .into_iter()
            .map(|(source, entry)| SourceEntry { source, entry })
            .collect(),
    }
}

/// One page of every digest-committed record, across both entry classes.
///
/// This is the pull a client folds to recompute the accumulator for itself, so the enumeration
/// has to be complete rather than merely convenient: it covers the agent whitelist as well as the
/// location entries, because a measured entry carries no signature and a reader that could not
/// see which writers were authorised would have no way to reject entries laundered through a
/// fabricated whitelist.
///
/// Pages are only comparable with a digest if they all come from one height, which is the
/// client's job: this contract cannot pin a height for it.
pub fn query_all_records_paged(
    deps: Deps,
    start_after: Option<RecordKey>,
    limit: Option<u32>,
) -> Result<AllRecordsPagedResponse, GeolocationContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::ALL_RECORDS_DEFAULT_LIMIT)
        .min(retrieval_limits::ALL_RECORDS_MAX_LIMIT) as usize;

    // we start the scan from the measurements, so if somebody asked for whitelist start,
    // we don't have to do more checks regarding ranges or further queries
    let mut records: Vec<GeolocationRecord> = match start_after {
        None => GEOLOCATION_CONTRACT_STORAGE
            .entries_paged(deps.storage, None, limit)?
            .into_iter()
            .map(Into::into)
            .collect(),
        Some(RecordKey::Location(entry_key)) => GEOLOCATION_CONTRACT_STORAGE
            .entries_paged(deps.storage, Some(build_storage_key(entry_key)), limit)?
            .into_iter()
            .map(Into::into)
            .collect(),
        Some(RecordKey::WhitelistedAgent { agent }) => {
            let entries = GEOLOCATION_CONTRACT_STORAGE.whitelisted_agents_paged(
                deps.storage,
                Some(agent),
                limit,
            )?;
            return Ok(AllRecordsPagedResponse::new(
                entries.into_iter().map(Into::into).collect(),
            ));
        }
    };

    // whatever the locations left unfilled goes to the whitelist, so a page can span both
    let remaining_slots = limit - records.len();
    if remaining_slots > 0 {
        records.extend(
            GEOLOCATION_CONTRACT_STORAGE
                .whitelisted_agents_paged(deps.storage, None, remaining_slots)?
                .into_iter()
                .map(GeolocationRecord::from),
        );
    }

    Ok(AllRecordsPagedResponse::new(records))
}

/// The 32-byte collapse of the accumulator, for comparing digests cheaply.
///
/// Unproven, and unavoidably so: smart queries carry no proof at all. A client that needs one
/// performs a raw store read at [`storage_keys::DIGEST_STATE`][k] instead, which returns the
/// full accumulator, and collapses it itself.
///
/// [k]: nym_geolocation_contract_common::constants::storage_keys::DIGEST_STATE
pub fn query_digest(deps: Deps) -> Result<DigestResponse, GeolocationContractError> {
    let digest = GEOLOCATION_CONTRACT_STORAGE.load_digest(deps.storage)?;
    Ok(DigestResponse {
        digest: digest.out().to_vec().into(),
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
        use nym_geolocation_contract_common::ExecuteMsg;

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
    mod config_query {
        use super::*;
        use crate::testing::init_contract_tester;
        use crate::transactions::try_update_config;
        use cosmwasm_std::testing::message_info;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts};
        use nym_geolocation_contract_common::constants::{
            DEFAULT_MAX_BATCH_SIZE, DEFAULT_MAX_PAYLOAD_SIZE, DEFAULT_MAX_SKEW_SECS,
        };
        use nym_geolocation_contract_common::{ContractConfig, QueryMsg};

        #[test]
        fn reports_what_instantiation_established() -> anyhow::Result<()> {
            let test = init_contract_tester();

            let res = query_config(test.deps())?;
            assert_eq!(
                res.config,
                ContractConfig {
                    max_skew_secs: DEFAULT_MAX_SKEW_SECS,
                    max_batch_size: DEFAULT_MAX_BATCH_SIZE,
                    max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
                }
            );

            // the address is what tells a reader which deployment a self-declaration's identity
            // key was resolved against, so serving the contract's own address, or the admin's,
            // would be worse than serving nothing
            assert_eq!(
                res.mixnet_contract_address,
                test.mixnet_contract_address().unwrap()
            );
            assert_ne!(res.mixnet_contract_address, test.contract_address);
            assert_ne!(res.mixnet_contract_address, test.admin_unchecked());

            Ok(())
        }

        #[test]
        fn every_tunable_is_reported_in_its_own_field() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();

            // three distinct values, so the two adjacent `u32`s cannot be transposed unnoticed
            try_update_config(
                test.deps_mut(),
                message_info(&admin, &[]),
                Some(11),
                Some(22),
                Some(33),
            )?;

            assert_eq!(
                query_config(test.deps())?.config,
                ContractConfig {
                    max_skew_secs: 11,
                    max_batch_size: 22,
                    max_payload_size: 33,
                }
            );

            Ok(())
        }

        #[test]
        fn the_config_query_is_reachable_through_the_query_entry_point() -> anyhow::Result<()> {
            let test = init_contract_tester();

            let res: ConfigResponse = test.query(&QueryMsg::Config {})?;
            assert_eq!(res, query_config(test.deps())?);

            Ok(())
        }
    }

    #[cfg(test)]
    mod whitelist_query {
        use super::*;
        use crate::testing::{init_contract_tester, GeolocationContractTesterExt};
        use nym_contracts_common_testing::{ChainOpts, ContractOpts};
        use nym_geolocation_contract_common::{AgentPermissions, QueryMsg, WhitelistEntry};

        const MEASURE_ONLY: AgentPermissions = AgentPermissions {
            can_measure: true,
            can_relay_self_declared: false,
        };
        const RELAY_ONLY: AgentPermissions = AgentPermissions {
            can_measure: false,
            can_relay_self_declared: true,
        };
        const BOTH: AgentPermissions = AgentPermissions {
            can_measure: true,
            can_relay_self_declared: true,
        };

        #[test]
        fn an_empty_whitelist_is_an_empty_list_rather_than_an_error() -> anyhow::Result<()> {
            let test = init_contract_tester();

            assert!(query_whitelist(test.deps())?.agents.is_empty());

            Ok(())
        }

        #[test]
        fn each_agent_is_paired_with_its_own_permissions() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            // the two single-flag grants are what a constant, a default or a transposed pair of
            // booleans would all get wrong, and they are the grants that actually restrict
            // anything
            let measurer = test.add_agent_with_permissions(MEASURE_ONLY);
            let relayer = test.add_agent_with_permissions(RELAY_ONLY);
            let both = test.add_agent_with_permissions(BOTH);

            let agents = query_whitelist(test.deps())?.agents;
            assert_eq!(agents.len(), 3);
            for (agent, expected) in [
                (measurer, MEASURE_ONLY),
                (relayer, RELAY_ONLY),
                (both, BOTH),
            ] {
                let found = agents
                    .iter()
                    .find(|e| e.agent == agent)
                    .unwrap_or_else(|| panic!("{agent} is missing from the whitelist"));
                assert_eq!(found.permissions, expected, "wrong permissions for {agent}");
            }

            Ok(())
        }

        #[test]
        fn agents_come_back_in_ascending_address_order() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            for _ in 0..5 {
                test.add_dummy_agent();
            }

            let agents = query_whitelist(test.deps())?
                .agents
                .into_iter()
                .map(|e| e.agent)
                .collect::<Vec<_>>();

            let mut sorted = agents.clone();
            sorted.sort();
            assert_eq!(agents, sorted);

            Ok(())
        }

        #[test]
        fn the_answer_tracks_grants_changes_and_revocations() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let whitelist = |test: &_| -> anyhow::Result<Vec<WhitelistEntry>> {
                Ok(query_whitelist(ContractOpts::deps(test))?.agents)
            };

            let agent = test.add_agent_with_permissions(MEASURE_ONLY);
            assert_eq!(
                whitelist(&test)?,
                vec![WhitelistEntry {
                    agent: agent.clone(),
                    permissions: MEASURE_ONLY
                }]
            );

            // a re-grant replaces the permissions in place rather than adding a second row
            GEOLOCATION_CONTRACT_STORAGE.set_whitelisted_agent(
                &mut test,
                agent.clone(),
                RELAY_ONLY,
            )?;
            assert_eq!(
                whitelist(&test)?,
                vec![WhitelistEntry {
                    agent: agent.clone(),
                    permissions: RELAY_ONLY
                }]
            );

            // de-whitelisting is what read-time authorisation rests on, so it has to be visible
            // here immediately, even though the agent's entries stay in storage
            test.set_dummy_measurement_from(42, &agent);
            test.remove_agent(&agent);
            assert!(whitelist(&test)?.is_empty());
            assert_eq!(test.node_entries(42).len(), 1);

            Ok(())
        }

        #[test]
        fn the_whitelist_query_is_reachable_through_the_query_entry_point() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            test.add_agent_with_permissions(MEASURE_ONLY);

            let res: WhitelistResponse = test.query(&QueryMsg::Whitelist {})?;
            assert_eq!(res, query_whitelist(test.deps())?);
            assert_eq!(res.agents.len(), 1);

            Ok(())
        }
    }

    #[cfg(test)]
    mod entry_queries {
        use super::*;
        use crate::testing::{
            init_contract_tester, location_entry, measured_by, GeolocationContractTesterExt,
        };
        use nym_contracts_common_testing::{ChainOpts, ContractOpts};
        use nym_geolocation_contract_common::{QueryMsg, Subject};

        #[test]
        fn a_single_entry_query_returns_exactly_what_was_stored() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            let source = measured_by(&agent);

            let stored = location_entry(b"whatever-the-agent-said", 1234);
            test.set_node_entry(42, source.clone(), stored.clone());

            let res = query_entry(test.deps(), Subject::new_nym_node(42), source)?;
            assert_eq!(res.entry, Some(stored));

            Ok(())
        }

        #[test]
        fn an_empty_slot_is_absent_rather_than_an_error() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            // a subject nothing was ever written for
            let res = query_entry(test.deps(), Subject::new_nym_node(42), measured_by(&agent))?;
            assert_eq!(res.entry, None);

            // and a subject that holds an entry, but not from this source
            test.set_dummy_node_measurement(42);
            let res = query_entry(test.deps(), Subject::new_nym_node(42), Source::Override)?;
            assert_eq!(res.entry, None);

            Ok(())
        }

        #[test]
        fn a_single_entry_query_distinguishes_sources_for_one_subject() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let agent1 = test.add_dummy_agent();
            let agent2 = test.add_dummy_agent();

            // every source for node 42 holds different content, so any key component the query
            // failed to honour would return somebody else's entry rather than nothing
            test.set_node_entry(42, measured_by(&agent1), location_entry(b"from-agent-1", 1));
            test.set_node_entry(42, measured_by(&agent2), location_entry(b"from-agent-2", 2));
            test.set_node_entry(42, Source::SelfDeclared, location_entry(b"self", 3));
            test.set_node_entry(42, Source::Override, location_entry(b"override", 4));

            for (source, expected) in [
                (measured_by(&agent1), b"from-agent-1".as_slice()),
                (measured_by(&agent2), b"from-agent-2".as_slice()),
                (Source::SelfDeclared, b"self".as_slice()),
                (Source::Override, b"override".as_slice()),
            ] {
                let res = query_entry(test.deps(), Subject::new_nym_node(42), source.clone())?;
                let entry = res.entry.expect("entry should exist");
                assert_eq!(
                    entry.payload.content.as_slice(),
                    expected,
                    "wrong entry returned for {source:?}"
                );
            }

            Ok(())
        }

        #[test]
        fn subject_entries_returns_every_source_in_ascending_order() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let agent1 = test.add_dummy_agent();
            let agent2 = test.add_dummy_agent();

            // deliberately written in an order unrelated to the key order they must come back in
            test.set_dummy_node_override(42);
            test.set_dummy_measurement_from(42, &agent2);
            test.set_dummy_node_self_declared(42);
            test.set_dummy_measurement_from(42, &agent1);

            let res = query_subject_entries(test.deps(), Subject::new_nym_node(42))?;
            assert_eq!(res.subject, Subject::new_nym_node(42));

            // measurements first, ordered by agent, then the self-declared slot, then the
            // override: the ordering the source tags were chosen to produce
            let (lower_agent, higher_agent) = if agent1 < agent2 {
                (&agent1, &agent2)
            } else {
                (&agent2, &agent1)
            };
            let sources = res.entries.iter().map(|e| &e.source).collect::<Vec<_>>();
            assert_eq!(
                sources,
                vec![
                    &measured_by(lower_agent),
                    &measured_by(higher_agent),
                    &Source::SelfDeclared,
                    &Source::Override,
                ]
            );

            Ok(())
        }

        #[test]
        fn subject_entries_pairs_each_source_with_its_own_entry() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            test.set_node_entry(42, measured_by(&agent), location_entry(b"measured", 1));
            test.set_node_entry(42, Source::Override, location_entry(b"override", 2));

            let res = query_subject_entries(test.deps(), Subject::new_nym_node(42))?;
            for pair in res.entries {
                let expected: &[u8] = match pair.source {
                    Source::Measured { .. } => b"measured",
                    Source::Override => b"override",
                    Source::SelfDeclared => panic!("nothing wrote a self-declaration"),
                };
                assert_eq!(pair.entry.payload.content.as_slice(), expected);
            }

            Ok(())
        }

        #[test]
        fn subject_entries_are_scoped_to_the_subject_asked_for() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let _agent = test.add_dummy_agent();

            // neighbouring ids on either side, since the subject id is the key component a
            // botched prefix would bleed across
            for node_id in [41, 42, 43] {
                test.set_dummy_node_measurement(node_id);
                test.set_dummy_node_override(node_id);
            }

            let res = query_subject_entries(test.deps(), Subject::new_nym_node(42))?;
            assert_eq!(res.entries.len(), 2);
            for pair in res.entries {
                let content = String::from_utf8(pair.entry.payload.content.to_vec())?;
                assert!(
                    content.contains("42") && !content.contains("41") && !content.contains("43"),
                    "leaked a neighbouring subject's entry: {content}"
                );
            }

            Ok(())
        }

        #[test]
        fn an_unknown_subject_has_no_entries_rather_than_erroring() -> anyhow::Result<()> {
            let test = init_contract_tester();

            let res = query_subject_entries(test.deps(), Subject::new_nym_node(1))?;
            assert_eq!(res.subject, Subject::new_nym_node(1));
            assert!(res.entries.is_empty());

            let res = query_subject_measurements(test.deps(), Subject::new_nym_node(1))?;
            assert!(res.entries.is_empty());

            Ok(())
        }

        #[test]
        fn subject_measurements_drops_the_self_declaration_and_the_override() -> anyhow::Result<()>
        {
            let mut test = init_contract_tester();
            let agent1 = test.add_dummy_agent();
            let agent2 = test.add_dummy_agent();

            test.set_dummy_measurement_from(42, &agent1);
            test.set_dummy_measurement_from(42, &agent2);
            test.set_dummy_node_self_declared(42);
            test.set_dummy_node_override(42);

            // sanity check: all four are there when nothing is filtered
            assert_eq!(
                query_subject_entries(test.deps(), Subject::new_nym_node(42))?
                    .entries
                    .len(),
                4
            );

            let res = query_subject_measurements(test.deps(), Subject::new_nym_node(42))?;
            assert_eq!(res.entries.len(), 2);
            assert!(res.entries.iter().all(|e| e.source.is_measured()));

            Ok(())
        }

        #[test]
        fn the_nym_node_shorthand_matches_the_subject_query_it_stands_for() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let _agent = test.add_dummy_agent();

            // populate neighbours too, so constructing the wrong subject would be visible
            for node_id in [41, 42, 43] {
                test.set_dummy_node_measurement(node_id);
            }

            assert_eq!(
                query_nym_node_entries(test.deps(), 42)?,
                query_subject_entries(test.deps(), Subject::new_nym_node(42))?
            );

            Ok(())
        }

        #[test]
        fn every_entry_query_is_reachable_through_the_query_entry_point() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            let source = measured_by(&agent);
            let subject = Subject::new_nym_node(42);

            test.set_dummy_measurement_from(42, &agent);
            test.set_dummy_node_override(42);

            // going through the dispatcher rather than calling the handlers directly, so a
            // transposed match arm is caught: every one of these returns the same shape as at
            // least one of its neighbours
            let entry: EntryResponse = test.query(&QueryMsg::Entry {
                subject: subject.clone(),
                source: source.clone(),
            })?;
            assert_eq!(entry, query_entry(test.deps(), subject.clone(), source)?);

            let subject_entries: SubjectEntriesResponse =
                test.query(&QueryMsg::SubjectEntries {
                    subject: subject.clone(),
                })?;
            assert_eq!(
                subject_entries,
                query_subject_entries(test.deps(), subject.clone())?
            );
            assert_eq!(subject_entries.entries.len(), 2);

            let nym_node_entries: SubjectEntriesResponse =
                test.query(&QueryMsg::NymNodeEntries { node_id: 42 })?;
            assert_eq!(nym_node_entries, subject_entries);

            let measurements: SubjectEntriesResponse =
                test.query(&QueryMsg::SubjectMeasurements {
                    subject: subject.clone(),
                })?;
            assert_eq!(
                measurements,
                query_subject_measurements(test.deps(), subject)?
            );
            assert_eq!(measurements.entries.len(), 1);

            Ok(())
        }
    }

    #[cfg(test)]
    mod paged_enumeration {
        use super::*;
        use crate::testing::{init_contract_tester, GeolocationContractTesterExt};
        use nym_contracts_common_testing::{ChainOpts, ContractOpts};
        use nym_geolocation_contract_common::{GeolocationRecord, QueryMsg};

        /// Enough of both classes that a page boundary can fall inside either one or exactly
        /// between them: 9 locations spread over 3 subjects and all 3 sources, then 4 agents.
        fn populated() -> impl GeolocationContractTesterExt {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            for node_id in [1, 2, 3] {
                test.set_dummy_measurement_from(node_id, &agent);
                test.set_dummy_node_self_declared(node_id);
                test.set_dummy_node_override(node_id);
            }
            for _ in 0..3 {
                test.add_dummy_agent();
            }
            test
        }

        const LOCATIONS: usize = 9;
        const AGENTS: usize = 4;
        const TOTAL: usize = LOCATIONS + AGENTS;

        fn page(
            test: &impl GeolocationContractTesterExt,
            start_after: Option<RecordKey>,
            limit: Option<u32>,
        ) -> anyhow::Result<AllRecordsPagedResponse> {
            Ok(query_all_records_paged(
                ContractOpts::deps(test),
                start_after,
                limit,
            )?)
        }

        /// Page from the beginning with the returned cursor until it is absent. The iteration
        /// bound is the test for termination: a cursor that never retires fails here rather than
        /// hanging the suite.
        fn page_through(
            test: &impl GeolocationContractTesterExt,
            limit: Option<u32>,
        ) -> anyhow::Result<Vec<GeolocationRecord>> {
            let mut collected = Vec::new();
            let mut start_after = None;

            for _ in 0..(TOTAL + 2) {
                let res = page(test, start_after, limit)?;
                collected.extend(res.records);
                match res.start_next_after {
                    Some(cursor) => start_after = Some(cursor),
                    None => return Ok(collected),
                }
            }

            panic!("the cursor never retired")
        }

        #[test]
        fn paging_through_yields_exactly_the_committed_set() -> anyhow::Result<()> {
            let test = populated();
            let expected = test.all_records();
            assert_eq!(expected.len(), TOTAL);

            // every page size that matters: below, at and above each class boundary, and one
            // larger than the whole set. The interesting ones are 9 (a page ending exactly where
            // the locations do) and 10 (a page straddling the two stores)
            for limit in [1, 2, 3, 8, 9, 10, 12, 13, TOTAL + 1] {
                let collected = page_through(&test, Some(limit as u32))?;
                assert_eq!(
                    collected, expected,
                    "paging at {limit} per page did not reproduce the enumeration"
                );
            }

            // and the same with the caller naming no limit at all
            assert_eq!(page_through(&test, None)?, expected);

            Ok(())
        }

        #[test]
        fn every_record_is_returned_exactly_once() -> anyhow::Result<()> {
            let test = populated();

            for limit in [1, 4, 9, 10, TOTAL] {
                let collected = page_through(&test, Some(limit as u32))?;
                let mut seen: Vec<RecordKey> = Vec::new();
                for record in &collected {
                    let key = record.key();
                    assert!(
                        !seen.contains(&key),
                        "{key:?} was emitted twice at {limit} per page"
                    );
                    seen.push(key);
                }
                assert_eq!(collected.len(), TOTAL);
            }

            Ok(())
        }

        #[test]
        fn a_page_ending_on_the_class_boundary_continues_into_the_whitelist() -> anyhow::Result<()>
        {
            let test = populated();

            // exactly the locations, so the cursor names a location and yet nothing is left in
            // that store: the next page has to cross into the whitelist rather than stop
            let first = page(&test, None, Some(LOCATIONS as u32))?;
            assert_eq!(first.records.len(), LOCATIONS);
            assert!(first
                .records
                .iter()
                .all(|r| matches!(r, GeolocationRecord::Location(..))));
            let cursor = first.start_next_after.expect("more records remain");
            assert!(matches!(cursor, RecordKey::Location(..)));

            let second = page(&test, Some(cursor), Some(LOCATIONS as u32))?;
            assert_eq!(second.records.len(), AGENTS);
            assert!(second
                .records
                .iter()
                .all(|r| matches!(r, GeolocationRecord::WhitelistedAgent(..))));

            Ok(())
        }

        #[test]
        fn a_page_straddles_the_two_stores_in_one_answer() -> anyhow::Result<()> {
            let test = populated();

            // one past the locations, so a single page must contain the tail of one store and
            // the head of the other
            let res = page(&test, None, Some(LOCATIONS as u32 + 1))?;
            assert_eq!(res.records.len(), LOCATIONS + 1);
            assert!(matches!(
                res.records.last(),
                Some(GeolocationRecord::WhitelistedAgent(..))
            ));

            Ok(())
        }

        #[test]
        fn an_agent_cursor_does_not_rewind_into_the_locations() -> anyhow::Result<()> {
            let test = populated();
            let agents = query_whitelist(test.deps())?.agents;

            // resuming after the first agent must yield only the agents that follow it: the
            // entries store is behind the cursor, and re-walking it would duplicate every
            // location on every remaining page
            let res = page(
                &test,
                Some(RecordKey::WhitelistedAgent {
                    agent: agents[0].agent.clone(),
                }),
                Some(TOTAL as u32),
            )?;

            assert_eq!(res.records.len(), AGENTS - 1);
            assert!(res
                .records
                .iter()
                .all(|r| matches!(r, GeolocationRecord::WhitelistedAgent(..))));

            Ok(())
        }

        #[test]
        fn a_cursor_excludes_only_the_record_it_names() -> anyhow::Result<()> {
            let test = populated();

            // the subjects each hold three sources, so a bound that excluded the whole subject
            // prefix rather than the single key would silently drop two records here
            let all = test.all_records();
            let first = page(&test, None, Some(1))?;
            assert_eq!(first.records, all[..1]);

            let second = page(&test, first.start_next_after, Some(1))?;
            assert_eq!(second.records, all[1..2]);

            Ok(())
        }

        #[test]
        fn an_over_large_limit_is_clamped_rather_than_refused() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            let over = retrieval_limits::ALL_RECORDS_MAX_LIMIT as usize + 10;
            for node_id in 0..over as u32 {
                test.set_dummy_measurement_from(node_id, &agent);
            }

            let res = page(&test, None, Some(over as u32))?;
            assert_eq!(
                res.records.len(),
                retrieval_limits::ALL_RECORDS_MAX_LIMIT as usize
            );

            // clamping must not cost anything: paging on still reaches every record
            assert_eq!(page_through_within(&test, Some(over as u32), 8)?, over + 1);

            Ok(())
        }

        /// [`page_through`] for the one test that builds a store larger than this module's
        /// fixed record count; returns how many records the enumeration yielded.
        ///
        /// Bounded like its sibling, and for the same reason: a cursor that cycles rather than
        /// advancing has to fail the test rather than hang the suite.
        fn page_through_within(
            test: &impl GeolocationContractTesterExt,
            limit: Option<u32>,
            max_pages: usize,
        ) -> anyhow::Result<usize> {
            let mut count = 0;
            let mut start_after = None;

            for _ in 0..max_pages {
                let res = page(test, start_after, limit)?;
                count += res.records.len();
                match res.start_next_after {
                    Some(cursor) => start_after = Some(cursor),
                    None => return Ok(count),
                }
            }

            panic!("the cursor never retired")
        }

        #[test]
        fn the_default_limit_applies_when_the_caller_names_none() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            let over = retrieval_limits::ALL_RECORDS_DEFAULT_LIMIT as usize + 5;
            for node_id in 0..over as u32 {
                test.set_dummy_measurement_from(node_id, &agent);
            }

            let res = page(&test, None, None)?;
            assert_eq!(
                res.records.len(),
                retrieval_limits::ALL_RECORDS_DEFAULT_LIMIT as usize
            );

            Ok(())
        }

        #[test]
        fn an_empty_contract_enumerates_to_nothing_and_retires_the_cursor() -> anyhow::Result<()> {
            let test = init_contract_tester();

            let res = page(&test, None, None)?;
            assert!(res.records.is_empty());
            assert!(res.start_next_after.is_none());

            Ok(())
        }

        #[test]
        fn the_enumeration_is_reachable_through_the_query_entry_point() -> anyhow::Result<()> {
            let test = populated();

            // through the dispatcher, and with the cursor making a JSON round trip, since the
            // cursor is the one query input a client hands straight back to the contract
            let first: AllRecordsPagedResponse = test.query(&QueryMsg::AllRecords {
                start_after: None,
                limit: Some(5),
            })?;
            assert_eq!(first.records.len(), 5);

            let second: AllRecordsPagedResponse = test.query(&QueryMsg::AllRecords {
                start_after: first.start_next_after,
                limit: Some(5),
            })?;
            assert_eq!(second.records.len(), 5);
            assert_eq!(second.records, test.all_records()[5..10]);

            Ok(())
        }
    }

    #[cfg(test)]
    mod digest_query {
        use super::*;
        use crate::testing::{init_contract_tester, GeolocationContractTesterExt};
        use nym_contracts_common_testing::{
            ArbitraryContractStorageReader, ChainOpts, ContractOpts,
        };
        use nym_geolocation_contract_common::constants::storage_keys;
        use nym_geolocation_contract_common::{AgentPermissions, QueryMsg};
        use nym_lthash::{LtHash16, DIGEST_LEN};

        #[test]
        fn an_untouched_contract_returns_the_collapse_of_the_empty_accumulator(
        ) -> anyhow::Result<()> {
            let test = init_contract_tester();

            // nothing has been written, so the digest key holds nothing at all; the query still
            // answers, with the identity accumulator's collapse rather than an error
            assert!(test
                .may_read_from_contract_storage(
                    test.contract_address.clone(),
                    storage_keys::DIGEST_STATE
                )
                .is_none());
            assert_eq!(
                query_digest(test.deps())?.digest.to_vec(),
                LtHash16::new().out().to_vec()
            );

            Ok(())
        }

        #[test]
        fn the_query_returns_the_collapse_of_the_stored_accumulator() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let _agent = test.add_dummy_agent();
            test.set_dummy_node_measurement(42);
            test.set_dummy_node_self_declared(42);

            assert_eq!(
                query_digest(test.deps())?.digest.to_vec(),
                test.digest().out().to_vec()
            );

            Ok(())
        }

        #[test]
        fn the_query_collapses_the_same_bytes_a_proof_would_cover() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let _agent = test.add_dummy_agent();
            test.set_dummy_node_measurement(42);

            // the raw read is what carries an ICS23 proof, so what the smart query serves has to
            // be derivable from it and nothing else
            let raw = test.must_read_from_contract_storage(
                test.contract_address.clone(),
                storage_keys::DIGEST_STATE,
            )?;
            assert_eq!(
                raw.len(),
                DIGEST_LEN,
                "the proven key must hold the accumulator, not its collapse"
            );

            let raw: [u8; DIGEST_LEN] = raw.try_into().expect("just checked the length");
            assert_eq!(
                query_digest(test.deps())?.digest.to_vec(),
                LtHash16::from_bytes(&raw).out().to_vec()
            );

            Ok(())
        }

        #[test]
        fn the_query_tracks_every_change_to_the_committed_set() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let digest = |test: &_| -> anyhow::Result<Vec<u8>> {
                Ok(query_digest(ContractOpts::deps(test))?.digest.to_vec())
            };

            let empty = digest(&test)?;

            // a whitelist entry is a committed record in its own right, not configuration
            let agent = test.add_dummy_agent();
            let with_agent = digest(&test)?;
            assert_ne!(empty, with_agent);

            // a location entry
            test.set_dummy_node_measurement(42);
            let with_entry = digest(&test)?;
            assert_ne!(with_agent, with_entry);

            // a change to an existing entry's content
            test.update_dummy_node_measurement(42);
            let updated = digest(&test)?;
            assert_ne!(with_entry, updated);

            // and a change to nothing but its `checked_at`, which is what makes an unchanged
            // resubmission observable as a freshness heartbeat
            test.node_heartbeat(42);
            let heartbeat = digest(&test)?;
            assert_ne!(updated, heartbeat);

            // permissions are committed too, not just membership
            GEOLOCATION_CONTRACT_STORAGE.set_whitelisted_agent(
                &mut test,
                agent.clone(),
                AgentPermissions {
                    can_measure: true,
                    can_relay_self_declared: false,
                },
            )?;
            assert_ne!(heartbeat, digest(&test)?);

            // and removing everything returns it to where it started
            test.remove_all_locations();
            test.remove_all_agents();
            assert_eq!(digest(&test)?, empty);

            Ok(())
        }

        #[test]
        fn the_digest_query_is_reachable_through_the_query_entry_point() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let _agent = test.add_dummy_agent();
            test.set_dummy_node_measurement(42);

            let res: DigestResponse = test.query(&QueryMsg::Digest {})?;
            assert_eq!(res, query_digest(test.deps())?);
            assert_eq!(res.digest.len(), 32);

            Ok(())
        }
    }

    /// The whole point of the contract, exercised the way a verifying client actually would:
    /// through the public query surface only, touching no storage helper and no handler.
    ///
    /// Every other test here checks one link of that chain against contract internals. This one
    /// checks the chain end to end, which matters because the enumeration and `all_records` now
    /// share `entries_paged`, so comparing them no longer cross-checks two implementations.
    #[cfg(test)]
    mod client_recompute {
        use super::*;
        use crate::testing::{init_contract_tester, measured_by, GeolocationContractTesterExt};
        use nym_contracts_common_testing::{ArbitraryContractStorageReader, ChainOpts};
        use nym_geolocation_contract_common::constants::{storage_keys, PAYLOAD_VERSION_1};
        use nym_geolocation_contract_common::{GeolocationRecord, LocationPayload, QueryMsg};
        use nym_lthash::{LtHash16, DIGEST_LEN};

        /// A payload version this contract has never been taught to read. It stores and commits
        /// opaque bytes, so nothing about enumerating or folding an entry may depend on the
        /// version byte - which is what lets a version 2 payload ship without a migration.
        const PAYLOAD_VERSION_2: u8 = PAYLOAD_VERSION_1 + 1;

        fn versioned_entry(version: u8, content: &[u8]) -> LocationEntry {
            LocationEntry {
                payload: LocationPayload {
                    version,
                    content: content.to_vec().into(),
                },
                checked_at: 1234,
                attestation: None,
            }
        }

        #[test]
        fn a_client_recomputes_the_digest_from_the_query_surface_alone() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            test.add_dummy_agent();

            for node_id in [1, 2, 3] {
                test.set_dummy_measurement_from(node_id, &agent);
                test.set_dummy_node_self_declared(node_id);
                test.set_dummy_node_override(node_id);
            }

            // a store holding two payload versions at once, which is the state any version
            // rollout passes through
            test.set_node_entry(
                4,
                measured_by(&agent),
                versioned_entry(PAYLOAD_VERSION_2, b"v2-content"),
            );
            test.set_node_entry(
                5,
                Source::Override,
                versioned_entry(PAYLOAD_VERSION_1, b"v1-content"),
            );

            // ---- from here on, only what a client can reach without trusting the node ----

            let mut pulled = Vec::new();
            let mut start_after = None;
            let mut pages = 0;
            loop {
                assert!(pages < 20, "the enumeration did not terminate");
                pages += 1;

                // small enough that the enumeration has to page, and to cross from the location
                // entries into the whitelist mid-page
                let page: AllRecordsPagedResponse = test.query(&QueryMsg::AllRecords {
                    start_after,
                    limit: Some(4),
                })?;
                pulled.extend(page.records);

                let Some(cursor) = page.start_next_after else {
                    break;
                };
                start_after = Some(cursor);
            }

            // 11 location entries and 2 agents, so a truncated pull is legible as that rather
            // than only as a digest mismatch
            assert_eq!(pulled.len(), 13);
            assert!(pages > 1, "the fixture is too small to have paged at all");
            assert_eq!(
                pulled
                    .iter()
                    .filter(|record| matches!(
                        record,
                        GeolocationRecord::Location(location)
                            if location.entry.payload.version == PAYLOAD_VERSION_2
                    ))
                    .count(),
                1,
                "the unreadable payload version must survive the enumeration verbatim"
            );

            let mut recomputed = LtHash16::new();
            for record in &pulled {
                recomputed.add(&record.digest_leaf());
            }

            // the accumulator is what an ICS23 proof covers, so a client that wants proof
            // compares these directly and never has to reproduce the collapse at all
            let proven = test.must_read_from_contract_storage(
                test.contract_address.clone(),
                storage_keys::DIGEST_STATE,
            )?;
            let proven: [u8; DIGEST_LEN] = proven
                .as_slice()
                .try_into()
                .expect("the digest key must hold a whole accumulator");
            assert_eq!(recomputed, LtHash16::from_bytes(&proven));

            // and the smart query serves the collapse of that same value, for clients that only
            // need to compare digests and can live without a proof
            let digest: DigestResponse = test.query(&QueryMsg::Digest {})?;
            assert_eq!(digest.digest.to_vec(), recomputed.out().to_vec());

            Ok(())
        }
    }
}
