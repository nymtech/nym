// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! State-mutating execute handlers.

use crate::storage::NYM_DIRECTORY_CONTRACT_STORAGE;
use cosmwasm_std::{Api, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response};
use nym_directory_contract_common::constants::{events, MAX_LABEL_SIZE_CEILING};
use nym_directory_contract_common::{
    node_signing_payload, CuratedEntry, DirectoryContractError, LabelConfig, NodeEntry,
};
use nym_mixnet_contract_common::{MixnetContractQuerier, NodeId};

/// Fetch `node_id`'s ed25519 identity public key (32 raw bytes) from its mixnet
/// bond, requiring the node to be bonded and not unbonding. The bond stores the key
/// base58-encoded; an unbonding or absent node is rejected with `NodeNotBonded`.
fn bonded_node_identity_key(
    deps: Deps,
    node_id: NodeId,
) -> Result<Vec<u8>, DirectoryContractError> {
    let mixnet = NYM_DIRECTORY_CONTRACT_STORAGE
        .mixnet_contract_address
        .load(deps.storage)?;
    let bond = deps
        .querier
        .query_nymnode_bond(&mixnet, node_id)?
        .ok_or(DirectoryContractError::NodeNotBonded { node_id })?;
    if bond.is_unbonding {
        return Err(DirectoryContractError::NodeNotBonded { node_id });
    }
    let key = bs58::decode(bond.identity())
        .into_vec()
        .map_err(|_| DirectoryContractError::InvalidIdentityKey { node_id })?;
    if key.len() != 32 {
        return Err(DirectoryContractError::InvalidIdentityKey { node_id });
    }
    Ok(key)
}

/// Assert the signed `sequence` exactly equals `node_id`'s expected next sequence
/// (gap-free replay protection - a too-low or too-high value is rejected alike).
fn ensure_expected_sequence(
    deps: Deps,
    node_id: NodeId,
    provided: u64,
) -> Result<(), DirectoryContractError> {
    let expected = NYM_DIRECTORY_CONTRACT_STORAGE.current_sequence(deps.storage, node_id)?;
    if provided != expected {
        return Err(DirectoryContractError::InvalidSequence {
            node_id,
            expected,
            provided,
        });
    }
    Ok(())
}

/// Verify `signature` over `payload` against the node's `identity_key`; both a
/// failed check and a verifier error map to `InvalidSignature`.
fn verify_node_signature(
    api: &dyn Api,
    payload: &[u8],
    signature: &[u8],
    identity_key: &[u8],
) -> Result<(), DirectoryContractError> {
    let verified = api
        .ed25519_verify(payload, signature, identity_key)
        .map_err(|_| DirectoryContractError::InvalidSignature)?;
    if !verified {
        return Err(DirectoryContractError::InvalidSignature);
    }
    Ok(())
}

/// Create or replace a node entry. Authorised solely by an ed25519 signature from
/// the node's identity key over [`node_signing_payload`]; the transaction sender is
/// unchecked, so any account may relay it.
pub(crate) fn try_set_node_entry(
    deps: DepsMut,
    env: Env,
    node_id: NodeId,
    label: String,
    data: Binary,
    sequence: u64,
    signature: Binary,
) -> Result<Response, DirectoryContractError> {
    // Empty data is disallowed: it would make a set signature coincide with a delete
    // signature (which signs the canonical payload with empty data) for the same
    // slot + sequence, letting a relayed set be replayed as a delete.
    if data.as_slice().is_empty() {
        return Err(DirectoryContractError::EmptyNodeData { label });
    }

    // fetch node identity key (it must be bonded)
    let identity_key = bonded_node_identity_key(deps.as_ref(), node_id)?;

    // label must be whitelisted and data within its configured size limit
    let label_config = NYM_DIRECTORY_CONTRACT_STORAGE
        .allowed_storage_labels
        .may_load(deps.storage, label.clone())?
        .ok_or_else(|| DirectoryContractError::LabelNotAllowed {
            label: label.clone(),
        })?;
    if data.len() > label_config.max_size as usize {
        return Err(DirectoryContractError::DataTooLarge {
            label,
            len: data.len(),
            max: label_config.max_size,
        });
    }

    // check for replayed message
    ensure_expected_sequence(deps.as_ref(), node_id, sequence)?;

    // authorise the write via the node's identity-key signature
    let payload = node_signing_payload(node_id, &label, sequence, data.as_slice());
    verify_node_signature(deps.api, &payload, signature.as_slice(), &identity_key)?;

    // persist (updating the digest) and advance the sequence
    let entry = NodeEntry {
        data,
        updated_at_height: env.block.height,
        sequence,
        signature,
    };
    NYM_DIRECTORY_CONTRACT_STORAGE.set_node_entry(deps.storage, node_id, &label, entry)?;
    NYM_DIRECTORY_CONTRACT_STORAGE.save_account_sequence(deps.storage, node_id, sequence + 1)?;

    Ok(Response::new().add_event(
        Event::new(events::SET_NODE_ENTRY)
            .add_attribute(events::ATTR_NODE_ID, node_id.to_string())
            .add_attribute(events::ATTR_LABEL, label.as_str())
            .add_attribute(events::ATTR_SEQUENCE, sequence.to_string()),
    ))
}

