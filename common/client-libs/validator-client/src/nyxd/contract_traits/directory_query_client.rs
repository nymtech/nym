// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use nym_mixnet_contract_common::NodeId;
use serde::Deserialize;

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
        };
    }
}
