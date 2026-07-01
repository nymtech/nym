// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Read-only query handlers.
//!
//! These are smart queries and produce no proofs. Provable reads come from RAW store
//! reads (the digest under `storage_keys::DIGEST_STATE`, and individual entries under
//! the per-class `Path` layout) verified against the app_hash off-chain.

use crate::storage::{retrieval_limits, NYM_DIRECTORY_CONTRACT_STORAGE};
use cosmwasm_std::{Binary, Deps, Order, StdResult};
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_directory_contract_common::{
    AllEntriesPagedResponse, AllowedLabelsResponse, AnnotatedNodeLabelEntry,
    CuratedEntriesPagedResponse, CuratedEntryResponse, CuratedLabelEntry, DigestResponse,
    DirectoryContractError, EntryKey, LabelEntry, NodeEntriesPagedResponse, NodeEntriesResponse,
    NodeEntryResponse, NodeLabelEntry, SequenceResponse,
};
use nym_mixnet_contract_common::NodeId;

/// The current contract admin.
pub(crate) fn query_admin(deps: Deps) -> Result<AdminResponse, DirectoryContractError> {
    Ok(NYM_DIRECTORY_CONTRACT_STORAGE
        .contract_admin
        .query_admin(deps)?)
}

/// A single node entry, or `None` if the slot is empty.
pub(crate) fn query_node_entry(
    deps: Deps,
    node_id: NodeId,
    label: String,
) -> Result<NodeEntryResponse, DirectoryContractError> {
    let entry =
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .may_load(deps.storage, node_id, &label)?;
    Ok(NodeEntryResponse { entry })
}

/// A single curated entry, or `None` if the slot is empty.
pub(crate) fn query_curated_entry(
    deps: Deps,
    key: String,
) -> Result<CuratedEntryResponse, DirectoryContractError> {
    let entry = NYM_DIRECTORY_CONTRACT_STORAGE
        .curated_entries
        .may_load(deps.storage, &key)?;
    Ok(CuratedEntryResponse { entry })
}

/// Every entry belonging to one node (its contiguous label range), unpaginated -
/// bounded by the governed label set.
pub(crate) fn query_node_entries(
    deps: Deps,
    node_id: NodeId,
) -> Result<NodeEntriesResponse, DirectoryContractError> {
    let entries = NYM_DIRECTORY_CONTRACT_STORAGE
        .node_entries
        .node_range(deps.storage, node_id)
        .map(|res| res.map(|(label, entry)| NodeLabelEntry { label, entry }))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(NodeEntriesResponse { node_id, entries })
}

/// A page of node entries across every node, ascending by `(node_id, label)`.
/// `start_after` is exclusive (pass the previous page's `start_next_after`); `limit`
/// defaults to [`retrieval_limits::DEFAULT_NODE_ENTRIES`], clamped to the max.
pub(crate) fn query_node_entries_paged(
    deps: Deps,
    start_after: Option<(NodeId, String)>,
    limit: Option<u32>,
) -> Result<NodeEntriesPagedResponse, DirectoryContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::DEFAULT_NODE_ENTRIES)
        .min(retrieval_limits::MAX_NODE_ENTRIES) as usize;

    let start = start_after.map(Bound::exclusive);

    let entries = NYM_DIRECTORY_CONTRACT_STORAGE
        .node_entries
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|((node_id, label), entry)| AnnotatedNodeLabelEntry {
                node_id,
                label,
                entry,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let start_next_after = entries
        .last()
        .map(|entry| (entry.node_id, entry.label.clone()));

    Ok(NodeEntriesPagedResponse {
        entries,
        start_next_after,
    })
}

/// A page of curated entries, ascending by key. `start_after` is exclusive (pass the
/// previous page's `start_next_after`); `limit` defaults to
/// [`retrieval_limits::DEFAULT_CURATED_ENTRIES`], clamped to the max.
pub(crate) fn query_curated_entries_paged(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<CuratedEntriesPagedResponse, DirectoryContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::DEFAULT_CURATED_ENTRIES)
        .min(retrieval_limits::MAX_CURATED_ENTRIES) as usize;

    let start = start_after.map(Bound::exclusive);

    let entries = NYM_DIRECTORY_CONTRACT_STORAGE
        .curated_entries
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(label, entry)| CuratedLabelEntry { label, entry }))
        .collect::<Result<Vec<_>, _>>()?;

    let start_next_after = entries.last().map(|entry| entry.label.clone());

    Ok(CuratedEntriesPagedResponse {
        entries,
        start_next_after,
    })
}

