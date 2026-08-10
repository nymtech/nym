// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
use cosmwasm_std::{Addr, Api, Deps, DepsMut, Env, Event, MessageInfo, Response, Storage};
use nym_geolocation_contract_common::constants::events;
use nym_geolocation_contract_common::{
    AgentPermissions, EntryKey, GeolocationContractError, LocationEntry, LocationPayload,
    Measurement, NymNodeLocation, Source, Subject,
};
use nym_mixnet_contract_common::{MixnetContractQuerier, NodeId};
use std::collections::BTreeSet;

/// Store a batch of third-party measurements.
///
/// The measuring agent is the message sender rather than a field on [`Measurement`], so an
/// agent structurally cannot write under another agent's key, and each agent keeps its own
/// slot per subject instead of overwriting its peers.
///
/// One accumulator load and save covers the whole batch, while each entry still does its own
/// read-modify-write so a replacement subtracts the exact leaf it retires - including when the
/// same subject appears twice in one batch, where the later write wins. LtHash is commutative,
/// so the order entries arrive in does not affect the resulting digest and none is required.
///
/// Every check runs before any write. That makes the all-or-nothing guarantee structural
/// rather than something resting on the transaction rolling back: there is no point at which
/// some of a rejected batch has been persisted.
pub fn try_submit_measurements(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    measurements: Vec<Measurement>,
) -> Result<Response, GeolocationContractError> {
    let agent = info.sender;
    let storage = GEOLOCATION_CONTRACT_STORAGE;

    let permissions = storage.must_load_agent_permissions(deps.storage, &agent)?;
    if !permissions.can_measure {
        return Err(GeolocationContractError::MissingAgentPermission {
            agent,
            permission: "can_measure",
        });
    }

    let config = storage.config.load(deps.storage)?;
    if measurements.len() > config.max_batch_size as usize {
        return Err(GeolocationContractError::BatchTooLarge {
            size: measurements.len(),
            max: config.max_batch_size,
        });
    }

    // the size bound is the only validation possible on a payload the contract never parses,
    // and it is what keeps a buggy agent's damage a bad value rather than state bloat and an
    // inflated recompute for every verifying client
    for measurement in &measurements {
        measurement
            .payload
            .ensure_within_size_limit(config.max_payload_size)?;
    }

    let submitted = measurements.len();
    let checked_at = env.block.time.seconds();

    GEOLOCATION_CONTRACT_STORAGE.set_entries(
        deps.storage,
        measurements.into_iter().map(|measurement| {
            (
                measurement.subject,
                Source::Measured {
                    method: measurement.method,
                    agent: agent.clone(),
                },
                LocationEntry {
                    payload: measurement.payload,
                    checked_at,
                    // measured entries carry no signature from anyone; only a subject's own
                    // relayed declaration does
                    attestation: None,
                },
            )
        }),
    )?;

    Ok(Response::new().add_event(
        Event::new(events::SUBMIT_MEASUREMENTS)
            .add_attribute(events::ATTR_AGENT, agent.as_str())
            .add_attribute(events::ATTR_COUNT, submitted.to_string()),
    ))
}

/// Fetch `node_id`'s ed25519 identity public key (32 raw bytes) from its mixnet bond.
///
/// Requires the node to be bonded and not unbonding. Rejecting an unbonding node is not
/// pedantry: the unbond callback is about to delete every entry for that subject, so accepting
/// a declaration for it would add a leaf that is immediately removed again.
fn bonded_node_identity_key(
    deps: Deps<'_>,
    node_id: NodeId,
) -> Result<Vec<u8>, GeolocationContractError> {
    let mixnet = GEOLOCATION_CONTRACT_STORAGE
        .mixnet_contract_address
        .load(deps.storage)?;
    let bond = deps
        .querier
        .query_nymnode_bond(&mixnet, node_id)?
        .ok_or(GeolocationContractError::NodeNotBonded { node_id })?;
    if bond.is_unbonding {
        return Err(GeolocationContractError::NodeNotBonded { node_id });
    }

    let key = bs58::decode(bond.identity())
        .into_vec()
        .map_err(|_| GeolocationContractError::InvalidIdentityKey { node_id })?;
    if key.len() != 32 {
        return Err(GeolocationContractError::InvalidIdentityKey { node_id });
    }
    Ok(key)
}

/// Reject a declaration that cannot supersede what is stored, or that is stamped further ahead
/// than the configured skew allows.
///
/// There is deliberately no lower bound: monotonicity already governs the past, so a node whose
/// clock lags simply advances from wherever it starts. The upper bound is load-bearing, because
/// one artifact stamped years ahead would freeze the slot permanently - nothing could ever
/// exceed it.
fn ensure_declaration_supersedes_stored(
    store: &dyn Storage,
    node_id: NodeId,
    declared_at: u64,
    block_time: u64,
    max_skew_secs: u64,
) -> Result<(), GeolocationContractError> {
    if declared_at > block_time.saturating_add(max_skew_secs) {
        return Err(GeolocationContractError::DeclarationTooFarInFuture {
            node_id,
            declared_at,
            block_time,
            max_skew_secs,
        });
    }

    let stored = GEOLOCATION_CONTRACT_STORAGE
        .may_load_entry(
            store,
            &Subject::new_nym_node(node_id),
            &Source::SelfDeclared,
        )?
        .and_then(|entry| entry.attestation)
        .map(|attestation| attestation.declared_at);

    // strictly greater, so re-relaying the current artifact is a replay rather than a
    // heartbeat: unlike a measurement, an unchanged self-declaration cannot refresh
    // `checked_at`, because the node would have to sign a new one to say so
    if let Some(stored) = stored {
        if declared_at <= stored {
            return Err(GeolocationContractError::StaleDeclaration {
                node_id,
                declared_at,
                stored,
            });
        }
    }

    Ok(())
}

/// Verify a declaration against `identity_key`, over the bytes the artifact itself produces so
/// a verifier cannot check the signature against anything other than what it holds. A verifier
/// error and a failed check are the same rejection.
fn verify_self_declaration(
    api: &dyn Api,
    declaration: &NymNodeLocation,
    identity_key: &[u8],
) -> Result<(), GeolocationContractError> {
    let verified = api
        .ed25519_verify(
            &declaration.signing_payload(),
            declaration.signature.as_slice(),
            identity_key,
        )
        .map_err(|_| GeolocationContractError::InvalidSignature)?;

    if !verified {
        return Err(GeolocationContractError::InvalidSignature);
    }
    Ok(())
}

/// Relay a batch of node-signed self-declarations.
///
/// The relaying agent is a courier rather than a witness, so it appears nowhere in the key: a
/// subject has exactly one self-declared slot however many agents relay for it, and conflicts
/// resolve by `declared_at` rather than by who submitted last. What the agent's permission buys
/// is the right to spend the contract's gas on someone else's data, not any authority over it.
///
/// The payload is stored exactly as received, which is the whole point of this path. The
/// signature is over those bytes, so anything that parsed and re-emitted them - reordering JSON
/// keys, reformatting a float - would leave an entry that no longer verifies against its own
/// stored content.
///
/// Kept a separate message from [`try_submit_measurements`] because an agent cannot fully
/// pre-validate what it relays: it did not produce the artifact and cannot check monotonicity
/// against state it does not hold. One bad artifact must not be able to fail a measurement
/// sweep, so the two never share a batch.
pub fn try_relay_self_declarations(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    declarations: Vec<NymNodeLocation>,
) -> Result<Response, GeolocationContractError> {
    let agent = info.sender;
    let storage = GEOLOCATION_CONTRACT_STORAGE;

    let permissions = storage.must_load_agent_permissions(deps.storage, &agent)?;
    if !permissions.can_relay_self_declared {
        return Err(GeolocationContractError::MissingAgentPermission {
            agent,
            permission: "can_relay_self_declared",
        });
    }

    let config = storage.config.load(deps.storage)?;
    if declarations.len() > config.max_batch_size as usize {
        return Err(GeolocationContractError::BatchTooLarge {
            size: declarations.len(),
            max: config.max_batch_size,
        });
    }

    // as with measurements, every check runs before any write, so a rejected batch has never
    // been partially persisted. Checks are ordered cheapest first: the bond lookup is a
    // cross-contract query and the signature check is the expensive one, so a replayed or
    // skewed artifact is thrown out without paying for either
    let block_time = env.block.time.seconds();
    let mut seen = BTreeSet::new();
    for declaration in &declarations {
        let node_id = declaration.node_id;
        if !seen.insert(node_id) {
            return Err(GeolocationContractError::DuplicateDeclaration { node_id });
        }

        declaration
            .payload
            .ensure_within_size_limit(config.max_payload_size)?;
        ensure_declaration_supersedes_stored(
            deps.storage,
            node_id,
            declaration.declared_at,
            block_time,
            config.max_skew_secs,
        )?;

        let identity_key = bonded_node_identity_key(deps.as_ref(), node_id)?;
        verify_self_declaration(deps.api, declaration, &identity_key)?;
    }

    let relayed = declarations.len();
    GEOLOCATION_CONTRACT_STORAGE.set_entries(
        deps.storage,
        declarations.into_iter().map(|declaration| {
            let subject = Subject::new_nym_node(declaration.node_id);
            (
                subject,
                Source::SelfDeclared,
                declaration.into_entry(block_time),
            )
        }),
    )?;

    Ok(Response::new().add_event(
        Event::new(events::RELAY_SELF_DECLARATIONS)
            .add_attribute(events::ATTR_AGENT, agent.as_str())
            .add_attribute(events::ATTR_COUNT, relayed.to_string()),
    ))
}

