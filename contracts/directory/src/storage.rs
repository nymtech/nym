// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, DepsMut, Order, Record, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{
    range_with_prefix, Bound, Item, Key, KeyDeserialize, Map, Path, Prefix, PrimaryKey,
};
use nym_directory_contract_common::constants::storage_keys;
use nym_directory_contract_common::msg::InitialLabel;
use nym_directory_contract_common::{
    CuratedEntry, DirectoryContractError, KnownLabel, LabelConfig, NodeEntry,
};
use nym_mixnet_contract_common::NodeId;
use std::ops::Deref;

pub const NYM_DIRECTORY_CONTRACT_STORAGE: NymDirectoryContractStorage =
    NymDirectoryContractStorage::new();

pub struct NymDirectoryContractStorage {
    /// Admin of the contract; gates privileged operations.
    pub(crate) contract_admin: Admin,

    /// Address of the mixnet contract; used to verify a node id refers to a
    /// real, registered, and bonded node.
    pub(crate) mixnet_contract_address: Item<Addr>,

    pub(crate) sequences: Map<NodeId, u64>,

    pub(crate) allowed_storage_labels: Map<String, LabelConfig>,

    pub(crate) digest_state: Item<[u8; nym_lthash::DIGEST_LEN]>,

    /// Self-published node entries, keyed `(node_id, label)`.
    pub(crate) node_entries: StoredNodeEntries,

    /// Admin-curated entries, keyed by a single admin-chosen path string.
    pub(crate) curated_entries: StoredCuratedEntries,
}

impl NymDirectoryContractStorage {
    #[allow(clippy::new_without_default)]
    pub(crate) const fn new() -> Self {
        NymDirectoryContractStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            mixnet_contract_address: Item::new(storage_keys::MIXNET_CONTRACT_ADDRESS),
            sequences: Map::new(storage_keys::SEQUENCES),
            allowed_storage_labels: Map::new(storage_keys::ALLOWED_LABELS),
            digest_state: Item::new(storage_keys::DIGEST_STATE),
            node_entries: StoredNodeEntries,
            curated_entries: StoredCuratedEntries,
        }
    }

    /// One-time storage initialisation called from the contract's `instantiate`
    /// entry point. Persists the mixnet contract address
    /// and sets `sender` as the contract admin.
    pub(crate) fn initialise(
        &self,
        deps: DepsMut,
        sender: Addr,
        mixnet_contract_address: Addr,
        initial_labels: Vec<InitialLabel>,
    ) -> Result<(), DirectoryContractError> {
        // 1. set mixnet contract address
        self.mixnet_contract_address
            .save(deps.storage, &mixnet_contract_address)?;

        // 2. save known labels
        for label in KnownLabel::ALL {
            self.allowed_storage_labels.save(
                deps.storage,
                label.as_str().to_string(),
                &label.default_config(),
            )?;
        }

        // 3. save additional, provided, labels (if applicable)
        // if there's an overlap with the known labels,
        // prefer the explicitly provided config
        for initial_label in initial_labels {
            self.allowed_storage_labels.save(
                deps.storage,
                initial_label.label.as_str().to_string(),
                &initial_label.config,
            )?;
        }

        // 4. save contract admin (consumes deps)
        self.contract_admin.set(deps, Some(sender))?;

        Ok(())
    }

    pub(crate) fn current_sequence(
        &self,
        store: &dyn Storage,
        node_id: NodeId,
    ) -> Result<u64, DirectoryContractError> {
        Ok(self.sequences.may_load(store, node_id)?.unwrap_or_default())
    }

    pub(crate) fn increment_account_sequence(
        &self,
        store: &mut dyn Storage,
        node_id: NodeId,
    ) -> Result<(), DirectoryContractError> {
        let current_sequence = self.current_sequence(store, node_id)?;
        self.sequences
            .save(store, node_id, &(current_sequence + 1))?;
        Ok(())
    }
}