/// A page of the whole directory - node entries first, then curated - the global
/// pull a client uses to recompute and verify the digest. The two classes are
/// returned back-to-back (not interleaved), which is sound because the digest is an
/// order-independent multiset. `start_after` is exclusive and its class decides where
/// the page resumes; `limit` defaults to [`retrieval_limits::DEFAULT_ALL_ENTRIES`],
/// clamped to the max.
pub(crate) fn query_all_entries(
    deps: Deps,
    start_after: Option<EntryKey>,
    limit: Option<u32>,
) -> Result<AllEntriesPagedResponse, DirectoryContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::DEFAULT_ALL_ENTRIES)
        .min(retrieval_limits::MAX_ALL_ENTRIES) as usize;

    // we start the scan from the node entries, so if somebody asked for curated start,
    // we don't have to do more checks regarding ranges or further queries
    let mut partial_res: AllEntriesPagedResponse = match start_after {
        None => query_node_entries_paged(deps, None, Some(limit as u32))?.into(),
        Some(EntryKey::Node { node_id, label }) => {
            query_node_entries_paged(deps, Some((node_id, label)), Some(limit as u32))?.into()
        }
        Some(EntryKey::Curated { key }) => {
            return Ok(query_curated_entries_paged(deps, Some(key), Some(limit as u32))?.into())
        }
    };

    // no more to pull this iteration
    if partial_res.entries.len() >= limit {
        return Ok(partial_res);
    }

    let remaining_slots = limit - partial_res.entries.len();
    let mut curated: AllEntriesPagedResponse =
        query_curated_entries_paged(deps, None, Some(remaining_slots as u32))?.into();

    partial_res.entries.append(&mut curated.entries);
    partial_res.start_next_after = curated.start_next_after;

    Ok(partial_res)
}

/// The next sequence a node must sign with.
pub(crate) fn query_sequence(
    deps: Deps,
    node_id: NodeId,
) -> Result<SequenceResponse, DirectoryContractError> {
    let next_sequence = NYM_DIRECTORY_CONTRACT_STORAGE.current_sequence(deps.storage, node_id)?;
    Ok(SequenceResponse { next_sequence })
}

/// The compact 32-byte global digest (the BLAKE3 collapse of the LtHash accumulator).
pub(crate) fn query_digest(deps: Deps) -> Result<DigestResponse, DirectoryContractError> {
    let accumulator = NYM_DIRECTORY_CONTRACT_STORAGE.load_digest(deps.storage)?;
    Ok(DigestResponse {
        digest: Binary::new(accumulator.out().to_vec()),
    })
}

