// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Submitting each epoch's materialised figures to the performance contract.

use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::submission::plan::{EpochPlan, unsent};
use anyhow::Context;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::nyxd::contract_traits::{
    PerformanceQueryClient, PerformanceSigningClient,
};
use nym_validator_client::nyxd::nym_performance_contract_common::EpochId;
use std::num::NonZeroUsize;
use tracing::info;

mod plan;

/// Sends each epoch's figures to the performance contract from the orchestrator's own account,
/// resuming from the contract's cursor rather than from any local record of what landed.
pub(crate) struct PerformanceSubmitter<C> {
    client: C,

    storage: NetworkMonitorStorage,

    /// The orchestrator's account, whose cursor the contract keeps.
    address: AccountId,

    /// Most nodes sent in one transaction.
    batch_size: NonZeroUsize,

    /// Coverage below which a test kind is omitted for every node.
    coverage_threshold: f64,
}

impl<C> PerformanceSubmitter<C>
where
    C: PerformanceQueryClient + PerformanceSigningClient + Sync,
{
    pub(crate) fn new(
        client: C,
        storage: NetworkMonitorStorage,
        address: AccountId,
        batch_size: NonZeroUsize,
        coverage_threshold: f64,
    ) -> Self {
        PerformanceSubmitter {
            client,
            storage,
            address,
            batch_size,
            coverage_threshold,
        }
    }

    /// Sends whatever of `epoch`'s plan the contract's cursor has not passed yet, one chunk per
    /// transaction, and returns `Ok` once nothing is left. A failed chunk ends the step, and the next
    /// one resumes from wherever the cursor then is.
    pub(crate) async fn submit(&self, epoch: EpochId) -> anyhow::Result<()> {
        // no entry means the account was retired after startup
        let cursor = self
            .client
            .get_network_monitor(&self.address)
            .await
            .context("failed to read the submission cursor")?
            .info
            .with_context(|| format!("{} is not an authorised network monitor", self.address))?
            .current_submission_metadata;

        let config_scores = self.storage.get_mixnet_epoch_config_scores(epoch).await?;
        let aggregates = self.storage.get_mixnet_epoch_aggregates(epoch).await?;
        let roles = self.storage.get_described_node_roles().await?;

        let plan = EpochPlan::build(&config_scores, aggregates, &roles, self.coverage_threshold);
        let coverage = plan.coverage;
        PROMETHEUS_METRICS.set(
            PrometheusMetric::PerformanceSubmissionMixnodeLivenessOmitted,
            i64::from(coverage.mixnode_liveness.omitted),
        );
        PROMETHEUS_METRICS.set(
            PrometheusMetric::PerformanceSubmissionGatewayLivenessOmitted,
            i64::from(coverage.gateway_liveness.omitted),
        );
        PROMETHEUS_METRICS.set(
            PrometheusMetric::PerformanceSubmissionStressOmitted,
            i64::from(coverage.stress.omitted),
        );

        let planned = plan.nodes.len();
        let unsent = unsent(plan.nodes, epoch, cursor)?;
        info!(
            epoch,
            planned,
            unsent = unsent.len(),
            ?coverage,
            "submitting to the performance contract"
        );

        for chunk in unsent.chunks(self.batch_size.get()) {
            self.client
                .batch_submit_performance(epoch, chunk.to_vec(), None)
                .await
                .inspect_err(|_| {
                    PROMETHEUS_METRICS.inc(PrometheusMetric::PerformanceSubmissionChunkFailures)
                })
                .with_context(|| format!("failed to submit {} nodes", chunk.len()))?;
            PROMETHEUS_METRICS.inc(PrometheusMetric::PerformanceSubmissionChunksLanded);
        }

        PROMETHEUS_METRICS.set(
            PrometheusMetric::PerformanceSubmissionLastCompleteEpoch,
            i64::from(epoch),
        );
        Ok(())
    }
}

/// The fake contract, shared with the materialiser's tests.
#[cfg(test)]
pub(crate) mod tests {
    pub(crate) use super::plan::tests::{EPOCH, THRESHOLD, config_score};

