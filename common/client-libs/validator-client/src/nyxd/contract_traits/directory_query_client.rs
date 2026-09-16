// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::{NymContractsProvider, MAX_PINNED_READ_RECORDS};
use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, Height};
use async_trait::async_trait;
use nym_mixnet_contract_common::NodeId;
use serde::Deserialize;

use nym_directory_contract_common::SnapshotIntervalResponse;
pub use nym_directory_contract_common::{
    msg::QueryMsg as DirectoryQueryMsg, AllEntriesPagedResponse, AllowedLabelsResponse,
    AnnotatedNodeLabelEntry, CuratedEntriesPagedResponse, CuratedEntry, CuratedEntryResponse,
    CuratedLabelEntry, DigestResponse, DirectoryEntryRecord, EntryKey, LabelConfig, LabelEntry,
    NodeEntriesPagedResponse, NodeEntriesResponse, NodeEntry, NodeEntryResponse, NodeLabelEntry,
    SequenceResponse,
};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait DirectoryQueryClient {
    async fn query_directory_contract<T>(&self, query: DirectoryQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_admin(&self) -> Result<cw_controllers::AdminResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::Admin {})
            .await
    }

    async fn get_node_entry(
        &self,
        node_id: NodeId,
        label: String,
    ) -> Result<NodeEntryResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::NodeEntry { node_id, label })
            .await
    }

    async fn get_curated_entry(&self, key: String) -> Result<CuratedEntryResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::CuratedEntry { key })
            .await
    }

    async fn get_node_entries(&self, node_id: NodeId) -> Result<NodeEntriesResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::NodeEntries { node_id })
            .await
    }

    async fn get_node_entries_paged(
        &self,
        start_after: Option<(NodeId, String)>,
        limit: Option<u32>,
    ) -> Result<NodeEntriesPagedResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::NodeEntriesPaged { start_after, limit })
            .await
    }

    async fn get_curated_entries_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<CuratedEntriesPagedResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::CuratedEntriesPaged { start_after, limit })
            .await
    }

    async fn get_all_entries(
        &self,
        start_after: Option<EntryKey>,
        limit: Option<u32>,
    ) -> Result<AllEntriesPagedResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::AllEntries { start_after, limit })
            .await
    }

    async fn get_sequence(&self, node_id: NodeId) -> Result<SequenceResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::Sequence { node_id })
            .await
    }

    async fn get_digest(&self) -> Result<DigestResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::Digest {})
            .await
    }

    async fn get_allowed_labels(&self) -> Result<AllowedLabelsResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::AllowedLabels {})
            .await
    }

    async fn get_snapshot_interval(&self) -> Result<SnapshotIntervalResponse, NyxdError> {
        self.query_directory_contract(DirectoryQueryMsg::SnapshotInterval {})
            .await
    }
}

// extension trait for paged queries
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedDirectoryQueryClient: DirectoryQueryClient {
    async fn get_all_node_entries_paged(&self) -> Result<Vec<AnnotatedNodeLabelEntry>, NyxdError> {
        collect_paged!(self, get_node_entries_paged, entries)
    }

    async fn get_all_curated_entries_paged(&self) -> Result<Vec<CuratedLabelEntry>, NyxdError> {
        collect_paged!(self, get_curated_entries_paged, entries)
    }

    async fn get_all_directory_entries(&self) -> Result<Vec<DirectoryEntryRecord>, NyxdError> {
        collect_paged!(self, get_all_entries, entries)
    }
}

#[async_trait]
impl<T> PagedDirectoryQueryClient for T where T: DirectoryQueryClient {}

/// Height-pinned reads, for callers that compare what they read against a digest proven at
/// the same height. Deliberately partial: only the queries a verifying client actually
/// needs pinned are here.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PinnedDirectoryQueryClient {
    /// Every directory entry at exactly `height`.
    ///
    /// Unlike [`PagedDirectoryQueryClient::get_all_directory_entries`], every page is
    /// requested at the same height, so a write landing mid-enumeration cannot duplicate or
    /// drop a record. That is what makes the result safe to fold into an accumulator and
    /// compare against the digest proven at `height`.
    async fn get_all_directory_entries_at_height(
        &self,
        height: Height,
    ) -> Result<Vec<DirectoryEntryRecord>, NyxdError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> PinnedDirectoryQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn get_all_directory_entries_at_height(
        &self,
        height: Height,
    ) -> Result<Vec<DirectoryEntryRecord>, NyxdError> {
        let contract_address = self
            .directory_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("directory contract"))?;

        let mut entries = Vec::new();
        let mut start_after: Option<EntryKey> = None;
        loop {
            let requested_from = start_after.clone();
            let page: AllEntriesPagedResponse = self
                .query_contract_smart_at_height(
                    contract_address,
                    &DirectoryQueryMsg::AllEntries {
                        start_after,
                        limit: None,
                    },
                    Some(height),
                )
                .await?;

            entries.extend(page.entries);
            match page.start_next_after {
                Some(cursor) => {
                    if requested_from.as_ref() == Some(&cursor) {
                        return Err(NyxdError::extension_query_failure(
                            "directory contract",
                            "pagination cursor did not advance",
                        ));
                    }
                    if entries.len() > MAX_PINNED_READ_RECORDS {
                        return Err(NyxdError::extension_query_failure(
                            "directory contract",
                            "paginated read exceeded the maximum record count",
                        ));
                    }
                    start_after = Some(cursor)
                }
                None => break,
            }
        }
        Ok(entries)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> DirectoryQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_directory_contract<T>(&self, query: DirectoryQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let directory_contract_address = &self
            .directory_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("directory contract"))?;
        self.query_contract_smart(directory_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;
    use nym_directory_contract_common::QueryMsg;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: DirectoryQueryClient + Send + Sync>(
        client: C,
        msg: DirectoryQueryMsg,
    ) {
        match msg {
            DirectoryQueryMsg::Admin {} => client.get_admin().ignore(),
            DirectoryQueryMsg::NodeEntry { node_id, label } => {
                client.get_node_entry(node_id, label).ignore()
            }
            DirectoryQueryMsg::CuratedEntry { key } => client.get_curated_entry(key).ignore(),
            DirectoryQueryMsg::NodeEntries { node_id } => client.get_node_entries(node_id).ignore(),
            DirectoryQueryMsg::NodeEntriesPaged { start_after, limit } => {
                client.get_node_entries_paged(start_after, limit).ignore()
            }
            DirectoryQueryMsg::CuratedEntriesPaged { start_after, limit } => client
                .get_curated_entries_paged(start_after, limit)
                .ignore(),
            QueryMsg::AllEntries { start_after, limit } => {
                client.get_all_entries(start_after, limit).ignore()
            }
            DirectoryQueryMsg::Sequence { node_id } => client.get_sequence(node_id).ignore(),
            DirectoryQueryMsg::Digest {} => client.get_digest().ignore(),
            DirectoryQueryMsg::AllowedLabels {} => client.get_allowed_labels().ignore(),
            DirectoryQueryMsg::SnapshotInterval {} => client.get_snapshot_interval().ignore(),
        };
    }
}