/// Delete a node entry. Same identity-key authorisation as [`try_set_node_entry`],
/// signing the canonical payload with empty `data` (sets reject empty data, so the
/// set and delete signature spaces are disjoint). Idempotent on the entry itself,
/// but the sequence still advances so the signed delete cannot be replayed.
pub(crate) fn try_delete_node_entry(
    deps: DepsMut,
    node_id: NodeId,
    label: String,
    sequence: u64,
    signature: Binary,
) -> Result<Response, DirectoryContractError> {
    let identity_key = bonded_node_identity_key(deps.as_ref(), node_id)?;
    ensure_expected_sequence(deps.as_ref(), node_id, sequence)?;

    let payload = node_signing_payload(node_id, &label, sequence, &[]);
    verify_node_signature(deps.api, &payload, signature.as_slice(), &identity_key)?;

    NYM_DIRECTORY_CONTRACT_STORAGE.remove_node_entry(deps.storage, node_id, &label)?;
    NYM_DIRECTORY_CONTRACT_STORAGE.save_account_sequence(deps.storage, node_id, sequence + 1)?;

    Ok(Response::new().add_event(
        Event::new(events::DELETE_NODE_ENTRY)
            .add_attribute(events::ATTR_NODE_ID, node_id.to_string())
            .add_attribute(events::ATTR_LABEL, label.as_str())
            .add_attribute(events::ATTR_SEQUENCE, sequence.to_string()),
    ))
}

// ---- admin path ----

/// Add or update a whitelisted label and its `max_size`. Admin only; `max_size`
/// must not exceed [`MAX_LABEL_SIZE_CEILING`].
pub(crate) fn try_set_label(
    deps: DepsMut,
    info: MessageInfo,
    label: String,
    max_size: u32,
) -> Result<Response, DirectoryContractError> {
    NYM_DIRECTORY_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;

    if max_size > MAX_LABEL_SIZE_CEILING {
        return Err(DirectoryContractError::MaxSizeAboveCeiling {
            requested: max_size,
            ceiling: MAX_LABEL_SIZE_CEILING,
        });
    }

    NYM_DIRECTORY_CONTRACT_STORAGE.allowed_storage_labels.save(
        deps.storage,
        label.clone(),
        &LabelConfig { max_size },
    )?;

    Ok(Response::new().add_event(
        Event::new(events::SET_LABEL)
            .add_attribute(events::ATTR_LABEL, label.as_str())
            .add_attribute(events::ATTR_MAX_SIZE, max_size.to_string()),
    ))
}

/// Remove a label from the whitelist. Admin only. Non-destructive: existing entries
/// under the label stay readable and committed to the digest; only new writes/updates
/// under it are blocked.
pub(crate) fn try_remove_label(
    deps: DepsMut,
    info: MessageInfo,
    label: String,
) -> Result<Response, DirectoryContractError> {
    NYM_DIRECTORY_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;

    NYM_DIRECTORY_CONTRACT_STORAGE
        .allowed_storage_labels
        .remove(deps.storage, label.clone());

    Ok(Response::new().add_event(
        Event::new(events::REMOVE_LABEL).add_attribute(events::ATTR_LABEL, label.as_str()),
    ))
}