    use super::plan::tests::aggregate;
    use super::*;
    use crate::storage::models::{FIXTURE_SEEN_AT, TestKind, mixnode};
    use async_trait::async_trait;
    use cosmwasm_std::Addr;
    use nym_validator_client::nyxd::contract_traits::performance_query_client::PerformanceQueryMsg;
    use nym_validator_client::nyxd::cosmwasm_client::types::{ExecuteResult, GasInfo};
    use nym_validator_client::nyxd::error::NyxdError;
    use nym_validator_client::nyxd::nym_performance_contract_common::{
        ExecuteMsg, NetworkMonitorDetails, NetworkMonitorInformation, NetworkMonitorResponse,
        NetworkMonitorSubmissionMetadata, NodeId,
    };
    use nym_validator_client::nyxd::{Coin, Fee, Hash};
    use serde::Deserialize;
    use std::ops::RangeInclusive;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// One `BatchSubmit` the fake contract took.
    #[derive(Debug, PartialEq)]
    pub(crate) struct Batch {
        epoch: EpochId,
        node_ids: Vec<NodeId>,
    }

    /// A batch of `node_ids` under [`EPOCH`].
    fn batch(node_ids: RangeInclusive<NodeId>) -> Batch {
        Batch {
            epoch: EPOCH,
            node_ids: node_ids.collect(),
        }
    }

    #[derive(Default)]
    struct FakeContractState {
        /// `None` while the account is not an authorised network monitor.
        cursor: Option<NetworkMonitorSubmissionMetadata>,

        batches: Vec<Batch>,

        /// Fails the batch arriving after this many were taken, once.
        fail_after: Option<usize>,
    }

    /// The performance contract as the submitter sees it: it answers the account's cursor, takes each
    /// batch and moves the cursor to its last node, as the contract does, and can fail one batch.
    #[derive(Clone, Default)]
    pub(crate) struct FakeContract(Arc<Mutex<FakeContractState>>);

    impl FakeContract {
        pub(crate) fn with_cursor(epoch: EpochId, node_id: NodeId) -> Self {
            let contract = FakeContract::default();
            contract.0.lock().unwrap().cursor = Some(NetworkMonitorSubmissionMetadata {
                last_submitted_epoch_id: epoch,
                last_submitted_node_id: node_id,
            });
            contract
        }

        fn fail_after(self, batches: usize) -> Self {
            self.0.lock().unwrap().fail_after = Some(batches);
            self
        }

        pub(crate) fn batches(&self) -> Vec<Batch> {
            std::mem::take(&mut self.0.lock().unwrap().batches)
        }
    }

    #[async_trait]
    impl PerformanceQueryClient for FakeContract {
        async fn query_performance_contract<T>(
            &self,
            query: PerformanceQueryMsg,
        ) -> Result<T, NyxdError>
        where
            for<'a> T: Deserialize<'a>,
        {
            let PerformanceQueryMsg::NetworkMonitor { address } = query else {
                panic!("the submitter only reads its cursor, got {query:?}")
            };
            let info =
                self.0
                    .lock()
                    .unwrap()
                    .cursor
                    .clone()
                    .map(|cursor| NetworkMonitorInformation {
                        details: NetworkMonitorDetails {
                            address: Addr::unchecked(address),
                            authorised_by: Addr::unchecked("admin"),
                            authorised_at_height: 1,
                        },
                        current_submission_metadata: cursor,
                    });

            // the trait is generic over the response, so it goes through its wire form
            let response = cosmwasm_std::to_json_vec(&NetworkMonitorResponse { info }).unwrap();
            Ok(cosmwasm_std::from_json(response).unwrap())
        }
    }

    #[async_trait]
    impl PerformanceSigningClient for FakeContract {
        async fn execute_performance_contract(
            &self,
            _: Option<Fee>,
            msg: ExecuteMsg,
            _: String,
            _: Vec<Coin>,
        ) -> Result<ExecuteResult, NyxdError> {
            let ExecuteMsg::BatchSubmit { epoch, data } = msg else {
                panic!("the submitter only sends batches, got {msg:?}")
            };

            let mut state = self.0.lock().unwrap();
            if state.fail_after == Some(state.batches.len()) {
                state.fail_after = None;
                return Err(NyxdError::BroadcastTimeout {
                    hash: Hash::None,
                    timeout: Duration::from_secs(60),
                });
            }

            if let (Some(cursor), Some(last)) = (&mut state.cursor, data.last()) {
                cursor.last_submitted_epoch_id = epoch;
                cursor.last_submitted_node_id = last.node_id;
            }
            state.batches.push(Batch {
                epoch,
                node_ids: data.iter().map(|node| node.node_id).collect(),
            });

            Ok(ExecuteResult {
                logs: Vec::new(),
                msg_responses: Vec::new(),
                events: Vec::new(),
                transaction_hash: Hash::None,
                gas_info: GasInfo::default(),
            })
        }
    }