/// Reject a sender that is not the current admin.
fn ensure_admin(deps: Deps<'_>, sender: &Addr) -> Result<(), GeolocationContractError> {
    GEOLOCATION_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps, sender)?;
    Ok(())
}

/// Create or replace an admin override for a subject.
///
/// The [`Source::Override`] slot names the admin *role* rather than the address that wrote it,
/// so transferring the role leaves existing overrides readable under the same key instead of
/// orphaning them.
///
/// The contract applies no precedence between an override and the other sources; it simply
/// stores one more opinion, and choosing between them is the client's job. That is why setting
/// an override neither suppresses nor deletes anything else for that subject.
///
/// The subject is deliberately not checked against the mixnet contract. The override is the
/// admin's escape hatch, the subject enum is meant to outgrow bonded nym-nodes, and a bonding
/// check would only apply to one class.
pub fn try_set_override(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    subject: Subject,
    payload: LocationPayload,
) -> Result<Response, GeolocationContractError> {
    ensure_admin(deps.as_ref(), &info.sender)?;
    let storage = GEOLOCATION_CONTRACT_STORAGE;

    let config = storage.config.load(deps.storage)?;
    payload.ensure_within_size_limit(config.max_payload_size)?;

    let event =
        Event::new(events::SET_OVERRIDE).add_attribute(events::ATTR_SUBJECT, subject.to_string());

    storage.set_entry(
        deps.storage,
        subject,
        Source::Override,
        LocationEntry {
            payload,
            checked_at: env.block.time.seconds(),
            // an override is the admin asserting a location, not the subject attesting to one
            attestation: None,
        },
    )?;

    Ok(Response::new().add_event(event))
}

/// Delete a subject's override, leaving every other source for that subject untouched.
///
/// A separate operation from setting one, so an override can be retracted immediately rather
/// than waiting for the subject to be re-measured. Idempotent: removing an override that is not
/// there leaves the digest untouched and still succeeds.
pub fn try_remove_override(
    deps: DepsMut<'_>,
    info: MessageInfo,
    subject: Subject,
) -> Result<Response, GeolocationContractError> {
    ensure_admin(deps.as_ref(), &info.sender)?;

    GEOLOCATION_CONTRACT_STORAGE.remove_entry(deps.storage, &subject, &Source::Override)?;

    Ok(Response::new().add_event(
        Event::new(events::REMOVE_OVERRIDE)
            .add_attribute(events::ATTR_SUBJECT, subject.to_string()),
    ))
}

/// Delete a batch of entries by explicit key.
///
/// Explicit keys rather than a purge scoped to one agent, which is what the de-whitelisting
/// story nominally wants. The agent sits inside the trailing `source` key component, so nothing
/// indexes by it: a scoped purge would scan the entire store to find one agent's share of it,
/// once per page, with every page a separate admin transaction. Here the admin pages the
/// enumeration off-chain, decides exactly what should go, and names it, so the cost is
/// proportional to what is deleted rather than to what is stored.
///
/// It also reaches entries no agent-scoped sweep could. Measurements deliberately do not check
/// the mixnet contract, so an agent may write for a subject that was never bonded, and the
/// unbond callback will never fire for it. Without this, such an entry is permanent.
///
/// Only location entries. Whitelist entries are digest-committed too but must go through
/// [`try_remove_whitelisted_agent`], where removal and its authorisation meaning stay together.
pub fn try_remove_entries(
    deps: DepsMut<'_>,
    info: MessageInfo,
    keys: Vec<EntryKey>,
) -> Result<Response, GeolocationContractError> {
    ensure_admin(deps.as_ref(), &info.sender)?;
    let storage = GEOLOCATION_CONTRACT_STORAGE;

    let config = storage.config.load(deps.storage)?;
    if keys.len() > config.max_batch_size as usize {
        return Err(GeolocationContractError::BatchTooLarge {
            size: keys.len(),
            max: config.max_batch_size,
        });
    }

    // naming a key that holds nothing is not an error: the admin works from an enumeration it
    // pulled at some earlier height, and an entry it names may have been overwritten or already
    // removed since. Failing the batch for that would make a purge a race against the agents
    let named = keys.len();
    storage.remove_entries(deps.storage, keys)?;

    Ok(Response::new().add_event(
        Event::new(events::REMOVE_ENTRIES).add_attribute(events::ATTR_COUNT, named.to_string()),
    ))
}

/// Add an agent to the whitelist, or replace an existing agent's permissions.
///
/// Both are the same operation because the whitelist is a map: granting and re-granting differ
/// only in whether a leaf has to be retired first, which the digest wrapper handles.
pub fn try_set_whitelisted_agent(
    deps: DepsMut<'_>,
    info: MessageInfo,
    agent: String,
    permissions: AgentPermissions,
) -> Result<Response, GeolocationContractError> {
    ensure_admin(deps.as_ref(), &info.sender)?;
    let agent = deps.api.addr_validate(&agent)?;

    let event = Event::new(events::SET_WHITELISTED_AGENT)
        .add_attribute(events::ATTR_AGENT, agent.as_str())
        .add_attribute(
            events::ATTR_CAN_MEASURE,
            permissions.can_measure.to_string(),
        )
        .add_attribute(
            events::ATTR_CAN_RELAY_SELF_DECLARED,
            permissions.can_relay_self_declared.to_string(),
        );

    GEOLOCATION_CONTRACT_STORAGE.set_whitelisted_agent(deps.storage, agent, permissions)?;

    Ok(Response::new().add_event(event))
}

/// Remove an agent from the whitelist.
///
/// Deliberately non-destructive: the agent's entries stay in storage and in the digest. Because
/// authorisation is evaluated at read time, a conforming client stops honouring them from this
/// block, which is what makes compromise recovery instant and free. Reclaiming the space is a
/// separate, paginated admin operation, and is hygiene rather than the security control.
pub fn try_remove_whitelisted_agent(
    deps: DepsMut<'_>,
    info: MessageInfo,
    agent: String,
) -> Result<Response, GeolocationContractError> {
    ensure_admin(deps.as_ref(), &info.sender)?;
    let agent = deps.api.addr_validate(&agent)?;

    GEOLOCATION_CONTRACT_STORAGE.remove_whitelisted_agent(deps.storage, &agent)?;

    Ok(Response::new().add_event(
        Event::new(events::REMOVE_WHITELISTED_AGENT)
            .add_attribute(events::ATTR_AGENT, agent.as_str()),
    ))
}