/// Raw-bytes reader/writer for the node-entry store, keyed `(node_id, label)`.
/// Mirrors the ecash `StoredDeposits` pattern: `cw-storage-plus` owns the key
/// (`Path` for writes, `Prefix` + `KeyDeserialize` for scans, so a client can still
/// reconstruct the raw key for an ICS23 proof) while the value is the compact
/// [`NodeEntry::to_bytes`] codec instead of JSON.
pub(crate) struct StoredNodeEntries;

impl StoredNodeEntries {
    const NAMESPACE: &'static [u8] = storage_keys::NODE_ENTRIES.as_bytes();

    /// The raw storage key for `(node_id, label)`.
    fn storage_key(node_id: NodeId, label: &str) -> Path<Vec<u8>> {
        let key = (node_id, label.to_owned());
        let parts = key.key();
        Path::new(
            Self::NAMESPACE,
            &parts.iter().map(Key::as_ref).collect::<Vec<_>>(),
        )
    }

    /// Scan prefix covering the whole store.
    fn no_prefix() -> Prefix<(NodeId, String), NodeEntry> {
        Prefix::new(Self::NAMESPACE, &[])
    }

    /// Scan prefix covering one node's contiguous `(node_id, *)` range.
    fn node_prefix(node_id: NodeId) -> Prefix<String, NodeEntry> {
        Prefix::new(Self::NAMESPACE, &node_id.key())
    }

    fn decode(kv: Record) -> Result<((NodeId, String), NodeEntry), DirectoryContractError> {
        let (k, v) = kv;
        let key = <(NodeId, String) as KeyDeserialize>::from_vec(k)?;
        Ok((key, NodeEntry::try_from_bytes(&v)?))
    }

    fn decode_label(kv: Record) -> Result<(String, NodeEntry), DirectoryContractError> {
        let (k, v) = kv;
        let label = <String as KeyDeserialize>::from_vec(k)?;
        Ok((label, NodeEntry::try_from_bytes(&v)?))
    }

    /// Create or overwrite a node entry.
    pub(crate) fn save(
        &self,
        store: &mut dyn Storage,
        node_id: NodeId,
        label: &str,
        entry: &NodeEntry,
    ) {
        store.set(&Self::storage_key(node_id, label), &entry.to_bytes());
    }

    /// Load a single node entry; `None` if the slot is empty.
    pub(crate) fn may_load(
        &self,
        store: &dyn Storage,
        node_id: NodeId,
        label: &str,
    ) -> Result<Option<NodeEntry>, DirectoryContractError> {
        match store.get(&Self::storage_key(node_id, label)) {
            Some(bytes) => Ok(Some(NodeEntry::try_from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Delete a node entry. Idempotent - removing an absent key is a no-op.
    pub(crate) fn remove(&self, store: &mut dyn Storage, node_id: NodeId, label: &str) {
        store.remove(&Self::storage_key(node_id, label));
    }

    /// Range over the whole node store, ordered by `(node_id, label)`. The bound is
    /// the full composite key (a `String`-only bound would be compared against keys
    /// led by the node-id length prefix and so would not mean "after this label").
    pub(crate) fn range<'a>(
        &'a self,
        store: &'a dyn Storage,
        min: Option<Bound<'a, (NodeId, String)>>,
        max: Option<Bound<'a, (NodeId, String)>>,
        order: Order,
    ) -> impl Iterator<Item = Result<((NodeId, String), NodeEntry), DirectoryContractError>> + 'a
    {
        let prefix = Self::no_prefix();
        let mapped = range_with_prefix(
            store,
            prefix.deref(),
            min.map(|b| b.to_raw_bound()),
            max.map(|b| b.to_raw_bound()),
            order,
        )
        .map(Self::decode);
        Box::new(mapped)
    }

    /// All of one node's entries (its contiguous label range), unpaginated. Backs the
    /// per-node query and the unbond cleanup.
    pub(crate) fn node_range<'a>(
        &'a self,
        store: &'a dyn Storage,
        node_id: NodeId,
    ) -> impl Iterator<Item = Result<(String, NodeEntry), DirectoryContractError>> + 'a {
        let prefix = Self::node_prefix(node_id);
        let mapped = range_with_prefix(store, prefix.deref(), None, None, Order::Ascending)
            .map(Self::decode_label);
        Box::new(mapped)
    }
}

/// Raw-bytes reader/writer for the curated-entry store, keyed by a single
/// admin-chosen path `String`. Same `Path`/`Prefix` key handling and compact
/// [`CuratedEntry::to_bytes`] value codec as [`StoredNodeEntries`].
pub(crate) struct StoredCuratedEntries;

impl StoredCuratedEntries {
    const NAMESPACE: &'static [u8] = storage_keys::CURATED_ENTRIES.as_bytes();