/// The label whitelist with per-label sizes.
pub(crate) fn query_allowed_labels(
    deps: Deps,
) -> Result<AllowedLabelsResponse, DirectoryContractError> {
    let labels = NYM_DIRECTORY_CONTRACT_STORAGE
        .allowed_storage_labels
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| res.map(|(label, config)| LabelEntry { label, config }))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(AllowedLabelsResponse { labels })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{init_contract_tester, DirectoryContractTesterExt};
    use nym_contracts_common_testing::ContractOpts;

    #[test]
    fn admin_query_returns_the_instantiator() {
        let tester = init_contract_tester();
        assert!(query_admin(tester.deps()).unwrap().admin.is_some());
    }

    #[test]
    fn allowed_labels_query_includes_the_seeded_sphinx_key() {
        let tester = init_contract_tester();
        let resp = query_allowed_labels(tester.deps()).unwrap();
        assert!(resp.labels.iter().any(|l| l.label == "sphinx_key"));
    }

    #[test]
    fn node_entry_query_round_trips() {
        let mut tester = init_contract_tester();
        tester.add_dummy_node_data(1, "sphinx_key");
        assert!(query_node_entry(tester.deps(), 1, "sphinx_key".to_string())
            .unwrap()
            .entry
            .is_some());
        assert!(query_node_entry(tester.deps(), 2, "sphinx_key".to_string())
            .unwrap()
            .entry
            .is_none());
    }

    #[test]
    fn curated_entry_query_round_trips() {
        let mut tester = init_contract_tester();
        tester.add_dummy_curated("nym-api/1");
        assert!(query_curated_entry(tester.deps(), "nym-api/1".to_string())
            .unwrap()
            .entry
            .is_some());
        assert!(query_curated_entry(tester.deps(), "missing".to_string())
            .unwrap()
            .entry
            .is_none());
    }

    #[test]
    fn node_entries_query_returns_all_labels_for_a_node() {
        let mut tester = init_contract_tester();
        tester.add_dummy_node_data(7, "a");
        tester.add_dummy_node_data(7, "b");
        tester.add_dummy_node_data(8, "a");
        let resp = query_node_entries(tester.deps(), 7).unwrap();
        assert_eq!(resp.node_id, 7);
        assert_eq!(
            resp.entries
                .iter()
                .map(|e| e.label.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn sequence_query_reports_the_expected_next() {
        let mut tester = init_contract_tester();
        assert_eq!(query_sequence(tester.deps(), 1).unwrap().next_sequence, 0);

        let deps = tester.deps_mut();
        NYM_DIRECTORY_CONTRACT_STORAGE
            .increment_account_sequence(deps.storage, 1)
            .unwrap();

        assert_eq!(query_sequence(tester.deps(), 1).unwrap().next_sequence, 1);
    }

    #[test]
    fn digest_query_is_32_bytes_and_tracks_entries() {
        let mut tester = init_contract_tester();
        let empty = query_digest(tester.deps()).unwrap().digest;
        assert_eq!(empty.len(), 32);

        tester.add_dummy_curated("k");
        let after = query_digest(tester.deps()).unwrap().digest;
        assert_eq!(after.len(), 32);
        assert_ne!(after, empty);
    }

    #[test]
    fn node_entries_paged_query_orders_and_paginates() {
        let mut tester = init_contract_tester();
        tester.add_dummy_node_data(1, "a");
        tester.add_dummy_node_data(2, "a");
        tester.add_dummy_node_data(2, "b");

        // full scan, ascending by (node_id, label)
        let all = query_node_entries_paged(tester.deps(), None, None).unwrap();
        assert_eq!(
            all.entries
                .iter()
                .map(|e| (e.node_id, e.label.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "a"), (2, "a"), (2, "b")]
        );

        // a page + exclusive composite cursor
        let page = query_node_entries_paged(tester.deps(), None, Some(2)).unwrap();
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.start_next_after, Some((2, "a".to_string())));

        let rest = query_node_entries_paged(tester.deps(), page.start_next_after, Some(2)).unwrap();
        assert_eq!(
            rest.entries
                .iter()
                .map(|e| (e.node_id, e.label.as_str()))
                .collect::<Vec<_>>(),
            vec![(2, "b")]
        );
    }

    #[test]
    fn curated_entries_paged_query_paginates() {
        let mut tester = init_contract_tester();
        tester.add_dummy_curated("a");
        tester.add_dummy_curated("b");
        tester.add_dummy_curated("c");

        let page = query_curated_entries_paged(tester.deps(), None, Some(2)).unwrap();
        assert_eq!(
            page.entries
                .iter()
                .map(|e| e.label.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(page.start_next_after.as_deref(), Some("b"));

        let rest =
            query_curated_entries_paged(tester.deps(), page.start_next_after, Some(2)).unwrap();
        assert_eq!(
            rest.entries
                .iter()
                .map(|e| e.label.as_str())
                .collect::<Vec<_>>(),
            vec!["c"]
        );
    }

    #[test]
    fn all_entries_query_stitches_nodes_then_curated_and_paginates() {
        let mut tester = init_contract_tester();
        tester.add_dummy_node_data(1, "a");
        tester.add_dummy_node_data(2, "a");
        tester.add_dummy_curated("x");
        tester.add_dummy_curated("y");

        let node1 = EntryKey::Node {
            node_id: 1,
            label: "a".into(),
        };
        let node2 = EntryKey::Node {
            node_id: 2,
            label: "a".into(),
        };
        let cur_x = EntryKey::Curated { key: "x".into() };
        let cur_y = EntryKey::Curated { key: "y".into() };

        // full pull: node entries first, then curated
        let all = query_all_entries(tester.deps(), None, None).unwrap();
        assert_eq!(
            all.entries
                .iter()
                .map(|r| r.key.clone())
                .collect::<Vec<_>>(),
            vec![node1, node2, cur_x.clone(), cur_y.clone()]
        );

        // a page spanning the node -> curated boundary, then resume via the cursor
        let page = query_all_entries(tester.deps(), None, Some(3)).unwrap();
        assert_eq!(page.entries.len(), 3);
        assert_eq!(page.entries[2].key, cur_x);
        let rest = query_all_entries(tester.deps(), page.start_next_after, Some(3)).unwrap();
        assert_eq!(
            rest.entries
                .iter()
                .map(|r| r.key.clone())
                .collect::<Vec<_>>(),
            vec![cur_y]
        );
    }

    #[test]
    fn resetting_identical_curated_data_leaves_the_digest_unchanged() {
        let mut tester = init_contract_tester();
        tester.add_dummy_curated("k");
        let after_first = query_digest(tester.deps()).unwrap().digest;

        tester.add_dummy_curated("k");
        assert_eq!(query_digest(tester.deps()).unwrap().digest, after_first);
    }

    #[test]
    fn digest_recomputes_from_all_entries() {
        let mut tester = init_contract_tester();
        tester.add_dummy_node_data(1, "a");
        tester.add_dummy_node_data(2, "a");
        tester.add_dummy_node_data(2, "b");
        tester.add_dummy_curated("x");
        tester.add_dummy_curated("y");

        let mut recomputed = nym_lthash::LtHash16::new();
        for record in query_all_entries(tester.deps(), None, None)
            .unwrap()
            .entries
        {
            recomputed.add(&record.digest_leaf());
        }

        assert_eq!(
            recomputed.out().to_vec(),
            query_digest(tester.deps()).unwrap().digest
        );
    }
}
