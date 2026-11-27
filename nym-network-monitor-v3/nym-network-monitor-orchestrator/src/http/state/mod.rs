// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::aggregation::{NodeEpochFigures, ReportedAggregates, epoch_figures};
use crate::http::api::v1::error::ApiError;
use crate::orchestrator::config::Config;
use crate::signing_client::SigningClient;
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{
    CompletedTestRun, MixnetEpochAggregate, MixnetEpochConfigScore, MixnetEpochFiveHopLiveness,
    TestKind,
};
use axum::extract::FromRef;
use nym_network_monitor_orchestrator_requests::models::{
    FiveHopMixnetResult, NodeEpochAggregates, NymNodeData, NymNodeWithTestRuns, PagedResult,
    Pagination, RouteSessionData, TestRunData, TestRunInProgressData, TestRunResult,
    WorkAssignment,
};
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_mixnet_contract_common::EpochId;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use tracing::error;

pub(crate) mod agents;
mod five_hop_mixnet_session_export;
pub(crate) mod work;

use agents::KnownAgents;
use work::WorkManager;

/// Lifts a stored run into its public shape. A value that will not decode is corruption, for the
/// same reason as [`work::malformed_target`].
fn testrun_data(completed: CompletedTestRun) -> Result<TestRunData, ApiError> {
    completed.try_into().map_err(|err: anyhow::Error| {
        error!("could not build a testrun out of a stored row: {err}");
        ApiError::MalformedStoredData
    })
}

/// Shared application state available to all axum request handlers.
#[derive(Clone, FromRef)]
pub(crate) struct AppState {
    pub(crate) agents: KnownAgents,

    pub(crate) work_manager: WorkManager,

    pub(crate) storage: NetworkMonitorStorage,

    pub(crate) validator_client: SigningClient,
}

impl AppState {
    pub(crate) fn new(
        config: &Config,
        agents: KnownAgents,
        storage: NetworkMonitorStorage,
        validator_client: SigningClient,
    ) -> Self {
        AppState {
            agents,
            storage,
            work_manager: WorkManager {
                one_hop_mixnet_schedules: config.schedules(),
                five_hop_mixnet: config.five_hop_mixnet_run_config(),
            },
            validator_client,
        }
    }

    /// Hands the requesting agent a due route run, else one assignment of whichever enabled one-hop
    /// kind is most overdue. `None` when nothing is due.
    pub(crate) async fn assign_work(&self) -> Result<Option<WorkAssignment>, ApiError> {
        self.work_manager.assign_work(&self.storage).await
    }

    /// Persists a completed test run result with its measurements, under the kind the orchestrator
    /// dispatched.
    pub(crate) async fn submit_one_hop_mixnet_result(
        &self,
        result: TestRunResult,
        node_id: NodeId,
        tested_address: SocketAddr,
    ) -> Result<(), ApiError> {
        self.work_manager
            .submit_one_hop_mixnet_result(&self.storage, result, node_id, tested_address)
            .await
    }

    /// Stores a route run's sessions, or rejects the submission whole.
    pub(crate) async fn submit_five_hop_mixnet_result(
        &self,
        result: FiveHopMixnetResult,
    ) -> Result<(), ApiError> {
        self.work_manager
            .submit_five_hop_mixnet_result(&self.storage, result)
            .await
    }