    fn storage_key(key: &str) -> Path<Vec<u8>> {
        let parts = key.key();
        Path::new(
            Self::NAMESPACE,
            &parts.iter().map(Key::as_ref).collect::<Vec<_>>(),
        )
    }

    fn no_prefix() -> Prefix<String, CuratedEntry> {
        Prefix::new(Self::NAMESPACE, &[])
    }

    fn decode(kv: Record) -> Result<(String, CuratedEntry), DirectoryContractError> {
        let (k, v) = kv;
        let key = <String as KeyDeserialize>::from_vec(k)?;
        Ok((key, CuratedEntry::try_from_bytes(&v)?))
    }

    /// Create or overwrite a curated entry.
    pub(crate) fn save(&self, store: &mut dyn Storage, key: &str, entry: &CuratedEntry) {
        store.set(&Self::storage_key(key), &entry.to_bytes());
    }

    /// Load a single curated entry; `None` if the slot is empty.
    pub(crate) fn may_load(
        &self,
        store: &dyn Storage,
        key: &str,
    ) -> Result<Option<CuratedEntry>, DirectoryContractError> {
        match store.get(&Self::storage_key(key)) {
            Some(bytes) => Ok(Some(CuratedEntry::try_from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Delete a curated entry. Idempotent - removing an absent key is a no-op.
    pub(crate) fn remove(&self, store: &mut dyn Storage, key: &str) {
        store.remove(&Self::storage_key(key));
    }

    pub(crate) fn range<'a>(
        &'a self,
        store: &'a dyn Storage,
        min: Option<Bound<'a, String>>,
        max: Option<Bound<'a, String>>,
        order: Order,
    ) -> impl Iterator<Item = Result<(String, CuratedEntry), DirectoryContractError>> + 'a {
        let prefix = Self::no_prefix();
        let mapped = range_with_prefix(
            store,
            prefix.deref(),
            min.map(|b| b.to_raw_bound()),
            max.map(|b| b.to_raw_bound()),
            order,
        )
        .map(Self::decode);
        Box::new(mapped)
    }
}

pub mod retrieval_limits {
    pub const DEFAULT_NODE_ENTRIES: usize = 50;
    pub const MAX_NODE_ENTRIES: usize = 100;

    pub const DEFAULT_CURATED_ENTRIES: usize = 50;
    pub const MAX_CURATED_ENTRIES: usize = 100;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Binary;

    fn node_entry(sequence: u64) -> NodeEntry {
        NodeEntry {
            data: Binary::new(b"payload".to_vec()),
            updated_at_height: 100,
            sequence,
            signature: Binary::new(vec![7u8; 64]),
        }
    }

    fn curated_entry(data: &[u8]) -> CuratedEntry {
        CuratedEntry {
            data: Binary::new(data.to_vec()),
        }
    }

    #[test]
    fn node_entry_round_trip() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;

        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(store, 1, "sphinx_key")
                .unwrap(),
            None
        );

        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "sphinx_key", &node_entry(0));
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(store, 1, "sphinx_key")
                .unwrap(),
            Some(node_entry(0))
        );

        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .remove(store, 1, "sphinx_key");
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(store, 1, "sphinx_key")
                .unwrap(),
            None
        );
        // removing an absent key is a no-op
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .remove(store, 1, "sphinx_key");
    }

    #[test]
    fn node_entry_overwrite() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "l", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "l", &node_entry(1));
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(store, 1, "l")
                .unwrap()
                .unwrap()
                .sequence,
            1
        );
    }

    #[test]
    fn curated_entry_round_trip() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE.curated_entries.save(
            store,
            "nym-api/1",
            &curated_entry(b"v"),
        );
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .curated_entries
                .may_load(store, "nym-api/1")
                .unwrap(),
            Some(curated_entry(b"v"))
        );
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .remove(store, "nym-api/1");
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .curated_entries
                .may_load(store, "nym-api/1")
                .unwrap(),
            None
        );
    }

    #[test]
    fn node_range_for_single_node_isolates_it() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 7, "a", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 7, "b", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 8, "a", &node_entry(0));

        let seven = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .node_range(store, 7)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            seven.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn node_range_orders_and_bounds() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "a", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 2, "b", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 2, "a", &node_entry(0));

        // full ascending scan, ordered by (node_id, label)
        let all = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .range(store, None, None, Order::Ascending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            all.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            vec![
                (1, "a".to_owned()),
                (2, "a".to_owned()),
                (2, "b".to_owned())
            ]
        );

        // descending order is honoured
        let desc = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .range(store, None, None, Order::Descending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            desc.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            vec![
                (2, "b".to_owned()),
                (2, "a".to_owned()),
                (1, "a".to_owned())
            ]
        );

        // an exclusive lower bound on the full composite key skips past it (and into
        // the next node) - a label-only bound could not express this
        let rest = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .range(
                store,
                Some(Bound::exclusive((1u32, "a".to_owned()))),
                None,
                Order::Ascending,
            )
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rest.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            vec![(2, "a".to_owned()), (2, "b".to_owned())]
        );

        // an inclusive upper bound is respected
        let head = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .range(
                store,
                None,
                Some(Bound::inclusive((2u32, "a".to_owned()))),
                Order::Ascending,
            )
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            head.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            vec![(1, "a".to_owned()), (2, "a".to_owned())]
        );
    }

    #[test]
    fn curated_range_orders_and_bounds() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .save(store, "a", &curated_entry(b"1"));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .save(store, "b", &curated_entry(b"2"));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .save(store, "c", &curated_entry(b"3"));

        let all = NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .range(store, None, None, Order::Ascending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            all.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );

        // exclusive lower bound on the key string
        let rest = NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .range(
                store,
                Some(Bound::exclusive("a".to_owned())),
                None,
                Order::Ascending,
            )
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rest.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            vec!["b", "c"]
        );
    }

    #[test]
    fn stores_do_not_collide_with_other_namespaces() {
        // the two entry stores share the underlying KV store with the sequence and
        // label maps; writes to one must not disturb the others.
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .sequences
            .save(store, 1, &42u64)
            .unwrap();
        NYM_DIRECTORY_CONTRACT_STORAGE
            .allowed_storage_labels
            .save(
                store,
                "sphinx_key".to_owned(),
                &LabelConfig { max_size: 256 },
            )
            .unwrap();

        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "sphinx_key", &node_entry(5));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .save(store, "nym-api", &curated_entry(b"k"));

        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .current_sequence(store, 1)
                .unwrap(),
            42
        );
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .allowed_storage_labels
                .load(store, "sphinx_key".to_owned())
                .unwrap()
                .max_size,
            256
        );
        assert!(NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .may_load(store, 1, "sphinx_key")
            .unwrap()
            .is_some());
        assert!(NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .may_load(store, "nym-api")
            .unwrap()
            .is_some());
        // exactly one entry in each store, nothing leaked between them
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .range(store, None, None, Order::Ascending)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .curated_entries
                .range(store, None, None, Order::Ascending)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .len(),
            1
        );
    }
}