/// Create or replace a curated entry under an admin-chosen `key`. Admin only;
/// keeps the global digest in sync.
pub(crate) fn try_set_curated_entry(
    deps: DepsMut,
    info: MessageInfo,
    key: String,
    data: Binary,
) -> Result<Response, DirectoryContractError> {
    NYM_DIRECTORY_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;

    NYM_DIRECTORY_CONTRACT_STORAGE.set_curated_entry(deps.storage, &key, CuratedEntry { data })?;

    Ok(Response::new().add_event(
        Event::new(events::SET_CURATED_ENTRY).add_attribute(events::ATTR_KEY, key.as_str()),
    ))
}

/// Delete a curated entry. Admin only; keeps the global digest in sync. Idempotent
/// on a missing key.
pub(crate) fn try_remove_curated_entry(
    deps: DepsMut,
    info: MessageInfo,
    key: String,
) -> Result<Response, DirectoryContractError> {
    NYM_DIRECTORY_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;

    NYM_DIRECTORY_CONTRACT_STORAGE.remove_curated_entry(deps.storage, &key)?;

    Ok(Response::new().add_event(
        Event::new(events::REMOVE_CURATED_ENTRY).add_attribute(events::ATTR_KEY, key.as_str()),
    ))
}

/// Transfer the admin role to `admin`. Admin only (delegated to `cw-controllers`,
/// which asserts the caller is the current admin and emits the update attributes).
/// The admin is always set - it cannot be cleared.
pub(crate) fn try_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    admin: String,
) -> Result<Response, DirectoryContractError> {
    let new_admin = deps.api.addr_validate(&admin)?;
    Ok(NYM_DIRECTORY_CONTRACT_STORAGE
        .contract_admin
        .execute_update_admin(deps, info, Some(new_admin))?)
}

// ---- mixnet unbond callback ----