/// Cross-contract callback fired by the mixnet contract when a node unbonds: delete everything
/// held for that subject, folding each removal into the digest.
///
/// Every source goes, the admin's override included. The subject itself has ceased to exist, so
/// nothing anyone asserted about where it was remains meaningful, and leaving an override behind
/// would resurrect a node that is no longer bonded for any client reading by subject.
///
/// Only the configured mixnet contract may call it. Without that check any address could clear
/// a live node's entries, which is a denial of service against a node that has done nothing.
///
/// Idempotent, and deliberately so rather than by accident: the callback fires per unbond, and a
/// node with nothing stored is the common case rather than an error.
pub(crate) fn try_handle_node_unbonding(
    deps: DepsMut,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, GeolocationContractError> {
    let storage = GEOLOCATION_CONTRACT_STORAGE;
    let mixnet_contract = storage.mixnet_contract_address.load(deps.storage)?;
    if info.sender != mixnet_contract {
        return Err(GeolocationContractError::UnauthorisedMixnetCallback {
            sender: info.sender,
        });
    }

    let subject = Subject::new_nym_node(node_id);
    let removed = storage.remove_all_entries_for_subject(deps.storage, &subject)?;

    Ok(Response::new().add_event(
        Event::new(events::ON_NYM_NODE_UNBOND)
            .add_attribute(events::ATTR_SUBJECT, subject.to_string())
            .add_attribute(events::ATTR_COUNT, removed.to_string()),
    ))
}

/// Change the contract's tunables. Omitted fields keep their current value.
///
/// Every one of these can need to move without a redeploy: a later payload version may want
/// more room or less, gas costs shift, and clock tolerance is an operational judgement. That is
/// the whole reason they live in state rather than in constants.
///
/// The result is validated as a whole rather than field by field, so a partial update cannot
/// reach a configuration that instantiation would have refused.
///
/// Lowering a bound is not retroactive. Entries already stored under a larger
/// `max_payload_size` stay readable and stay in the digest; the new bound governs the next
/// write only. Shrinking the stored set is [`try_remove_entries`]'s job.
pub fn try_update_config(
    deps: DepsMut<'_>,
    info: MessageInfo,
    max_skew_secs: Option<u64>,
    max_batch_size: Option<u32>,
    max_payload_size: Option<u32>,
) -> Result<Response, GeolocationContractError> {
    ensure_admin(deps.as_ref(), &info.sender)?;
    let storage = GEOLOCATION_CONTRACT_STORAGE;

    let mut config = storage.config.load(deps.storage)?;
    if let Some(max_skew_secs) = max_skew_secs {
        config.max_skew_secs = max_skew_secs;
    }
    if let Some(max_batch_size) = max_batch_size {
        config.max_batch_size = max_batch_size;
    }
    if let Some(max_payload_size) = max_payload_size {
        config.max_payload_size = max_payload_size;
    }

    config.validate()?;
    storage.config.save(deps.storage, &config)?;

    Ok(Response::new().add_event(
        Event::new(events::UPDATE_CONFIG)
            .add_attribute(events::ATTR_MAX_SKEW_SECS, config.max_skew_secs.to_string())
            .add_attribute(
                events::ATTR_MAX_BATCH_SIZE,
                config.max_batch_size.to_string(),
            )
            .add_attribute(
                events::ATTR_MAX_PAYLOAD_SIZE,
                config.max_payload_size.to_string(),
            ),
    ))
}

pub fn try_update_contract_admin(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, GeolocationContractError> {
    let new_admin = deps.api.addr_validate(&new_admin)?;

    let res = GEOLOCATION_CONTRACT_STORAGE
        .contract_admin
        .execute_update_admin(deps, info, Some(new_admin))?;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod measurement_submission {
        use super::*;
        use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
        use crate::testing::{
            init_contract_tester, measured_by, node_measurement, GeolocationContractTesterExt,
        };
        use cosmwasm_std::testing::message_info;
        use cosmwasm_std::Addr;
        use nym_contracts_common_testing::{ChainOpts, ContractOpts, RandExt};
        use nym_geolocation_contract_common::constants::DEFAULT_MAX_PAYLOAD_SIZE;
        use nym_geolocation_contract_common::{AgentPermissions, ContractConfig, Subject};

        /// Submit `measurements` as `agent`, returning whatever the handler returned.
        fn submit(
            test: &mut impl GeolocationContractTesterExt,
            agent: &Addr,
            measurements: Vec<Measurement>,
        ) -> Result<Response, GeolocationContractError> {
            let env = test.env();
            try_submit_measurements(test.deps_mut(), env, message_info(agent, &[]), measurements)
        }

        #[test]
        fn a_batch_is_written_under_the_senders_own_agent_key() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            submit(
                &mut test,
                &agent,
                vec![
                    node_measurement(1, b"first"),
                    node_measurement(2, b"second"),
                ],
            )
            .unwrap();

            // the agent comes from `info.sender`, never from the message body, so an agent
            // cannot write into another agent's slot even deliberately
            let source = measured_by(&agent);
            assert_eq!(
                test.node_entry(1, &source).unwrap().payload.content,
                b"first".to_vec()
            );
            assert_eq!(
                test.node_entry(2, &source).unwrap().payload.content,
                b"second".to_vec()
            );
            test.assert_digest_is_refold();
        }

        #[test]
        fn payload_bytes_are_stored_verbatim() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            // deliberately neither valid UTF-8 nor valid JSON: the contract never parses a
            // payload, and a relayed self-declaration's signature is over exactly these bytes
            let content = [0x00, 0xff, 0x7b, 0x22, 0x80, 0x41];
            submit(&mut test, &agent, vec![node_measurement(1, &content)]).unwrap();

            let entry = test.measurement_by(1, &agent).unwrap();
            assert_eq!(entry.payload.content.as_slice(), content);
            // measured entries carry nobody's signature; only a relayed declaration does
            assert_eq!(entry.attestation, None);
        }

        #[test]
        fn checked_at_comes_from_block_time_not_from_the_agent() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            test.advance_time_by(12_345);
            let block_time = test.env().block.time.seconds();
            submit(&mut test, &agent, vec![node_measurement(1, b"here")]).unwrap();

            // `checked_at` is what makes freshness provable, so it cannot be something the
            // submitter chooses
            assert_eq!(
                test.measurement_by(1, &agent).unwrap().checked_at,
                block_time
            );
        }

        #[test]
        fn concurrent_agents_keep_separate_slots_for_the_same_subject() {
            let mut test = init_contract_tester();
            let first = test.add_dummy_agent();
            let second = test.add_dummy_agent();

            submit(&mut test, &first, vec![node_measurement(1, b"from-first")]).unwrap();
            submit(
                &mut test,
                &second,
                vec![node_measurement(1, b"from-second")],
            )
            .unwrap();

            // disagreement is meant to be visible rather than collapsed: whoever wrote last
            // must not have overwritten the other's answer
            assert_eq!(
                test.measurement_by(1, &first)
                    .unwrap()
                    .payload
                    .content,
                b"from-first".to_vec()
            );
            assert_eq!(
                test.measurement_by(1, &second)
                    .unwrap()
                    .payload
                    .content,
                b"from-second".to_vec()
            );
            assert_eq!(test.node_measurements(1).len(), 2);
            test.assert_digest_is_refold();
        }

        #[test]
        fn a_subject_repeated_within_one_batch_resolves_to_the_last_write() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            submit(
                &mut test,
                &agent,
                vec![node_measurement(1, b"stale"), node_measurement(1, b"fresh")],
            )
            .unwrap();

            assert_eq!(
                test.measurement_by(1, &agent)
                    .unwrap()
                    .payload
                    .content,
                b"fresh".to_vec()
            );
            // the superseded leaf was retired inside the batch rather than left summed in
            test.assert_digest_is_refold();
        }

        #[test]
        fn resubmitting_an_unchanged_location_advances_checked_at_and_moves_the_digest() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            submit(&mut test, &agent, vec![node_measurement(1, b"unchanged")]).unwrap();
            let before = test.digest();
            let first_checked_at = test.measurement_by(1, &agent).unwrap().checked_at;

            test.advance_time_by(60);
            submit(&mut test, &agent, vec![node_measurement(1, b"unchanged")]).unwrap();

            // the whole point of the heartbeat: an agent re-submitting an unchanged location
            // has to move the digest, otherwise verifying it would say nothing about freshness
            let entry = test.measurement_by(1, &agent).unwrap();
            assert_eq!(entry.payload.content, b"unchanged".to_vec());
            assert_eq!(entry.checked_at, first_checked_at + 60);
            assert_ne!(test.digest(), before);
            test.assert_digest_is_refold();
        }

        #[test]
        fn a_non_whitelisted_sender_is_rejected() {
            let mut test = init_contract_tester();
            let stranger = test.generate_account();

            let err = submit(&mut test, &stranger, vec![node_measurement(1, b"x")]).unwrap_err();

            assert_eq!(
                err,
                GeolocationContractError::NotWhitelisted {
                    agent: stranger.clone()
                }
            );
            assert!(test.measurement_by(1, &stranger).is_none());
        }

        #[test]
        fn a_de_whitelisted_agent_stops_being_able_to_write() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            submit(&mut test, &agent, vec![node_measurement(1, b"before")]).unwrap();

            test.remove_agent(&agent);

            // membership is read on every write, so revocation needs nothing invalidated and
            // nothing enumerated - it simply takes effect
            let err = submit(&mut test, &agent, vec![node_measurement(1, b"after")]).unwrap_err();
            assert_eq!(
                err,
                GeolocationContractError::NotWhitelisted {
                    agent: agent.clone()
                }
            );

            // and the already-written entry survives: de-whitelisting neutralises it for
            // readers rather than deleting it
            assert_eq!(
                test.measurement_by(1, &agent)
                    .unwrap()
                    .payload
                    .content,
                b"before".to_vec()
            );
        }

        #[test]
        fn an_agent_without_can_measure_is_rejected() {
            let mut test = init_contract_tester();
            let relay_only = test.add_agent_with_permissions(AgentPermissions {
                can_measure: false,
                can_relay_self_declared: true,
            });

            let err = submit(&mut test, &relay_only, vec![node_measurement(1, b"x")]).unwrap_err();

            // distinct from `NotWhitelisted`: this agent was authorised, just not for this
            assert_eq!(
                err,
                GeolocationContractError::MissingAgentPermission {
                    agent: relay_only,
                    permission: "can_measure"
                }
            );
        }

        #[test]
        fn a_batch_over_the_configured_maximum_is_rejected_whole() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            let max = GEOLOCATION_CONTRACT_STORAGE
                .config
                .load(&test)
                .unwrap()
                .max_batch_size;
            let oversized = (0..=max)
                .map(|node_id| node_measurement(node_id, b"x"))
                .collect::<Vec<_>>();

            let err = submit(&mut test, &agent, oversized).unwrap_err();
            assert_eq!(
                err,
                GeolocationContractError::BatchTooLarge {
                    size: max as usize + 1,
                    max
                }
            );

            // rejected before anything was written, so all-or-nothing does not rest on the
            // transaction rolling back
            assert_eq!(
                test.all_records().len(),
                1,
                "only the whitelisted agent itself should be stored"
            );
        }

        #[test]
        fn a_batch_at_exactly_the_configured_maximum_is_accepted() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            let max = GEOLOCATION_CONTRACT_STORAGE
                .config
                .load(&test)
                .unwrap()
                .max_batch_size;
            let full = (0..max)
                .map(|node_id| node_measurement(node_id, b"x"))
                .collect::<Vec<_>>();

            // the bound is inclusive, so an agent filling a batch exactly is not punished
            submit(&mut test, &agent, full).unwrap();
            assert_eq!(test.node_measurements(max - 1).len(), 1);
            test.assert_digest_is_refold();
        }

        #[test]
        fn one_oversized_payload_fails_the_whole_batch_without_writing_anything() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            let oversized = vec![0u8; DEFAULT_MAX_PAYLOAD_SIZE as usize + 1];
            let err = submit(
                &mut test,
                &agent,
                vec![
                    node_measurement(1, b"fine"),
                    node_measurement(2, &oversized),
                    node_measurement(3, b"also fine"),
                ],
            )
            .unwrap_err();

            assert_eq!(
                err,
                GeolocationContractError::PayloadTooLarge {
                    len: oversized.len(),
                    max: DEFAULT_MAX_PAYLOAD_SIZE
                }
            );

            // the valid entries either side of the bad one are not written: every check runs
            // before any write, so nothing has to be rolled back
            for node_id in [1, 2, 3] {
                assert!(test.measurement_by(node_id, &agent).is_none());
            }
            test.assert_digest_is_refold();
        }

        #[test]
        fn the_payload_bound_is_read_from_state_not_from_the_constant() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            // a later payload version may need more room, or less, which is why the bound is
            // admin-adjustable state rather than a redeploy
            GEOLOCATION_CONTRACT_STORAGE
                .config
                .save(
                    &mut test,
                    &ContractConfig {
                        max_skew_secs: 300,
                        max_batch_size: 50,
                        max_payload_size: 4,
                    },
                )
                .unwrap();

            assert!(submit(&mut test, &agent, vec![node_measurement(1, b"12345")]).is_err());
            submit(&mut test, &agent, vec![node_measurement(1, b"1234")]).unwrap();
        }

        #[test]
        fn the_emitted_event_names_the_agent_and_the_batch_size() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();

            let res = submit(
                &mut test,
                &agent,
                vec![node_measurement(1, b"a"), node_measurement(2, b"b")],
            )
            .unwrap();

            // one event for the batch rather than one per entry, which at `MAX_BATCH_SIZE`
            // would swamp the block for data that is queryable anyway
            assert_eq!(res.events.len(), 1);
            let event = &res.events[0];
            assert_eq!(event.ty, events::SUBMIT_MEASUREMENTS);
            assert!(event
                .attributes
                .iter()
                .any(|attr| attr.key == events::ATTR_AGENT && attr.value == agent.as_str()));
            assert!(event
                .attributes
                .iter()
                .any(|attr| attr.key == events::ATTR_COUNT && attr.value == "2"));
        }

        #[test]
        fn a_measurement_never_carries_an_attestation() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            submit(&mut test, &agent, vec![node_measurement(1, b"x")]).unwrap();

            let entries = GEOLOCATION_CONTRACT_STORAGE
                .subject_entries(&test, &Subject::new_nym_node(1))
                .unwrap();
            assert!(entries.iter().all(|(_, entry)| entry.attestation.is_none()));
        }
    }

    #[cfg(test)]
    mod self_declaration_relay {
        use super::*;
        use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
        use crate::testing::{
            init_contract_tester, signed_declaration, GeolocationContractTesterExt,
        };
        use cosmwasm_std::testing::message_info;
        use cosmwasm_std::Addr;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::{ChainOpts, RandExt};
        use nym_crypto::asymmetric::ed25519;
        use nym_geolocation_contract_common::constants::{
            DEFAULT_MAX_PAYLOAD_SIZE, DEFAULT_MAX_SKEW_SECS,
        };
        use nym_geolocation_contract_common::AgentPermissions;
        use nym_mixnet_contract_common::ExecuteMsg as MixnetExecuteMsg;

        /// A tester holding one whitelisted relay agent and one bonded node whose identity
        /// keypair the caller can sign with.
        fn relay_setup() -> (
            impl GeolocationContractTesterExt,
            Addr,
            NodeId,
            ed25519::KeyPair,
        ) {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            let (node_id, keypair) = test.bond_dummy_nymnode_with_keypair().unwrap();
            (test, agent, node_id, keypair)
        }

        fn relay(
            test: &mut impl GeolocationContractTesterExt,
            agent: &Addr,
            declarations: Vec<NymNodeLocation>,
        ) -> Result<Response, GeolocationContractError> {
            let env = test.env();
            try_relay_self_declarations(
                test.deps_mut(),
                env,
                message_info(agent, &[]),
                declarations,
            )
        }

        /// A `declared_at` comfortably inside the skew window, so tests that are not about the
        /// clock do not accidentally depend on it.
        fn now(test: &impl GeolocationContractTesterExt) -> u64 {
            test.env().block.time.seconds()
        }

        #[test]
        fn a_node_signed_declaration_is_stored_with_its_attestation() {
            let (mut test, agent, node_id, keypair) = relay_setup();
            let declared_at = now(&test);
            let declaration = signed_declaration(&keypair, node_id, declared_at, b"declared");

            relay(&mut test, &agent, vec![declaration.clone()]).unwrap();

            let entry = test.node_entry(node_id, &Source::SelfDeclared).unwrap();
            // stored verbatim: the signature is over exactly these bytes, so anything that
            // parsed and re-emitted them would leave an entry that no longer verifies
            assert_eq!(entry.payload, declaration.payload);
            assert_eq!(entry.checked_at, declared_at);
            let attestation = entry.attestation.unwrap();
            assert_eq!(attestation.declared_at, declared_at);
            assert_eq!(attestation.signature, declaration.signature);
            test.assert_digest_is_refold();
        }

        #[test]
        fn the_relaying_agent_appears_nowhere_in_the_key() {
            let (mut test, first, node_id, keypair) = relay_setup();
            let second = test.add_dummy_agent();

            relay(
                &mut test,
                &first,
                vec![signed_declaration(&keypair, node_id, 1_000, b"first relay")],
            )
            .unwrap();
            relay(
                &mut test,
                &second,
                vec![signed_declaration(
                    &keypair,
                    node_id,
                    2_000,
                    b"second relay",
                )],
            )
            .unwrap();

            // one self-declared slot per subject however many agents relay for it: the agent is
            // a courier, not a witness, so conflicts resolve by `declared_at` rather than by
            // who submitted last
            let entries = test.node_entries(node_id);
            assert_eq!(
                entries
                    .iter()
                    .filter(|(source, _)| matches!(source, Source::SelfDeclared))
                    .count(),
                1
            );
            assert_eq!(
                test.node_entry(node_id, &Source::SelfDeclared)
                    .unwrap()
                    .payload
                    .content,
                b"second relay".to_vec()
            );
        }

        #[test]
        fn a_forged_signature_is_rejected() {
            let (mut test, agent, node_id, _) = relay_setup();
            let attacker = ed25519::KeyPair::new(test.raw_rng());
            let declared_at = now(&test);

            // signed correctly, just not by the node whose location it claims to be
            let forged = signed_declaration(&attacker, node_id, declared_at, b"somewhere else");

            assert_eq!(
                relay(&mut test, &agent, vec![forged]).unwrap_err(),
                GeolocationContractError::InvalidSignature
            );
            assert!(test.node_entry(node_id, &Source::SelfDeclared).is_none());
        }

        #[test]
        fn tampering_with_any_signed_field_is_rejected() {
            let (mut test, agent, node_id, keypair) = relay_setup();
            let declared_at = now(&test);
            let genuine = signed_declaration(&keypair, node_id, declared_at, b"declared");

            let mut moved = genuine.clone();
            moved.payload.content = b"elsewhere".to_vec().into();

            let mut restamped = genuine.clone();
            restamped.declared_at = declared_at - 1;

            // the version is signed too, so a relayer cannot restate v1-signed content as v2
            // and thereby choose which format consumers read those bytes as
            let mut reversioned = genuine;
            reversioned.payload.version += 1;

            for tampered in [moved, restamped, reversioned] {
                assert_eq!(
                    relay(&mut test, &agent, vec![tampered]).unwrap_err(),
                    GeolocationContractError::InvalidSignature
                );
            }
        }

        #[test]
        fn a_declaration_for_an_unbonded_node_is_rejected() {
            let (mut test, agent, _, keypair) = relay_setup();
            let never_bonded = 12_345;
            let declared_at = now(&test);

            assert_eq!(
                relay(
                    &mut test,
                    &agent,
                    vec![signed_declaration(
                        &keypair,
                        never_bonded,
                        declared_at,
                        b"x"
                    )]
                )
                .unwrap_err(),
                GeolocationContractError::NodeNotBonded {
                    node_id: never_bonded
                }
            );
        }

        #[test]
        fn a_declaration_for_an_unbonding_node_is_rejected() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            let owner = test.generate_account_with_balance();
            let (node_id, keypair) = test.bond_dummy_nymnode_for_with_keypair(&owner).unwrap();

            // deliberately not `unbond_nymnode`, which advances the epoch and removes the bond
            // outright - that is the absent-bond case above. Executing the unbond on its own
            // leaves the bond in place with `is_unbonding` set, which is the state under test
            test.execute_mixnet_contract(
                message_info(&owner, &[]),
                &MixnetExecuteMsg::UnbondNymNode {},
            )
            .unwrap();
            let declared_at = now(&test);

            // its entries are about to be cleared by the unbond callback, so accepting this
            // would only add a leaf that is immediately removed again
            assert_eq!(
                relay(
                    &mut test,
                    &agent,
                    vec![signed_declaration(&keypair, node_id, declared_at, b"x")]
                )
                .unwrap_err(),
                GeolocationContractError::NodeNotBonded { node_id }
            );
        }

        #[test]
        fn replaying_a_superseded_artifact_is_rejected() {
            let (mut test, agent, node_id, keypair) = relay_setup();
            let old = signed_declaration(&keypair, node_id, 1_000, b"old address");
            let new = signed_declaration(&keypair, node_id, 2_000, b"new address");

            relay(&mut test, &agent, vec![old.clone()]).unwrap();
            relay(&mut test, &agent, vec![new]).unwrap();

            // the artifact is genuinely signed and stays valid forever; only monotonicity stops
            // anyone who once saw it from reinstating last year's location
            assert_eq!(
                relay(&mut test, &agent, vec![old]).unwrap_err(),
                GeolocationContractError::StaleDeclaration {
                    node_id,
                    declared_at: 1_000,
                    stored: 2_000
                }
            );
            assert_eq!(
                test.node_entry(node_id, &Source::SelfDeclared)
                    .unwrap()
                    .payload
                    .content,
                b"new address".to_vec()
            );
        }

        #[test]
        fn re_relaying_the_current_artifact_is_rejected_rather_than_treated_as_a_heartbeat() {
            let (mut test, agent, node_id, keypair) = relay_setup();
            let declaration = signed_declaration(&keypair, node_id, 1_000, b"unchanged");

            relay(&mut test, &agent, vec![declaration.clone()]).unwrap();
            let before = test.digest();
            test.advance_time_by(60);

            // strictly greater, not greater-or-equal: unlike a measurement, an unchanged
            // self-declaration cannot refresh `checked_at`, because the node would have to sign
            // a new artifact to say so
            assert_eq!(
                relay(&mut test, &agent, vec![declaration]).unwrap_err(),
                GeolocationContractError::StaleDeclaration {
                    node_id,
                    declared_at: 1_000,
                    stored: 1_000
                }
            );
            assert_eq!(test.digest(), before);
        }

        #[test]
        fn a_declaration_beyond_the_skew_window_is_rejected_but_the_window_itself_is_inclusive() {
            let (mut test, agent, node_id, keypair) = relay_setup();
            let block_time = now(&test);
            let edge = block_time + DEFAULT_MAX_SKEW_SECS;

            // without an upper bound at all, one artifact stamped years ahead would freeze the
            // slot permanently, since nothing could ever exceed it
            assert_eq!(
                relay(
                    &mut test,
                    &agent,
                    vec![signed_declaration(&keypair, node_id, edge + 1, b"x")]
                )
                .unwrap_err(),
                GeolocationContractError::DeclarationTooFarInFuture {
                    node_id,
                    declared_at: edge + 1,
                    block_time,
                    max_skew_secs: DEFAULT_MAX_SKEW_SECS
                }
            );

            // a node exactly at the edge of the tolerance is not punished for it
            relay(
                &mut test,
                &agent,
                vec![signed_declaration(&keypair, node_id, edge, b"x")],
            )
            .unwrap();
        }

        #[test]
        fn a_node_whose_clock_lags_is_not_locked_out() {
            let (mut test, agent, node_id, keypair) = relay_setup();
            test.advance_time_by(100_000);

            // no lower bound: monotonicity alone governs the past, so a node starting from a
            // slow clock simply advances from wherever it is
            relay(
                &mut test,
                &agent,
                vec![signed_declaration(&keypair, node_id, 1, b"slow clock")],
            )
            .unwrap();
            relay(
                &mut test,
                &agent,
                vec![signed_declaration(&keypair, node_id, 2, b"still slow")],
            )
            .unwrap();

            assert_eq!(
                test.node_entry(node_id, &Source::SelfDeclared)
                    .unwrap()
                    .attestation
                    .unwrap()
                    .declared_at,
                2
            );
        }

        #[test]
        fn a_non_whitelisted_sender_is_rejected() {
            let (mut test, _, node_id, keypair) = relay_setup();
            let stranger = test.generate_account();
            let declared_at = now(&test);

            assert_eq!(
                relay(
                    &mut test,
                    &stranger,
                    vec![signed_declaration(&keypair, node_id, declared_at, b"x")]
                )
                .unwrap_err(),
                GeolocationContractError::NotWhitelisted {
                    agent: stranger.clone()
                }
            );
        }

        #[test]
        fn an_agent_without_can_relay_self_declared_is_rejected() {
            let (mut test, _, node_id, keypair) = relay_setup();
            let measure_only = test.add_agent_with_permissions(AgentPermissions {
                can_measure: true,
                can_relay_self_declared: false,
            });
            let declared_at = now(&test);

            // the two flags are independent: this agent's measurement writes keep working
            assert_eq!(
                relay(
                    &mut test,
                    &measure_only,
                    vec![signed_declaration(&keypair, node_id, declared_at, b"x")]
                )
                .unwrap_err(),
                GeolocationContractError::MissingAgentPermission {
                    agent: measure_only,
                    permission: "can_relay_self_declared"
                }
            );
        }

        #[test]
        fn the_same_node_twice_in_one_batch_is_rejected() {
            let (mut test, agent, node_id, keypair) = relay_setup();

            // measurements deliberately allow a repeated key, resolving to the last write, but
            // here that would let the relayer downgrade: both are checked against stored state,
            // so both pass, and whichever it puts last wins regardless of `declared_at`.
            // Rejecting the batch keeps validity independent of the order it arrives in
            let err = relay(
                &mut test,
                &agent,
                vec![
                    signed_declaration(&keypair, node_id, 2_000, b"new"),
                    signed_declaration(&keypair, node_id, 1_000, b"old"),
                ],
            )
            .unwrap_err();

            assert_eq!(
                err,
                GeolocationContractError::DuplicateDeclaration { node_id }
            );
            assert!(test.node_entry(node_id, &Source::SelfDeclared).is_none());
        }

        #[test]
        fn one_bad_artifact_fails_the_whole_batch_without_writing_anything() {
            let (mut test, agent, first, first_key) = relay_setup();
            let (second, second_key) = test.bond_dummy_nymnode_with_keypair().unwrap();
            let (third, third_key) = test.bond_dummy_nymnode_with_keypair().unwrap();
            let declared_at = now(&test);

            let mut forged = signed_declaration(&second_key, second, declared_at, b"b");
            forged.signature = vec![0u8; 64].into();

            let err = relay(
                &mut test,
                &agent,
                vec![
                    signed_declaration(&first_key, first, declared_at, b"a"),
                    forged,
                    signed_declaration(&third_key, third, declared_at, b"c"),
                ],
            )
            .unwrap_err();
            assert_eq!(err, GeolocationContractError::InvalidSignature);

            // every check runs before any write, so the good artifacts either side of the bad
            // one were never persisted and nothing had to be rolled back
            for node_id in [first, second, third] {
                assert!(test.node_entry(node_id, &Source::SelfDeclared).is_none());
            }
            test.assert_digest_is_refold();
        }

        #[test]
        fn an_oversized_payload_is_rejected() {
            let (mut test, agent, node_id, keypair) = relay_setup();
            let oversized = vec![0u8; DEFAULT_MAX_PAYLOAD_SIZE as usize + 1];
            let declared_at = now(&test);

            assert_eq!(
                relay(
                    &mut test,
                    &agent,
                    vec![signed_declaration(
                        &keypair,
                        node_id,
                        declared_at,
                        &oversized
                    )]
                )
                .unwrap_err(),
                GeolocationContractError::PayloadTooLarge {
                    len: oversized.len(),
                    max: DEFAULT_MAX_PAYLOAD_SIZE
                }
            );
        }

        #[test]
        fn an_oversized_batch_is_rejected() {
            let (mut test, agent, node_id, keypair) = relay_setup();
            let max = GEOLOCATION_CONTRACT_STORAGE
                .config
                .load(&test)
                .unwrap()
                .max_batch_size;
            let declared_at = now(&test);

            // distinct node ids so the duplicate check does not fire first; none of them is
            // bonded, which does not matter because the size check comes before the lookup
            let oversized = (0..=max)
                .map(|offset| signed_declaration(&keypair, node_id + offset, declared_at, b"x"))
                .collect::<Vec<_>>();

            assert_eq!(
                relay(&mut test, &agent, oversized).unwrap_err(),
                GeolocationContractError::BatchTooLarge {
                    size: max as usize + 1,
                    max
                }
            );
        }

        #[test]
        fn several_nodes_relay_in_one_batch() {
            let (mut test, agent, first, first_key) = relay_setup();
            let (second, second_key) = test.bond_dummy_nymnode_with_keypair().unwrap();
            let declared_at = now(&test);

            let res = relay(
                &mut test,
                &agent,
                vec![
                    signed_declaration(&first_key, first, declared_at, b"a"),
                    signed_declaration(&second_key, second, declared_at, b"b"),
                ],
            )
            .unwrap();

            assert_eq!(res.events.len(), 1);
            assert_eq!(res.events[0].ty, events::RELAY_SELF_DECLARATIONS);
            assert!(res.events[0]
                .attributes
                .iter()
                .any(|attr| attr.key == events::ATTR_COUNT && attr.value == "2"));
            test.assert_digest_is_refold();
        }
    }

    #[cfg(test)]
    mod admin_overrides {
        use super::*;
        use crate::testing::{
            init_contract_tester, location_entry, measured_by, GeolocationContractTesterExt,
        };
        use cosmwasm_std::testing::message_info;
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
        use nym_geolocation_contract_common::constants::DEFAULT_MAX_PAYLOAD_SIZE;
        use nym_geolocation_contract_common::ExecuteMsg;

        fn payload(content: &[u8]) -> LocationPayload {
            location_entry(content, 0).payload
        }

        fn set_override(
            test: &mut impl GeolocationContractTesterExt,
            sender: &Addr,
            node_id: NodeId,
            content: &[u8],
        ) -> Result<Response, GeolocationContractError> {
            let env = test.env();
            try_set_override(
                test.deps_mut(),
                env,
                message_info(sender, &[]),
                Subject::new_nym_node(node_id),
                payload(content),
            )
        }

        fn remove_override(
            test: &mut impl GeolocationContractTesterExt,
            sender: &Addr,
            node_id: NodeId,
        ) -> Result<Response, GeolocationContractError> {
            try_remove_override(
                test.deps_mut(),
                message_info(sender, &[]),
                Subject::new_nym_node(node_id),
            )
        }

        #[test]
        fn the_admin_sets_and_then_replaces_an_override() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();

            set_override(&mut test, &admin, 42, b"first").unwrap();
            let entry = test.node_entry(42, &Source::Override).unwrap();
            assert_eq!(entry.payload.content, b"first".to_vec());
            assert_eq!(entry.checked_at, test.env().block.time.seconds());
            // an override is the admin asserting a location, not the subject attesting to one
            assert_eq!(entry.attestation, None);
            test.assert_digest_is_refold();

            test.advance_time_by(60);
            set_override(&mut test, &admin, 42, b"second").unwrap();
            assert_eq!(
                test.node_entry(42, &Source::Override)
                    .unwrap()
                    .payload
                    .content,
                b"second".to_vec()
            );
            // one override slot per subject, so replacing retires the old leaf rather than
            // adding a second entry
            assert_eq!(test.node_entries(42).len(), 1);
            test.assert_digest_is_refold();
        }

        #[test]
        fn a_non_admin_cannot_set_an_override() {
            let mut test = init_contract_tester();
            // deliberately an agent that *is* trusted to measure: the two authorities are
            // separate, and being whitelisted buys nothing here
            let agent = test.add_dummy_agent();

            assert_eq!(
                set_override(&mut test, &agent, 42, b"x").unwrap_err(),
                GeolocationContractError::Admin(AdminError::NotAdmin {})
            );
            assert!(test.node_entry(42, &Source::Override).is_none());
        }

        #[test]
        fn a_non_admin_cannot_remove_an_override() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();
            set_override(&mut test, &admin, 42, b"admin says so").unwrap();

            assert_eq!(
                remove_override(&mut test, &agent, 42).unwrap_err(),
                GeolocationContractError::Admin(AdminError::NotAdmin {})
            );
            assert_eq!(
                test.node_entry(42, &Source::Override)
                    .unwrap()
                    .payload
                    .content,
                b"admin says so".to_vec()
            );
        }

        #[test]
        fn removing_an_override_leaves_every_other_source_untouched() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();

            test.set_dummy_measurement_from(42, &agent);
            test.set_dummy_node_self_declared(42);
            set_override(&mut test, &admin, 42, b"override").unwrap();
            assert_eq!(test.node_entries(42).len(), 3);

            remove_override(&mut test, &admin, 42).unwrap();

            // retracting an override must not wait on a re-measurement, so it cannot take the
            // other sources with it
            assert!(test.node_entry(42, &Source::Override).is_none());
            assert!(test.measurement_by(42, &agent).is_some());
            assert!(test.node_entry(42, &Source::SelfDeclared).is_some());
            test.assert_digest_is_refold();
        }

        #[test]
        fn an_override_does_not_suppress_other_entries() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();

            test.set_dummy_measurement_from(42, &agent);
            test.set_dummy_node_self_declared(42);
            set_override(&mut test, &admin, 42, b"override").unwrap();

            // the contract stores one more opinion; it applies no precedence, because choosing
            // between disagreeing sources is the client's job
            let sources = test
                .node_entries(42)
                .into_iter()
                .map(|(source, _)| source)
                .collect::<Vec<_>>();
            assert_eq!(
                sources,
                vec![measured_by(&agent), Source::SelfDeclared, Source::Override]
            );
        }

        #[test]
        fn transferring_the_admin_role_does_not_orphan_overrides() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            set_override(&mut test, &admin, 42, b"set by the old admin").unwrap();

            let new_admin = test.generate_account();
            test.execute_msg(
                admin.clone(),
                &ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                },
            )
            .unwrap();

            // the source names the admin *role*, not the address that wrote it, so the entry
            // stays under the same key and the new admin inherits it
            assert_eq!(
                test.node_entry(42, &Source::Override)
                    .unwrap()
                    .payload
                    .content,
                b"set by the old admin".to_vec()
            );
            assert_eq!(
                set_override(&mut test, &admin, 42, b"x").unwrap_err(),
                GeolocationContractError::Admin(AdminError::NotAdmin {})
            );
            remove_override(&mut test, &new_admin, 42).unwrap();
            assert!(test.node_entry(42, &Source::Override).is_none());
        }

        #[test]
        fn an_oversized_override_payload_is_rejected() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let oversized = vec![0u8; DEFAULT_MAX_PAYLOAD_SIZE as usize + 1];

            // the admin is trusted but not exempt: the bound exists to cap every verifying
            // client's recompute, not to police the writer
            assert_eq!(
                set_override(&mut test, &admin, 42, &oversized).unwrap_err(),
                GeolocationContractError::PayloadTooLarge {
                    len: oversized.len(),
                    max: DEFAULT_MAX_PAYLOAD_SIZE
                }
            );
            assert!(test.node_entry(42, &Source::Override).is_none());
        }

        #[test]
        fn removing_an_override_that_is_not_there_is_a_no_op() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let before = test.digest();

            remove_override(&mut test, &admin, 42).unwrap();

            assert_eq!(test.digest(), before);
            test.assert_digest_is_refold();
        }
    }

    #[cfg(test)]
    mod admin_whitelist {
        use super::*;
        use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
        use crate::testing::{
            init_contract_tester, node_measurement, GeolocationContractTesterExt,
        };
        use cosmwasm_std::testing::message_info;
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};

        fn permissions(can_measure: bool, can_relay_self_declared: bool) -> AgentPermissions {
            AgentPermissions {
                can_measure,
                can_relay_self_declared,
            }
        }

        fn set_agent(
            test: &mut impl GeolocationContractTesterExt,
            sender: &Addr,
            agent: &Addr,
            permissions: AgentPermissions,
        ) -> Result<Response, GeolocationContractError> {
            try_set_whitelisted_agent(
                test.deps_mut(),
                message_info(sender, &[]),
                agent.to_string(),
                permissions,
            )
        }

        fn remove_agent(
            test: &mut impl GeolocationContractTesterExt,
            sender: &Addr,
            agent: &Addr,
        ) -> Result<Response, GeolocationContractError> {
            try_remove_whitelisted_agent(
                test.deps_mut(),
                message_info(sender, &[]),
                agent.to_string(),
            )
        }

        #[test]
        fn the_admin_whitelists_an_agent_and_the_grant_is_digest_committed() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.generate_account();
            let before = test.digest();

            set_agent(&mut test, &admin, &agent, permissions(true, false)).unwrap();

            assert_eq!(
                GEOLOCATION_CONTRACT_STORAGE
                    .may_load_agent_permissions(&test, &agent)
                    .unwrap(),
                Some(permissions(true, false))
            );
            // a measured entry carries no signature, so the whitelist is the only evidence a
            // verifying client has that its writer was ever authorised
            assert_ne!(test.digest(), before);
            test.assert_digest_is_refold();
        }

        #[test]
        fn changing_an_agents_permissions_takes_effect_on_the_next_write() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();

            let env = test.env();
            try_submit_measurements(
                test.deps_mut(),
                env,
                message_info(&agent, &[]),
                vec![node_measurement(1, b"allowed")],
            )
            .unwrap();

            set_agent(&mut test, &admin, &agent, permissions(false, true)).unwrap();

            let env = test.env();
            let err = try_submit_measurements(
                test.deps_mut(),
                env,
                message_info(&agent, &[]),
                vec![node_measurement(2, b"no longer allowed")],
            )
            .unwrap_err();
            assert_eq!(
                err,
                GeolocationContractError::MissingAgentPermission {
                    agent: agent.clone(),
                    permission: "can_measure"
                }
            );

            // revoking one flag leaves the other alone, and does not touch what was already
            // written under it
            assert!(test.measurement_by(1, &agent).is_some());
            test.assert_digest_is_refold();
        }

        #[test]
        fn removing_an_agent_leaves_its_entries_in_place() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();
            test.set_dummy_measurement_from(7, &agent);

            remove_agent(&mut test, &admin, &agent).unwrap();

            // deliberately non-destructive: read-time authorisation makes the entry
            // unauthorised from this block without anything having to be enumerated or
            // deleted, and reclaiming the space is a separate paginated operation
            assert_eq!(
                GEOLOCATION_CONTRACT_STORAGE
                    .may_load_agent_permissions(&test, &agent)
                    .unwrap(),
                None
            );
            assert!(test.measurement_by(7, &agent).is_some());
            test.assert_digest_is_refold();
        }

        #[test]
        fn a_non_admin_cannot_change_the_whitelist() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            let outsider = test.generate_account();

            // notably including a whitelisted agent: being trusted to write measurements does
            // not let an agent grant that trust to anyone else, or revoke a rival
            for sender in [&agent, &outsider] {
                assert_eq!(
                    set_agent(&mut test, sender, &outsider, permissions(true, true)).unwrap_err(),
                    GeolocationContractError::Admin(AdminError::NotAdmin {})
                );
                assert_eq!(
                    remove_agent(&mut test, sender, &agent).unwrap_err(),
                    GeolocationContractError::Admin(AdminError::NotAdmin {})
                );
            }

            assert!(GEOLOCATION_CONTRACT_STORAGE
                .may_load_agent_permissions(&test, &outsider)
                .unwrap()
                .is_none());
            assert!(GEOLOCATION_CONTRACT_STORAGE
                .may_load_agent_permissions(&test, &agent)
                .unwrap()
                .is_some());
        }

        #[test]
        fn an_unparseable_agent_address_is_rejected() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();

            assert!(try_set_whitelisted_agent(
                test.deps_mut(),
                message_info(&admin, &[]),
                "not-an-address".to_owned(),
                permissions(true, true),
            )
            .is_err());
        }

        #[test]
        fn removing_an_agent_that_is_not_whitelisted_is_a_no_op() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let stranger = test.generate_account();
            let before = test.digest();

            remove_agent(&mut test, &admin, &stranger).unwrap();

            assert_eq!(test.digest(), before);
            test.assert_digest_is_refold();
        }
    }

    #[cfg(test)]
    mod explicit_entry_removal {
        use super::*;
        use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
        use crate::testing::{
            init_contract_tester, measured_by, node_measurement, GeolocationContractTesterExt,
        };
        use cosmwasm_std::testing::message_info;
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};
        use nym_geolocation_contract_common::RecordKey;

        fn remove(
            test: &mut impl GeolocationContractTesterExt,
            sender: &Addr,
            keys: Vec<EntryKey>,
        ) -> Result<Response, GeolocationContractError> {
            try_remove_entries(test.deps_mut(), message_info(sender, &[]), keys)
        }

        fn measured_key(node_id: NodeId, agent: &Addr) -> EntryKey {
            EntryKey::new(Subject::new_nym_node(node_id), measured_by(agent))
        }

        #[test]
        fn the_admin_removes_exactly_the_named_entries() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let first = test.add_dummy_agent();
            let second = test.add_dummy_agent();

            for node_id in [1, 2, 3] {
                test.set_dummy_measurement_from(node_id, &first);
                test.set_dummy_measurement_from(node_id, &second);
            }
            test.set_dummy_node_self_declared(2);

            // a mixed batch spanning several subjects and several sources, which is the shape a
            // real purge takes: whatever the admin's off-chain filter selected
            remove(
                &mut test,
                &admin,
                vec![
                    measured_key(1, &first),
                    measured_key(3, &first),
                    EntryKey::new(Subject::new_nym_node(2), Source::SelfDeclared),
                ],
            )
            .unwrap();

            assert!(test.measurement_by(1, &first).is_none());
            assert!(test.measurement_by(3, &first).is_none());
            assert!(test.node_entry(2, &Source::SelfDeclared).is_none());

            // and nothing else went with them
            assert!(test.measurement_by(2, &first).is_some());
            for node_id in [1, 2, 3] {
                assert!(test.measurement_by(node_id, &second).is_some());
            }
            test.assert_digest_is_refold();
        }

        #[test]
        fn de_whitelisting_then_removing_is_the_purge_flow() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let compromised = test.add_dummy_agent();
            let honest = test.add_dummy_agent();
            for node_id in [1, 2] {
                test.set_dummy_measurement_from(node_id, &compromised);
                test.set_dummy_measurement_from(node_id, &honest);
            }

            // step one neutralises the agent for every conforming reader, immediately and
            // without touching any entry
            try_remove_whitelisted_agent(
                test.deps_mut(),
                message_info(&admin, &[]),
                compromised.to_string(),
            )
            .unwrap();
            assert!(test.measurement_by(1, &compromised).is_some());

            // step two reclaims the space, at the admin's leisure, from a key list worked out
            // off-chain against the enumeration
            let stale = test
                .all_records()
                .into_iter()
                .filter_map(|record| match record.key() {
                    RecordKey::Location(key) if key.source.agent() == Some(&compromised) => {
                        Some(key)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(stale.len(), 2);

            remove(&mut test, &admin, stale).unwrap();

            for node_id in [1, 2] {
                assert!(test
                    .measurement_by(node_id, &compromised)
                    .is_none());
                assert!(test.measurement_by(node_id, &honest).is_some());
            }
            test.assert_digest_is_refold();
        }

        #[test]
        fn an_entry_for_a_subject_that_was_never_bonded_can_be_removed() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();

            // measurements deliberately do not check the mixnet contract, so an agent can write
            // for a node id that never existed. The unbond callback will never fire for it, so
            // this is the only path that can ever delete it
            let env = test.env();
            try_submit_measurements(
                test.deps_mut(),
                env,
                message_info(&agent, &[]),
                vec![node_measurement(99_999, b"nobody home")],
            )
            .unwrap();
            assert!(test.measurement_by(99_999, &agent).is_some());

            remove(&mut test, &admin, vec![measured_key(99_999, &agent)]).unwrap();

            assert!(test.measurement_by(99_999, &agent).is_none());
            test.assert_digest_is_refold();
        }

        #[test]
        fn a_non_admin_cannot_remove_entries() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            let outsider = test.generate_account();
            test.set_dummy_measurement_from(1, &agent);

            // notably including the agent that wrote it: an agent may overwrite its own slot but
            // may not delete, so a compromised agent cannot erase its own history
            for sender in [&agent, &outsider] {
                assert_eq!(
                    remove(&mut test, sender, vec![measured_key(1, &agent)]).unwrap_err(),
                    GeolocationContractError::Admin(AdminError::NotAdmin {})
                );
            }
            assert!(test.measurement_by(1, &agent).is_some());
        }

        #[test]
        fn naming_a_key_that_holds_nothing_is_not_an_error() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();
            test.set_dummy_measurement_from(1, &agent);
            let before = test.digest();

            // the admin works from an enumeration pulled at some earlier height, so an entry it
            // names may already be gone. Failing the batch for that would make a purge a race
            remove(
                &mut test,
                &admin,
                vec![
                    measured_key(7, &agent),
                    EntryKey::new(Subject::new_nym_node(1), Source::Override),
                ],
            )
            .unwrap();

            assert_eq!(test.digest(), before);
            assert!(test.measurement_by(1, &agent).is_some());
            test.assert_digest_is_refold();
        }

        #[test]
        fn a_batch_over_the_configured_maximum_is_rejected_whole() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();
            let max = GEOLOCATION_CONTRACT_STORAGE
                .config
                .load(&test)
                .unwrap()
                .max_batch_size;

            for node_id in 0..=max {
                test.set_dummy_measurement_from(node_id, &agent);
            }
            let before = test.digest();

            let oversized = (0..=max)
                .map(|node_id| measured_key(node_id, &agent))
                .collect::<Vec<_>>();
            assert_eq!(
                remove(&mut test, &admin, oversized).unwrap_err(),
                GeolocationContractError::BatchTooLarge {
                    size: max as usize + 1,
                    max
                }
            );

            // rejected before anything was deleted, so a purge cannot half-apply
            assert_eq!(test.digest(), before);
            assert_eq!(test.node_measurements(0).len(), 1);
        }

        #[test]
        fn a_whitelist_entry_cannot_be_removed_this_way() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();

            // `EntryKey` names a location entry and nothing else, so there is no way to express
            // a whitelist removal here. That keeps revocation and its authorisation meaning in
            // one place rather than two
            remove(&mut test, &admin, vec![measured_key(1, &agent)]).unwrap();

            assert!(GEOLOCATION_CONTRACT_STORAGE
                .may_load_agent_permissions(&test, &agent)
                .unwrap()
                .is_some());
            test.assert_digest_is_refold();
        }
    }

    #[cfg(test)]
    mod unbond_callback {
        use super::*;
        use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
        use crate::testing::{init_contract_tester, measured_by, GeolocationContractTesterExt};
        use cosmwasm_std::testing::message_info;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::{AdminExt, RandExt};

        fn unbond(
            test: &mut impl GeolocationContractTesterExt,
            sender: &Addr,
            node_id: NodeId,
        ) -> Result<Response, GeolocationContractError> {
            try_handle_node_unbonding(test.deps_mut(), message_info(sender, &[]), node_id)
        }

        /// Populate one node with an entry from every source there is.
        fn populate(test: &mut impl GeolocationContractTesterExt, node_id: NodeId) -> (Addr, Addr) {
            let first = test.add_dummy_agent();
            let second = test.add_dummy_agent();
            test.set_dummy_measurement_from(node_id, &first);
            test.set_dummy_measurement_from(node_id, &second);
            test.set_dummy_node_self_declared(node_id);
            test.set_dummy_node_override(node_id);
            (first, second)
        }

        #[test]
        fn the_callback_deletes_every_source_for_that_node() {
            let mut test = init_contract_tester();
            let mixnet = test.mixnet_contract_address().unwrap();
            let (first, second) = populate(&mut test, 42);
            assert_eq!(test.node_entries(42).len(), 4);

            let res = unbond(&mut test, &mixnet, 42).unwrap();

            // the admin's override goes with the rest: the subject has ceased to exist, so
            // nothing anyone asserted about where it was is meaningful any more
            assert!(test.node_entries(42).is_empty());
            for source in [
                measured_by(&first),
                measured_by(&second),
                Source::SelfDeclared,
                Source::Override,
            ] {
                assert!(test.node_entry(42, &source).is_none());
            }
            test.assert_digest_is_refold();

            assert_eq!(res.events[0].ty, events::ON_NYM_NODE_UNBOND);
            assert!(res.events[0]
                .attributes
                .iter()
                .any(|attr| attr.key == events::ATTR_COUNT && attr.value == "4"));
        }

        #[test]
        fn other_subjects_are_untouched() {
            let mut test = init_contract_tester();
            let mixnet = test.mixnet_contract_address().unwrap();
            populate(&mut test, 42);
            let (survivor, _) = populate(&mut test, 43);

            unbond(&mut test, &mixnet, 42).unwrap();

            assert!(test.node_entries(42).is_empty());
            assert_eq!(test.node_entries(43).len(), 4);
            assert!(test.measurement_by(43, &survivor).is_some());
            test.assert_digest_is_refold();
        }

        #[test]
        fn the_whitelist_is_untouched() {
            let mut test = init_contract_tester();
            let mixnet = test.mixnet_contract_address().unwrap();
            let (first, second) = populate(&mut test, 42);

            unbond(&mut test, &mixnet, 42).unwrap();

            // one node unbonding says nothing about who is authorised to measure, so the
            // whitelist is a different entry class and stays put
            for agent in [&first, &second] {
                assert!(GEOLOCATION_CONTRACT_STORAGE
                    .may_load_agent_permissions(&test, agent)
                    .unwrap()
                    .is_some());
            }
            test.assert_digest_is_refold();
        }

        #[test]
        fn only_the_configured_mixnet_contract_may_invoke_it() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();
            let outsider = test.generate_account();
            test.set_dummy_measurement_from(42, &agent);

            // notably including the admin: without this check any address could clear a live
            // node's entries, which is a denial of service against a node that has done nothing
            for sender in [&admin, &agent, &outsider] {
                assert_eq!(
                    unbond(&mut test, sender, 42).unwrap_err(),
                    GeolocationContractError::UnauthorisedMixnetCallback {
                        sender: sender.clone()
                    }
                );
            }
            assert!(test.measurement_by(42, &agent).is_some());
        }

        #[test]
        fn a_node_with_nothing_stored_is_a_no_op() {
            let mut test = init_contract_tester();
            let mixnet = test.mixnet_contract_address().unwrap();
            let agent = test.add_dummy_agent();
            test.set_dummy_measurement_from(7, &agent);
            let before = test.digest();

            // the common case, since the callback fires for every unbond whether or not the node
            // was ever measured
            unbond(&mut test, &mixnet, 42).unwrap();

            assert_eq!(test.digest(), before);
            assert!(test.measurement_by(7, &agent).is_some());
            test.assert_digest_is_refold();
        }

        #[test]
        fn unbonding_through_the_mixnet_contract_reaches_this_handler() {
            let mut test = init_contract_tester();
            let (node_id, _) = test.bond_dummy_nymnode_with_keypair().unwrap();
            let agent = test.add_dummy_agent();
            test.set_dummy_measurement_from(node_id, &agent);
            test.set_dummy_node_self_declared(node_id);
            assert_eq!(test.node_entries(node_id).len(), 2);

            // App-level rather than a direct handler call: deps-level tests do not dispatch the
            // sub-messages a `Response` carries, so this is the only shape that proves the
            // mixnet contract actually reaches us rather than merely intending to
            test.unbond_nymnode(node_id).unwrap();

            assert!(test.node_entries(node_id).is_empty());
            // the whitelisted agent is the only record left, since it is a different entry
            // class and one node unbonding says nothing about who may measure
            assert_eq!(test.all_records().len(), 1);
            test.assert_digest_is_refold();
        }
    }

    #[cfg(test)]
    mod admin_config_and_role {
        use super::*;
        use crate::testing::{
            init_contract_tester, measured_by, node_measurement, GeolocationContractTesterExt,
        };
        use cosmwasm_std::testing::message_info;
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
        use nym_geolocation_contract_common::constants::{
            DEFAULT_MAX_PAYLOAD_SIZE, DEFAULT_MAX_SKEW_SECS,
        };
        use nym_geolocation_contract_common::{ContractConfig, ExecuteMsg};

        fn update(
            test: &mut impl GeolocationContractTesterExt,
            sender: &Addr,
            max_skew_secs: Option<u64>,
            max_batch_size: Option<u32>,
            max_payload_size: Option<u32>,
        ) -> Result<Response, GeolocationContractError> {
            try_update_config(
                test.deps_mut(),
                message_info(sender, &[]),
                max_skew_secs,
                max_batch_size,
                max_payload_size,
            )
        }

        fn config(test: &impl GeolocationContractTesterExt) -> ContractConfig {
            GEOLOCATION_CONTRACT_STORAGE.config.load(test).unwrap()
        }

        fn submit(
            test: &mut impl GeolocationContractTesterExt,
            agent: &Addr,
            content: &[u8],
        ) -> Result<Response, GeolocationContractError> {
            let env = test.env();
            try_submit_measurements(
                test.deps_mut(),
                env,
                message_info(agent, &[]),
                vec![node_measurement(1, content)],
            )
        }

        #[test]
        fn every_field_lands_in_its_own_slot() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();

            // three distinct values, so swapping any two of them would fail here. Two of the
            // three are `u32` and adjacent, which is exactly the pair that could be transposed
            // without the compiler noticing
            update(&mut test, &admin, Some(11), Some(22), Some(33)).unwrap();

            assert_eq!(
                config(&test),
                ContractConfig {
                    max_skew_secs: 11,
                    max_batch_size: 22,
                    max_payload_size: 33,
                }
            );
        }

        #[test]
        fn omitted_fields_keep_their_current_value() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();

            update(&mut test, &admin, None, Some(7), None).unwrap();

            assert_eq!(
                config(&test),
                ContractConfig {
                    max_skew_secs: DEFAULT_MAX_SKEW_SECS,
                    max_batch_size: 7,
                    max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
                }
            );

            update(&mut test, &admin, None, None, None).unwrap();
            assert_eq!(config(&test).max_batch_size, 7);
        }

        #[test]
        fn a_partial_update_cannot_reach_a_configuration_instantiation_would_refuse() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let before = config(&test);

            // validated as a whole rather than field by field, so the same guard that rejects an
            // inert contract at instantiation also rejects arriving at one a field at a time
            for (batch, payload) in [(Some(0), None), (None, Some(0))] {
                assert!(matches!(
                    update(&mut test, &admin, None, batch, payload).unwrap_err(),
                    GeolocationContractError::InvalidConfig { .. }
                ));
                assert_eq!(config(&test), before);
            }

            // a zero skew stays acceptable: strict policy, not an inert contract
            update(&mut test, &admin, Some(0), None, None).unwrap();
            assert_eq!(config(&test).max_skew_secs, 0);
        }

        #[test]
        fn a_non_admin_cannot_update_the_config() {
            let mut test = init_contract_tester();
            let agent = test.add_dummy_agent();
            let outsider = test.generate_account();
            let before = config(&test);

            for sender in [&agent, &outsider] {
                assert_eq!(
                    update(&mut test, sender, Some(1), Some(1), Some(1)).unwrap_err(),
                    GeolocationContractError::Admin(AdminError::NotAdmin {})
                );
            }
            assert_eq!(config(&test), before);
        }

        #[test]
        fn a_raised_payload_bound_takes_effect_on_the_next_write() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();
            let big = vec![0u8; DEFAULT_MAX_PAYLOAD_SIZE as usize + 1];

            assert!(submit(&mut test, &agent, &big).is_err());

            // the whole reason the bound is state rather than a constant: a later payload
            // version needing more room is an admin transaction, not a redeploy
            update(
                &mut test,
                &admin,
                None,
                None,
                Some(DEFAULT_MAX_PAYLOAD_SIZE * 2),
            )
            .unwrap();

            submit(&mut test, &agent, &big).unwrap();
            assert_eq!(
                test.measurement_by(1, &agent)
                    .unwrap()
                    .payload
                    .content
                    .len(),
                big.len()
            );
        }

        #[test]
        fn lowering_the_payload_bound_does_not_invalidate_what_is_already_stored() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let agent = test.add_dummy_agent();
            test.set_node_measurement(1, measured_by(&agent), b"a payload of some length", 1234);

            update(&mut test, &admin, None, None, Some(4)).unwrap();

            // not retroactive: the entry stays readable and stays in the digest, and shrinking
            // the stored set is a removal rather than a config change
            let entry = test.measurement_by(1, &agent).unwrap();
            assert_eq!(entry.payload.content, b"a payload of some length".to_vec());
            test.assert_digest_is_refold();
        }

        #[test]
        fn a_non_admin_cannot_transfer_the_admin_role() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();
            let outsider = test.generate_account();

            // the highest-privilege operation there is: whoever holds it can set overrides,
            // rewrite the whitelist and delete any entry
            let err = try_update_contract_admin(
                test.deps_mut(),
                message_info(&outsider, &[]),
                outsider.to_string(),
            )
            .unwrap_err();
            assert_eq!(
                err,
                GeolocationContractError::Admin(AdminError::NotAdmin {})
            );

            test.execute_msg(
                admin.clone(),
                &ExecuteMsg::UpdateAdmin {
                    admin: outsider.to_string(),
                },
            )
            .unwrap();
            assert_eq!(test.admin_unchecked(), outsider);
        }

        #[test]
        fn an_unparseable_admin_address_is_rejected() {
            let mut test = init_contract_tester();
            let admin = test.admin_unchecked();

            assert!(try_update_contract_admin(
                test.deps_mut(),
                message_info(&admin, &[]),
                "not-an-address".to_owned(),
            )
            .is_err());
            assert_eq!(test.admin_unchecked(), admin);
        }
    }
}