    /// Backs `GET /v1/results/testrun/{kind}/{id}`. `Ok(None)` means the row
    /// doesn't exist (the handler maps this to a 404); storage errors are logged
    /// and collapsed to [`ApiError::StorageFailure`].
    pub(crate) async fn get_testrun_by_id(
        &self,
        kind: TestKind,
        id: i64,
    ) -> Result<Option<TestRunData>, ApiError> {
        let result = match self.storage.get_testrun_by_id(kind, id).await {
            Err(err) => {
                error!("get_testrun_by_id storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(None) => return Ok(None),
            Ok(Some(testrun)) => testrun,
        };

        Ok(Some(testrun_data(result)?))
    }

    /// Backs `GET /v1/results/nym-node/{node_id}`. If the node is known, its
    /// snapshot is returned along with its most recent completed run of each kind,
    /// each `None` when that kind has no such run.
    ///
    /// Malformed stored data (e.g. an unparsable base58 key) is surfaced as
    /// [`ApiError::MalformedStoredData`]; this should never happen in practice
    /// because the orchestrator writes these fields itself.
    pub(crate) async fn get_nym_node_by_id(
        &self,
        node_id: NodeId,
    ) -> Result<Option<NymNodeWithTestRuns>, ApiError> {
        let nym_node = match self.storage.get_nym_node_by_id(node_id).await {
            Err(err) => {
                error!("get_nym_node_by_id storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(None) => return Ok(None),
            Ok(Some(nym_node)) => nym_node,
        };

        Ok(Some(NymNodeWithTestRuns {
            node: nym_node.try_into().map_err(|err| {
                error!("get_nym_node_by_id malformed stored data: {err}");
                ApiError::MalformedStoredData
            })?,
            latest_mixnode_liveness: self
                .get_latest_testrun_for_node(TestKind::MixnodeLiveness, node_id)
                .await?,
            latest_gateway_liveness: self
                .get_latest_testrun_for_node(TestKind::GatewayLiveness, node_id)
                .await?,
            latest_mixnode_stress: self
                .get_latest_testrun_for_node(TestKind::MixnodeStress, node_id)
                .await?,
        }))
    }

    /// The newest completed run of `kind` against a node, for [`Self::get_nym_node_by_id`].
    async fn get_latest_testrun_for_node(
        &self,
        kind: TestKind,
        node_id: NodeId,
    ) -> Result<Option<TestRunData>, ApiError> {
        match self
            .storage
            .get_latest_testrun_for_node(kind, node_id)
            .await
        {
            Err(err) => {
                error!("get_latest_testrun_for_node storage failure: {err}");
                Err(ApiError::StorageFailure)
            }
            Ok(latest) => latest.map(testrun_data).transpose(),
        }
    }

    /// Backs `GET /v1/results/testruns-in-progress`. Returns a page of rows
    /// from `testrun_in_progress` ordered oldest `started_at` first so stale
    /// runs surface at the top.
    pub(crate) async fn get_testruns_in_progress_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunInProgressData>, ApiError> {
        let (in_progress, total) = match self
            .storage
            .get_testruns_in_progress_paginated(pagination)
            .await
        {
            Err(err) => {
                error!("get_testruns_in_progress_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(result) => result,
        };

        Ok(PagedResult {
            page: pagination.page(),
            per_page: in_progress.len(),
            total,
            items: in_progress.into_iter().map(Into::into).collect(),
        })
    }

    /// Backs `GET /v1/results/testruns/{kind}`. Returns a single page of the
    /// kind's completed runs ordered newest first, together with the total row
    /// count at the time the page was read (fetched in the same transaction as
    /// the page itself for consistency).
    pub(crate) async fn get_testruns_paginated(
        &self,
        kind: TestKind,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunData>, ApiError> {
        let (testruns, total) = match self.storage.get_testruns_paginated(kind, pagination).await {
            Err(err) => {
                error!("get_testruns_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(testruns) => testruns,
        };

        Ok(PagedResult {
            page: pagination.page(),
            per_page: testruns.len(),
            total,
            items: testruns
                .into_iter()
                .map(testrun_data)
                .collect::<Result<_, _>>()?,
        })
    }

    /// Backs `GET /v1/results/nym-nodes`. Returns a page of nodes ordered by
    /// `node_id` ascending. Each row is converted to [`NymNodeData`] via the
    /// fallible `TryFrom` impl that decodes stored base58 keys; a failure
    /// anywhere in the page produces [`ApiError::MalformedStoredData`].
    pub(crate) async fn get_nym_nodes_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PagedResult<NymNodeData>, ApiError> {
        let (nym_nodes, total) = match self.storage.get_nym_nodes_paginated(pagination).await {
            Err(err) => {
                error!("get_nym_nodes_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok((nym_nodes, total)) => (nym_nodes, total),
        };

        let mut items = Vec::with_capacity(nym_nodes.len());
        for node in nym_nodes {
            items.push(node.try_into().map_err(|err| {
                error!("get_nym_nodes_paginated malformed stored data: {err}");
                ApiError::MalformedStoredData
            })?);
        }

        Ok(PagedResult {
            page: pagination.page(),
            per_page: items.len(),
            total,
            items,
        })
    }

    /// Backs `GET /v1/results/nym-node/{node_id}/testruns/{kind}`. Returns a
    /// page of the kind's completed runs for a single node ordered newest first.
    /// Unknown or never-tested nodes produce a valid empty page (`total: 0`)
    /// rather than a 404.
    pub(crate) async fn get_testruns_for_node_paginated(
        &self,
        kind: TestKind,
        node_id: NodeId,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunData>, ApiError> {
        let (testruns, total) = match self
            .storage
            .get_testruns_for_node_paginated(kind, node_id, pagination)
            .await
        {
            Err(err) => {
                error!("get_testruns_for_node_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok((testruns, total)) => (testruns, total),
        };

        Ok(PagedResult {
            page: pagination.page(),
            per_page: testruns.len(),
            total,
            items: testruns
                .into_iter()
                .map(testrun_data)
                .collect::<Result<_, _>>()?,
        })
    }

    /// Backs `GET /v1/results/route-sessions/epoch/{mixnet_epoch}`: the epoch's route sessions as
    /// test messages, each with the verdict stored when its run was submitted.
    pub(crate) async fn get_route_sessions_of_epoch(
        &self,
        mixnet_epoch: EpochId,
    ) -> Result<Vec<RouteSessionData>, ApiError> {
        five_hop_mixnet_session_export::five_hop_mixnet_sessions_of_epoch(
            &self.storage,
            mixnet_epoch,
        )
        .await
    }

    /// Backs `GET /v1/aggregates/epoch/{mixnet_epoch}`. Every node's figures for that epoch, one
    /// record per node with a config score or an aggregate. Not paginated: the population is around
    /// a thousand nodes and each record is small.
    pub(crate) async fn get_epoch_aggregates(
        &self,
        mixnet_epoch: EpochId,
    ) -> Result<Vec<NodeEpochAggregates>, ApiError> {
        let aggregates = match self.storage.get_mixnet_epoch_aggregates(mixnet_epoch).await {
            Err(err) => {
                error!("get_mixnet_epoch_aggregates storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(rows) => rows,
        };

        let config_scores = match self
            .storage
            .get_mixnet_epoch_config_scores(mixnet_epoch)
            .await
        {
            Err(err) => {
                error!("get_mixnet_epoch_config_scores storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(rows) => rows,
        };

        let five_hop_liveness = match self
            .storage
            .get_mixnet_epoch_five_hop_liveness(mixnet_epoch)
            .await
        {
            Err(err) => {
                error!("get_mixnet_epoch_five_hop_liveness storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(rows) => rows,
        };

        Ok(epoch_records(
            &config_scores,
            &aggregates,
            &five_hop_liveness,
            mixnet_epoch,
        ))
    }

    /// Backs `GET /v1/aggregates/nym-node/{node_id}/epoch/{mixnet_epoch}`. One node's figures for
    /// that epoch. A node with none returns a record with every entry absent rather than a 404, the
    /// same way a never-tested node returns an empty page of test runs: the per-epoch tables hold no
    /// row for a node nothing was computed for, so they cannot answer node existence anyway.
    pub(crate) async fn get_node_epoch_aggregates(
        &self,
        mixnet_epoch: EpochId,
        node_id: NodeId,
    ) -> Result<NodeEpochAggregates, ApiError> {
        let aggregates = match self
            .storage
            .get_mixnet_epoch_aggregates_for_node(mixnet_epoch, node_id)
            .await
        {
            Err(err) => {
                error!("get_mixnet_epoch_aggregates_for_node storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(rows) => rows,
        };

        let config_score = match self
            .storage
            .get_mixnet_epoch_config_score_for_node(mixnet_epoch, node_id)
            .await
        {
            Err(err) => {
                error!("get_mixnet_epoch_config_score_for_node storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(row) => row,
        };

        let five_hop_liveness = match self
            .storage
            .get_mixnet_epoch_five_hop_liveness_for_node(mixnet_epoch, node_id)
            .await
        {
            Err(err) => {
                error!("get_mixnet_epoch_five_hop_liveness_for_node storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(row) => row,
        };

        let figures = NodeEpochFigures {
            node_id,
            config_score,
            aggregates: ReportedAggregates::from_rows(&aggregates),
        };
        let measured_five_hop_liveness = five_hop_liveness.and_then(|value| value.measured_score());
        Ok(node_record(
            figures,
            measured_five_hop_liveness,
            mixnet_epoch,
        ))
    }
}

/// One node's figures for an epoch, as the wire record, with its measured five-hop liveness.
fn node_record(
    figures: NodeEpochFigures,
    five_hop_liveness: Option<f64>,
    mixnet_epoch: EpochId,
) -> NodeEpochAggregates {
    NodeEpochAggregates::new(
        figures.node_id,
        mixnet_epoch,
        figures.config_score.map(Into::into),
        figures.aggregates.liveness.map(Into::into),
        figures.aggregates.stress.map(Into::into),
        five_hop_liveness,
    )
}

/// One record per node with a config score or an aggregate for the epoch, ordered by node id, each
/// with its five-hop liveness if one was measured for the epoch.
///
/// `mixnet_epoch` is passed in rather than read from a row so the result is well-formed even when
/// neither table has anything for the epoch.
fn epoch_records(
    config_scores: &[MixnetEpochConfigScore],
    aggregates: &[MixnetEpochAggregate],
    five_hop_liveness: &[MixnetEpochFiveHopLiveness],
    mixnet_epoch: EpochId,
) -> Vec<NodeEpochAggregates> {
    let measured_five_hop_liveness: BTreeMap<NodeId, f64> = five_hop_liveness
        .iter()
        // stored from a `NodeId`, so it fits
        .filter_map(|value| Some((value.node_id as NodeId, value.measured_score()?)))
        .collect();

    epoch_figures(config_scores, aggregates)
        .into_iter()
        .map(|figures| {
            let five_hop_liveness = measured_five_hop_liveness.get(&figures.node_id).copied();
            node_record(figures, five_hop_liveness, mixnet_epoch)
        })
        .collect()
}

#[cfg(test)]
mod aggregate_shaping {
    use super::*;
    use crate::storage::models::{carried_five_hop_liveness, measured_five_hop_liveness};
    use nym_network_monitor_orchestrator_requests::models::{ConfigScore, KindAggregate};
    use time::OffsetDateTime;
    use time::macros::datetime;

    const EPOCH_START: OffsetDateTime = datetime!(2025-06-01 12:00:00 UTC);

    fn row(node_id: i64, test_kind: TestKind, score: f64, samples: i64) -> MixnetEpochAggregate {
        MixnetEpochAggregate {
            mixnet_epoch: 7,
            epoch_start: EPOCH_START,
            node_id,
            test_kind,
            score,
            samples,
        }
    }

    fn config_row(node_id: i64, score: f64) -> MixnetEpochConfigScore {
        MixnetEpochConfigScore {
            mixnet_epoch: 7,
            epoch_start: EPOCH_START,
            node_id,
            score,
            versions_behind: Some(0),
            accepted_terms_and_conditions: true,
            runs_nym_node_binary: true,
            has_sufficient_tokens: true,
            is_feegrant_grantee: false,
        }
    }

    // the epoch's rows are grouped by node rather than smeared together, and within a node both
    // liveness kinds become its one liveness figure
    #[test]
    fn each_node_becomes_one_record_built_from_its_own_rows() {
        let rows = vec![
            row(1, TestKind::GatewayLiveness, 0.0, 1),
            row(1, TestKind::MixnodeLiveness, 1.0, 3),
            row(2, TestKind::MixnodeStress, 0.5, 2),
        ];

        assert_eq!(
            epoch_records(&[], &rows, &[], 7),
            vec![
                NodeEpochAggregates::new(
                    1,
                    7,
                    None,
                    Some(KindAggregate {
                        score: 0.75,
                        samples: 4
                    }),
                    None,
                    None
                ),
                NodeEpochAggregates::new(
                    2,
                    7,
                    None,
                    None,
                    Some(KindAggregate {
                        score: 0.5,
                        samples: 2
                    }),
                    None
                ),
            ]
        );
    }

    // either figure alone makes a node a record: a described node measured by nothing this window
    // keeps its config score, a node with aggregates but no config score (the norm for a backfilled
    // epoch) keeps its aggregates, and whatever a node lacks is absent rather than a zero
    #[test]
    fn a_node_with_either_figure_is_a_record_with_the_other_absent() {
        let config_scores = vec![config_row(1, 0.9), config_row(3, 0.4)];
        let aggregates = vec![
            row(2, TestKind::MixnodeStress, 0.5, 2),
            row(3, TestKind::MixnodeStress, 1.0, 1),
        ];

        let records = epoch_records(&config_scores, &aggregates, &[], 7);

        let stress = |score, samples| Some(KindAggregate { score, samples });
        assert_eq!(
            records,
            vec![
                NodeEpochAggregates::new(
                    1,
                    7,
                    Some(ConfigScore::from(config_row(1, 0.9))),
                    None,
                    None,
                    None
                ),
                NodeEpochAggregates::new(2, 7, None, None, stress(0.5, 2), None),
                NodeEpochAggregates::new(
                    3,
                    7,
                    Some(ConfigScore::from(config_row(3, 0.4))),
                    None,
                    stress(1.0, 1),
                    None
                ),
            ]
        );
    }

    // the five-hop liveness entry is the value measured for the epoch: a value carried forward into
    // it is not served, and a node without one has no entry rather than a zero
    #[test]
    fn the_five_hop_liveness_entry_is_the_measured_value_only() {
        let rows = vec![
            row(1, TestKind::MixnodeLiveness, 1.0, 1),
            row(2, TestKind::MixnodeLiveness, 1.0, 1),
            row(3, TestKind::MixnodeLiveness, 1.0, 1),
        ];
        let five_hop_liveness = vec![
            measured_five_hop_liveness(1, 0.9),
            carried_five_hop_liveness(2, 0.8),
        ];

        let served: Vec<Option<f64>> = epoch_records(&[], &rows, &five_hop_liveness, 7)
            .into_iter()
            .map(|record| record.five_hop_liveness)
            .collect();

        assert_eq!(served, vec![Some(0.9), None, None]);
    }
}