    /// Storage holding `nodes` described mixnodes, each with a config score and a stress aggregate
    /// for [`EPOCH`], so every one of them is planned.
    async fn storage_with_epoch(nodes: i64) -> NetworkMonitorStorage {
        let storage = NetworkMonitorStorage::in_memory().await;
        let described: Vec<_> = (1..=nodes).map(mixnode).collect();
        let config_scores: Vec<_> = (1..=nodes).map(|node| config_score(node, 1.0)).collect();
        let aggregates: Vec<_> = (1..=nodes)
            .map(|node| aggregate(node, TestKind::MixnodeStress, 1.0))
            .collect();

        storage
            .store_refresh(&described, FIXTURE_SEEN_AT)
            .await
            .unwrap();
        storage
            .batch_insert_mixnet_epoch_config_scores(&config_scores)
            .await
            .unwrap();
        storage
            .batch_insert_mixnet_epoch_aggregates(&aggregates)
            .await
            .unwrap();
        storage
    }

    /// A submitter with the default batch size over 450 planned nodes.
    async fn submitter(contract: &FakeContract) -> PerformanceSubmitter<FakeContract> {
        PerformanceSubmitter::new(
            contract.clone(),
            storage_with_epoch(450).await,
            AccountId::new("n", &[1; 20]).unwrap(),
            NonZeroUsize::new(200).unwrap(),
            THRESHOLD,
        )
    }

    // a monitor authorised before this epoch has sent nothing of it
    #[tokio::test]
    async fn the_plan_goes_out_in_ascending_chunks_of_the_batch_size() {
        let contract = FakeContract::with_cursor(EPOCH - 1, 9);

        submitter(&contract).await.submit(EPOCH).await.unwrap();

        assert_eq!(
            contract.batches(),
            vec![batch(1..=200), batch(201..=400), batch(401..=450)]
        );
    }

    // a restart mid-epoch knows what landed from the chain alone
    #[tokio::test]
    async fn a_cursor_mid_epoch_resumes_after_it() {
        let contract = FakeContract::with_cursor(EPOCH, 300);

        submitter(&contract).await.submit(EPOCH).await.unwrap();

        assert_eq!(contract.batches(), vec![batch(301..=450)]);
    }

    #[tokio::test]
    async fn a_failed_chunk_stops_the_step_and_the_next_one_resumes_after_what_landed() {
        let contract = FakeContract::with_cursor(EPOCH - 1, 0).fail_after(1);
        let submitter = submitter(&contract).await;

        assert!(submitter.submit(EPOCH).await.is_err());
        assert_eq!(
            contract.batches(),
            vec![batch(1..=200)],
            "a chunk was sent after one had failed"
        );

        submitter.submit(EPOCH).await.unwrap();
        assert_eq!(contract.batches(), vec![batch(201..=400), batch(401..=450)]);
    }

    // the account was retired after startup, so every transaction would be rejected
    #[tokio::test]
    async fn no_network_monitor_entry_fails_without_sending() {
        let contract = FakeContract::default();

        assert!(submitter(&contract).await.submit(EPOCH).await.is_err());
        assert!(contract.batches().is_empty());
    }

    // a restart after the epoch had fully landed
    #[tokio::test]
    async fn a_cursor_at_the_last_planned_node_sends_nothing_and_completes() {
        let contract = FakeContract::with_cursor(EPOCH, 450);

        submitter(&contract).await.submit(EPOCH).await.unwrap();

        assert!(contract.batches().is_empty());
    }

    // the interval reading is behind the chain, so nothing planned for it can land
    #[tokio::test]
    async fn a_cursor_at_a_later_epoch_fails_without_sending() {
        let contract = FakeContract::with_cursor(EPOCH + 1, 0);

        assert!(submitter(&contract).await.submit(EPOCH).await.is_err());
        assert!(contract.batches().is_empty());
    }
}