/// Cross-contract callback fired by the mixnet contract when `node_id` unbonds:
/// delete all of that node's entries in a single digest update. Authorised only when
/// the caller is the configured mixnet contract (`UnauthorisedMixnetCallback`
/// otherwise). The mixnet dispatches this as a hard sub-message (not reply-on-error),
/// so the directory contract must be deployed in every environment; the handler is
/// idempotent, so a node with no entries is a safe no-op.
pub(crate) fn try_handle_node_unbonding(
    deps: DepsMut,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, DirectoryContractError> {
    let mixnet_contract = NYM_DIRECTORY_CONTRACT_STORAGE
        .mixnet_contract_address
        .load(deps.storage)?;
    if info.sender != mixnet_contract {
        return Err(DirectoryContractError::UnauthorisedMixnetCallback {
            sender: info.sender,
        });
    }

    NYM_DIRECTORY_CONTRACT_STORAGE.remove_all_node_entries(deps.storage, node_id)?;

    Ok(Response::new().add_event(
        Event::new(events::ON_NYM_NODE_UNBOND)
            .add_attribute(events::ATTR_NODE_ID, node_id.to_string()),
    ))
}

pub(crate) fn try_update_snapshot_interval(
    deps: DepsMut,
    info: MessageInfo,
    interval: u32,
) -> Result<Response, DirectoryContractError> {
    NYM_DIRECTORY_CONTRACT_STORAGE
        .contract_admin
        .assert_admin(deps.as_ref(), &info.sender)?;

    if interval == 0 {
        return Err(DirectoryContractError::InvalidSnapshotInterval);
    }

    NYM_DIRECTORY_CONTRACT_STORAGE.save_snapshot_interval(deps.storage, interval)?;
    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    #[test]
    fn set_node_entry_rejects_empty_data() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let err = try_set_node_entry(
            deps.as_mut(),
            env,
            1,
            "sphinx_key".to_string(),
            Binary::default(),
            0,
            Binary::default(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            DirectoryContractError::EmptyNodeData {
                label: "sphinx_key".to_string()
            }
        );
    }

    mod snapshot_interval {
        use super::*;
        use crate::queries::query_snapshot_interval;
        use crate::testing::init_contract_tester;
        use cosmwasm_std::testing::message_info;
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};
        use nym_directory_contract_common::constants::DEFAULT_SNAPSHOT_INTERVAL;
        use nym_lthash::LtHash16;

        fn not_admin() -> DirectoryContractError {
            DirectoryContractError::Admin(AdminError::NotAdmin {})
        }

        #[test]
        fn admin_can_update_the_interval_and_it_is_queryable() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();

            // a fresh contract carries the default interval, and it is queryable
            assert_eq!(
                query_snapshot_interval(tester.deps()).unwrap().interval,
                DEFAULT_SNAPSHOT_INTERVAL
            );

            try_update_snapshot_interval(tester.deps_mut(), admin, 250).unwrap();

            assert_eq!(
                query_snapshot_interval(tester.deps()).unwrap().interval,
                250
            );
        }

        #[test]
        fn non_admin_cannot_update_the_interval() {
            let mut tester = init_contract_tester();
            let stranger = tester.generate_account();

            let err =
                try_update_snapshot_interval(tester.deps_mut(), message_info(&stranger, &[]), 250)
                    .unwrap_err();
            assert_eq!(err, not_admin());

            // and the interval is left untouched
            assert_eq!(
                query_snapshot_interval(tester.deps()).unwrap().interval,
                DEFAULT_SNAPSHOT_INTERVAL
            );
        }

        #[test]
        fn update_rejects_a_zero_interval() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();

            let err = try_update_snapshot_interval(tester.deps_mut(), admin, 0).unwrap_err();
            assert_eq!(err, DirectoryContractError::InvalidSnapshotInterval);
        }

        #[test]
        fn updating_the_interval_leaves_the_digest_unchanged() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();

            // plant an entry so the digest is non-trivial ...
            try_set_curated_entry(
                tester.deps_mut(),
                admin.clone(),
                "nym-api/1".to_string(),
                Binary::from(b"v".to_vec()),
            )
            .unwrap();
            let before = NYM_DIRECTORY_CONTRACT_STORAGE
                .load_digest(tester.deps().storage)
                .unwrap();
            assert_ne!(before, LtHash16::new());

            // ... updating the interval must not touch it (the interval is not a digest leaf)
            try_update_snapshot_interval(tester.deps_mut(), admin, 250).unwrap();
            let after = NYM_DIRECTORY_CONTRACT_STORAGE
                .load_digest(tester.deps().storage)
                .unwrap();
            assert_eq!(before, after);
        }
    }

    mod admin_path {
        use super::*;
        use crate::testing::init_contract_tester;
        use cosmwasm_std::testing::message_info;
        use cw_controllers::AdminError;
        use nym_contracts_common_testing::{AdminExt, ContractOpts, RandExt};
        use nym_lthash::LtHash16;

        fn not_admin() -> DirectoryContractError {
            DirectoryContractError::Admin(AdminError::NotAdmin {})
        }

        #[test]
        fn admin_can_add_and_update_a_label() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();

            try_set_label(
                tester.deps_mut(),
                admin.clone(),
                "newlabel".to_string(),
                1024,
            )
            .unwrap();
            assert_eq!(
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .allowed_storage_labels
                    .load(tester.deps().storage, "newlabel".to_string())
                    .unwrap()
                    .max_size,
                1024
            );

            // setting an existing label updates its size
            try_set_label(tester.deps_mut(), admin, "newlabel".to_string(), 2048).unwrap();
            assert_eq!(
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .allowed_storage_labels
                    .load(tester.deps().storage, "newlabel".to_string())
                    .unwrap()
                    .max_size,
                2048
            );
        }

        #[test]
        fn set_label_rejects_max_size_above_ceiling() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();
            let err = try_set_label(
                tester.deps_mut(),
                admin,
                "big".to_string(),
                MAX_LABEL_SIZE_CEILING + 1,
            )
            .unwrap_err();
            assert_eq!(
                err,
                DirectoryContractError::MaxSizeAboveCeiling {
                    requested: MAX_LABEL_SIZE_CEILING + 1,
                    ceiling: MAX_LABEL_SIZE_CEILING,
                }
            );
        }

        #[test]
        fn non_admin_cannot_set_a_label() {
            let mut tester = init_contract_tester();
            let stranger = tester.generate_account();
            let err = try_set_label(
                tester.deps_mut(),
                message_info(&stranger, &[]),
                "x".to_string(),
                1,
            )
            .unwrap_err();
            assert_eq!(err, not_admin());
        }

        #[test]
        fn remove_label_is_non_destructive() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();

            try_set_label(tester.deps_mut(), admin.clone(), "lbl".to_string(), 1024).unwrap();
            // plant an entry under the label directly (bypassing the signed write path)
            {
                let deps = tester.deps_mut();
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .set_node_entry(
                        deps.storage,
                        1,
                        "lbl",
                        NodeEntry {
                            data: Binary::from(b"payload".to_vec()),
                            updated_at_height: 0,
                            sequence: 0,
                            signature: Binary::default(),
                        },
                    )
                    .unwrap();
            }

            try_remove_label(tester.deps_mut(), admin, "lbl".to_string()).unwrap();

            // the whitelist entry is gone (new writes blocked) ...
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .allowed_storage_labels
                .may_load(tester.deps().storage, "lbl".to_string())
                .unwrap()
                .is_none());
            // ... but the existing entry survives
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(tester.deps().storage, 1, "lbl")
                .unwrap()
                .is_some());
        }

        #[test]
        fn non_admin_cannot_remove_a_label() {
            let mut tester = init_contract_tester();
            let stranger = tester.generate_account();
            let err = try_remove_label(
                tester.deps_mut(),
                message_info(&stranger, &[]),
                "sphinx_key".to_string(),
            )
            .unwrap_err();
            assert_eq!(err, not_admin());
        }

        #[test]
        fn admin_can_set_and_remove_curated_entry_updating_digest() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();

            // fresh contract has the empty digest
            assert_eq!(
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .load_digest(tester.deps().storage)
                    .unwrap(),
                LtHash16::new()
            );

            try_set_curated_entry(
                tester.deps_mut(),
                admin.clone(),
                "nym-api/1".to_string(),
                Binary::from(b"v".to_vec()),
            )
            .unwrap();
            assert_eq!(
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .curated_entries
                    .may_load(tester.deps().storage, "nym-api/1")
                    .unwrap(),
                Some(CuratedEntry {
                    data: Binary::from(b"v".to_vec())
                })
            );
            // the digest now commits the entry
            assert_ne!(
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .load_digest(tester.deps().storage)
                    .unwrap(),
                LtHash16::new()
            );

            try_remove_curated_entry(tester.deps_mut(), admin, "nym-api/1".to_string()).unwrap();
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .curated_entries
                .may_load(tester.deps().storage, "nym-api/1")
                .unwrap()
                .is_none());
            // ... and the digest is back to empty
            assert_eq!(
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .load_digest(tester.deps().storage)
                    .unwrap(),
                LtHash16::new()
            );
        }

        #[test]
        fn non_admin_cannot_set_a_curated_entry() {
            let mut tester = init_contract_tester();
            let stranger = tester.generate_account();
            let err = try_set_curated_entry(
                tester.deps_mut(),
                message_info(&stranger, &[]),
                "k".to_string(),
                Binary::from(b"v".to_vec()),
            )
            .unwrap_err();
            assert_eq!(err, not_admin());
        }

        #[test]
        fn admin_can_transfer_the_admin_role() {
            let mut tester = init_contract_tester();
            let admin = tester.admin_msg();
            let new_admin = tester.generate_account();

            try_update_admin(tester.deps_mut(), admin.clone(), new_admin.to_string()).unwrap();

            // the new admin is now in control; the old one is not
            NYM_DIRECTORY_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(tester.deps(), &new_admin)
                .unwrap();
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(tester.deps(), &admin.sender)
                .is_err());
        }

        #[test]
        fn non_admin_cannot_transfer_the_admin_role() {
            let mut tester = init_contract_tester();
            let stranger = tester.generate_account();
            let err = try_update_admin(
                tester.deps_mut(),
                message_info(&stranger, &[]),
                stranger.to_string(),
            )
            .unwrap_err();
            assert_eq!(err, not_admin());
        }
    }

    mod unbond_callback {
        use super::*;
        use crate::testing::{init_contract_tester, init_contract_tester_with_node_families};
        use cosmwasm_std::testing::message_info;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::{ContractOpts, RandExt};

        fn node_entry() -> NodeEntry {
            NodeEntry {
                data: Binary::from(b"payload".to_vec()),
                updated_at_height: 0,
                sequence: 0,
                signature: Binary::default(),
            }
        }

        #[test]
        fn mixnet_callback_clears_the_nodes_entries() {
            let mut tester = init_contract_tester();
            let mixnet = NYM_DIRECTORY_CONTRACT_STORAGE
                .mixnet_contract_address
                .load(tester.deps().storage)
                .unwrap();

            // plant entries directly: two for node 7, one for node 8, one curated
            {
                let deps = tester.deps_mut();
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .set_node_entry(deps.storage, 7, "a", node_entry())
                    .unwrap();
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .set_node_entry(deps.storage, 7, "b", node_entry())
                    .unwrap();
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .set_node_entry(deps.storage, 8, "a", node_entry())
                    .unwrap();
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .set_curated_entry(
                        deps.storage,
                        "k",
                        CuratedEntry {
                            data: Binary::from(b"v".to_vec()),
                        },
                    )
                    .unwrap();
            }
            let before = NYM_DIRECTORY_CONTRACT_STORAGE
                .load_digest(tester.deps().storage)
                .unwrap();

            try_handle_node_unbonding(tester.deps_mut(), message_info(&mixnet, &[]), 7).unwrap();

            // node 7's entries are gone
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .node_range(tester.deps().storage, 7)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .is_empty());
            // node 8 and the curated entry survive
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(tester.deps().storage, 8, "a")
                .unwrap()
                .is_some());
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .curated_entries
                .may_load(tester.deps().storage, "k")
                .unwrap()
                .is_some());
            // the digest changed (node 7's leaves removed); exact digest math is
            // covered by the storage-level `remove_all_node_entries` test
            assert_ne!(
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .load_digest(tester.deps().storage)
                    .unwrap(),
                before
            );
        }

        #[test]
        fn non_mixnet_caller_is_rejected() {
            let mut tester = init_contract_tester();
            let stranger = tester.generate_account();
            let err = try_handle_node_unbonding(tester.deps_mut(), message_info(&stranger, &[]), 7)
                .unwrap_err();
            assert_eq!(
                err,
                DirectoryContractError::UnauthorisedMixnetCallback { sender: stranger }
            );
        }

        #[test]
        fn callback_is_idempotent_for_a_node_with_no_entries() {
            let mut tester = init_contract_tester();
            let mixnet = NYM_DIRECTORY_CONTRACT_STORAGE
                .mixnet_contract_address
                .load(tester.deps().storage)
                .unwrap();
            // plant an unrelated entry so the digest is non-trivial
            {
                let deps = tester.deps_mut();
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .set_node_entry(deps.storage, 8, "a", node_entry())
                    .unwrap();
            }
            let before = NYM_DIRECTORY_CONTRACT_STORAGE
                .load_digest(tester.deps().storage)
                .unwrap();

            // clearing a node that has no entries leaves everything untouched
            try_handle_node_unbonding(tester.deps_mut(), message_info(&mixnet, &[]), 999).unwrap();

            assert_eq!(
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .load_digest(tester.deps().storage)
                    .unwrap(),
                before
            );
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(tester.deps().storage, 8, "a")
                .unwrap()
                .is_some());
        }

        #[test]
        fn real_mixnet_unbond_dispatches_the_callback_and_clears_entries() {
            let mut tester = init_contract_tester_with_node_families();
            let owner = tester.generate_account_with_balance();
            let (node_id, _kp) = tester.bond_dummy_nymnode_for_with_keypair(&owner).unwrap();

            // plant a directory entry for the node
            {
                let deps = tester.deps_mut();
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .set_node_entry(deps.storage, node_id, "sphinx_key", node_entry())
                    .unwrap();
            }
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(tester.deps().storage, node_id, "sphinx_key")
                .unwrap()
                .is_some());

            // a real UnbondNymNode fires the mixnet -> directory OnNymNodeUnbond
            // sub-message, which the App dispatches to the directory contract
            tester
                .execute_mixnet_contract(
                    message_info(&owner, &[]),
                    &nym_mixnet_contract_common::ExecuteMsg::UnbondNymNode {},
                )
                .unwrap();

            // the callback cleared the node's directory entries
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(tester.deps().storage, node_id, "sphinx_key")
                .unwrap()
                .is_none());
        }
    }

    mod node_write_auth {
        use super::*;
        use crate::testing::{
            init_contract_tester, init_contract_tester_with_node_families, sign_node_payload,
        };
        use cosmwasm_std::testing::message_info;
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::{ContractOpts, RandExt};

        // seeded at instantiation from `KnownLabel::SphinxKeys`, max_size 256
        const LABEL: &str = "sphinx_key";

        #[test]
        fn valid_signed_write_is_stored_and_advances_the_sequence() {
            let mut tester = init_contract_tester();
            let (node_id, kp) = tester.bond_dummy_nymnode_with_keypair().unwrap();
            let data = b"payload".to_vec();
            let sig = sign_node_payload(&kp, node_id, LABEL, 0, &data);

            let env = tester.env();
            try_set_node_entry(
                tester.deps_mut(),
                env,
                node_id,
                LABEL.to_string(),
                Binary::from(data),
                0,
                sig,
            )
            .unwrap();

            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(tester.deps().storage, node_id, LABEL)
                .unwrap()
                .is_some());
            // the sequence advanced, so the same signature cannot be replayed
            assert_eq!(
                NYM_DIRECTORY_CONTRACT_STORAGE
                    .current_sequence(tester.deps().storage, node_id)
                    .unwrap(),
                1
            );
        }

        #[test]
        fn invalid_signature_is_rejected() {
            let mut tester = init_contract_tester();
            let (node_id, kp) = tester.bond_dummy_nymnode_with_keypair().unwrap();
            // the signature is over different data than what is submitted
            let sig = sign_node_payload(&kp, node_id, LABEL, 0, b"something else");

            let env = tester.env();
            let err = try_set_node_entry(
                tester.deps_mut(),
                env,
                node_id,
                LABEL.to_string(),
                Binary::from(b"payload".to_vec()),
                0,
                sig,
            )
            .unwrap_err();
            assert_eq!(err, DirectoryContractError::InvalidSignature);
        }

        #[test]
        fn wrong_sequence_is_rejected() {
            let mut tester = init_contract_tester();
            let (node_id, kp) = tester.bond_dummy_nymnode_with_keypair().unwrap();
            let data = b"payload".to_vec();
            // correctly signed, but for a sequence that isn't the expected next (0)
            let sig = sign_node_payload(&kp, node_id, LABEL, 5, &data);

            let env = tester.env();
            let err = try_set_node_entry(
                tester.deps_mut(),
                env,
                node_id,
                LABEL.to_string(),
                Binary::from(data),
                5,
                sig,
            )
            .unwrap_err();
            assert_eq!(
                err,
                DirectoryContractError::InvalidSequence {
                    node_id,
                    expected: 0,
                    provided: 5,
                }
            );
        }

        #[test]
        fn unknown_node_is_rejected() {
            let mut tester = init_contract_tester();
            // a real keypair, but the node id is not bonded
            let (_, kp) = tester.bond_dummy_nymnode_with_keypair().unwrap();
            let unknown = 999;
            let data = b"payload".to_vec();
            let sig = sign_node_payload(&kp, unknown, LABEL, 0, &data);

            let env = tester.env();
            let err = try_set_node_entry(
                tester.deps_mut(),
                env,
                unknown,
                LABEL.to_string(),
                Binary::from(data),
                0,
                sig,
            )
            .unwrap_err();
            assert_eq!(
                err,
                DirectoryContractError::NodeNotBonded { node_id: unknown }
            );
        }

        #[test]
        fn unbonding_node_is_rejected() {
            // needs the node-families contract deployed too, so the mixnet unbond
            // (which hard-calls both node-families and the directory) can succeed
            let mut tester = init_contract_tester_with_node_families();
            let owner = tester.generate_account_with_balance();
            let (node_id, kp) = tester.bond_dummy_nymnode_for_with_keypair(&owner).unwrap();

            // begin unbonding without advancing the epoch: the node stays bonded but
            // is flagged is_unbonding
            tester
                .execute_mixnet_contract(
                    message_info(&owner, &[]),
                    &nym_mixnet_contract_common::ExecuteMsg::UnbondNymNode {},
                )
                .unwrap();

            let data = b"payload".to_vec();
            let sig = sign_node_payload(&kp, node_id, LABEL, 0, &data);
            let env = tester.env();
            let err = try_set_node_entry(
                tester.deps_mut(),
                env,
                node_id,
                LABEL.to_string(),
                Binary::from(data),
                0,
                sig,
            )
            .unwrap_err();
            assert_eq!(err, DirectoryContractError::NodeNotBonded { node_id });
        }

        #[test]
        fn disallowed_label_is_rejected() {
            let mut tester = init_contract_tester();
            let (node_id, kp) = tester.bond_dummy_nymnode_with_keypair().unwrap();
            let data = b"payload".to_vec();
            let sig = sign_node_payload(&kp, node_id, "not-whitelisted", 0, &data);

            let env = tester.env();
            let err = try_set_node_entry(
                tester.deps_mut(),
                env,
                node_id,
                "not-whitelisted".to_string(),
                Binary::from(data),
                0,
                sig,
            )
            .unwrap_err();
            assert_eq!(
                err,
                DirectoryContractError::LabelNotAllowed {
                    label: "not-whitelisted".to_string()
                }
            );
        }

        #[test]
        fn oversized_data_is_rejected() {
            let mut tester = init_contract_tester();
            let (node_id, kp) = tester.bond_dummy_nymnode_with_keypair().unwrap();
            let data = vec![0u8; 257]; // sphinx_key max_size is 128
            let sig = sign_node_payload(&kp, node_id, LABEL, 0, &data);

            let env = tester.env();
            let err = try_set_node_entry(
                tester.deps_mut(),
                env,
                node_id,
                LABEL.to_string(),
                Binary::from(data),
                0,
                sig,
            )
            .unwrap_err();
            assert_eq!(
                err,
                DirectoryContractError::DataTooLarge {
                    label: LABEL.to_string(),
                    len: 257,
                    max: 128,
                }
            );
        }

        #[test]
        fn signed_delete_removes_the_entry_and_replay_after_delete_is_rejected() {
            let mut tester = init_contract_tester();
            let (node_id, kp) = tester.bond_dummy_nymnode_with_keypair().unwrap();
            let data = b"payload".to_vec();

            // write @ seq 0 (advances to 1)
            let set_sig = sign_node_payload(&kp, node_id, LABEL, 0, &data);
            let env = tester.env();
            try_set_node_entry(
                tester.deps_mut(),
                env,
                node_id,
                LABEL.to_string(),
                Binary::from(data.clone()),
                0,
                set_sig,
            )
            .unwrap();

            // delete @ seq 1 (advances to 2), signing the empty-data payload
            let delete_sig = sign_node_payload(&kp, node_id, LABEL, 1, &[]);
            try_delete_node_entry(tester.deps_mut(), node_id, LABEL.to_string(), 1, delete_sig)
                .unwrap();
            assert!(NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(tester.deps().storage, node_id, LABEL)
                .unwrap()
                .is_none());

            // replaying the original seq-0 write is rejected: the sequence moved on
            let replay_sig = sign_node_payload(&kp, node_id, LABEL, 0, &data);
            let env = tester.env();
            let err = try_set_node_entry(
                tester.deps_mut(),
                env,
                node_id,
                LABEL.to_string(),
                Binary::from(data),
                0,
                replay_sig,
            )
            .unwrap_err();
            assert_eq!(
                err,
                DirectoryContractError::InvalidSequence {
                    node_id,
                    expected: 2,
                    provided: 0,
                }
            );
        }
    }
}
