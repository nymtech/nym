// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use nym_geolocation_contract_common::{
    AllRecordsPagedResponse, ConfigResponse, DigestResponse, EntryResponse, GeolocationRecord,
    QueryMsg as GeolocationQueryMsg, RecordKey, Source, Subject, SubjectEntriesResponse,
    WhitelistResponse,
};
use nym_mixnet_contract_common::NodeId;
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GeolocationQueryClient {
    async fn query_geolocation_contract<T>(
        &self,
        query: GeolocationQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_admin(&self) -> Result<cw_controllers::AdminResponse, NyxdError> {
        self.query_geolocation_contract(GeolocationQueryMsg::Admin {})
            .await
    }

    async fn get_geolocation_config(&self) -> Result<ConfigResponse, NyxdError> {
        self.query_geolocation_contract(GeolocationQueryMsg::Config {})
            .await
    }

    async fn get_location_entry(
        &self,
        subject: Subject,
        source: Source,
    ) -> Result<EntryResponse, NyxdError> {
        self.query_geolocation_contract(GeolocationQueryMsg::Entry { subject, source })
            .await
    }

    async fn get_subject_entries(
        &self,
        subject: Subject,
    ) -> Result<SubjectEntriesResponse, NyxdError> {
        self.query_geolocation_contract(GeolocationQueryMsg::SubjectEntries { subject })
            .await
    }

    async fn get_nym_node_entries(
        &self,
        node_id: NodeId,
    ) -> Result<SubjectEntriesResponse, NyxdError> {
        self.query_geolocation_contract(GeolocationQueryMsg::NymNodeEntries { node_id })
            .await
    }

    async fn get_subject_measurements(
        &self,
        subject: Subject,
    ) -> Result<SubjectEntriesResponse, NyxdError> {
        self.query_geolocation_contract(GeolocationQueryMsg::SubjectMeasurements { subject })
            .await
    }

    async fn get_all_geolocation_records_paged(
        &self,
        start_after: Option<RecordKey>,
        limit: Option<u32>,
    ) -> Result<AllRecordsPagedResponse, NyxdError> {
        self.query_geolocation_contract(GeolocationQueryMsg::AllRecords { start_after, limit })
            .await
    }

    /// The 32-byte collapse of the contract's accumulator.
    ///
    /// A convenience for comparing digests, and unproven: smart queries carry no proof. A client
    /// that needs one performs a raw store read at the contract's digest key instead.
    async fn get_geolocation_digest(&self) -> Result<DigestResponse, NyxdError> {
        self.query_geolocation_contract(GeolocationQueryMsg::Digest {})
            .await
    }

    /// The agent whitelist, which a reader needs in full before it can decide which measured
    /// entries to honour: authorisation is evaluated at read time against this set.
    async fn get_geolocation_whitelist(&self) -> Result<WhitelistResponse, NyxdError> {
        self.query_geolocation_contract(GeolocationQueryMsg::Whitelist {})
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedGeolocationQueryClient: GeolocationQueryClient {
    /// Every digest-committed record, across both entry classes.
    ///
    /// This is what a verifying client folds to recompute the accumulator for itself. Note that
    /// the pages are pulled one query at a time and are therefore *not* guaranteed to come from
    /// a single height; anything comparing the result against a proven digest has to pin the
    /// height itself.
    async fn get_all_geolocation_records(&self) -> Result<Vec<GeolocationRecord>, NyxdError> {
        collect_paged!(self, get_all_geolocation_records_paged, records)
    }
}

#[async_trait]
impl<T> PagedGeolocationQueryClient for T where T: GeolocationQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> GeolocationQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_geolocation_contract<T>(
        &self,
        query: GeolocationQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let contract_address = &self
            .geolocation_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("geolocation contract"))?;
        self.query_contract_smart(contract_address, &query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: GeolocationQueryClient + Send + Sync>(
        client: C,
        msg: GeolocationQueryMsg,
    ) {
        match msg {
            GeolocationQueryMsg::Admin {} => client.get_admin().ignore(),
            GeolocationQueryMsg::Config {} => client.get_geolocation_config().ignore(),
            GeolocationQueryMsg::Entry { subject, source } => {
                client.get_location_entry(subject, source).ignore()
            }
            GeolocationQueryMsg::SubjectEntries { subject } => {
                client.get_subject_entries(subject).ignore()
            }
            GeolocationQueryMsg::NymNodeEntries { node_id } => {
                client.get_nym_node_entries(node_id).ignore()
            }
            GeolocationQueryMsg::SubjectMeasurements { subject } => {
                client.get_subject_measurements(subject).ignore()
            }
            GeolocationQueryMsg::AllRecords { .. } => client
                .get_all_geolocation_records_paged(None, None)
                .ignore(),
            GeolocationQueryMsg::Digest {} => client.get_geolocation_digest().ignore(),
            GeolocationQueryMsg::Whitelist {} => client.get_geolocation_whitelist().ignore(),
        };
    }
}
