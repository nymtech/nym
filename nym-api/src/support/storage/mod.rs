// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::monitor::summary_producer::TestReport;
use crate::network_monitor::test_route::TestRoute;
use crate::node_status_api::models::{
    GatewayStatusReport, GatewayUptimeHistory, HistoricalUptime as ApiHistoricalUptime,
    MixnodeStatusReport, MixnodeUptimeHistory, NymApiStorageError, Uptime,
};
use crate::node_status_api::{ONE_DAY, ONE_HOUR};
use crate::storage::manager::StorageManager;
use crate::storage::models::TestingRoute;
use crate::support::storage::models::{
    GatewayDetails, HistoricalUptime, MixnodeDetails, MonitorRunReport, MonitorRunScore,
    NymNodeLivenessResult, NymNodeStressTestingResult, RetrievedAverageStressTestResult,
    TestedGatewayStatus, TestedMixnodeStatus,
};
use dashmap::DashMap;
use nym_mixnet_contract_common::NodeId;
use nym_types::monitoring::NodeResult;
use sqlx::sqlite::{SqliteAutoVacuum, SqliteSynchronous};
use sqlx::ConnectOptions;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use time::{Date, OffsetDateTime};
use tracing::log::LevelFilter;
use tracing::{error, info, warn};

pub(crate) mod manager;
pub(crate) mod models;
pub(crate) mod runtime_migrations;

#[derive(Default)]
pub(crate) struct DbIdCache {
    pub mixnodes_v1: DashMap<NodeId, i64>,
    pub gateways_v1: DashMap<NodeId, i64>,
}

impl DbIdCache {
    pub(crate) fn mixnode_db_id(&self, node_id: NodeId) -> Option<i64> {
        self.mixnodes_v1.get(&node_id).map(|v| *v)
    }

    pub(crate) fn gateway_db_id(&self, node_id: NodeId) -> Option<i64> {
        self.gateways_v1.get(&node_id).map(|v| *v)
    }

    pub(crate) fn set_mixnode_db_id(&self, node_id: NodeId, db_id: i64) {
        self.mixnodes_v1.insert(node_id, db_id);
    }

    pub(crate) fn set_gateway_db_id(&self, node_id: NodeId, db_id: i64) {
        self.gateways_v1.insert(node_id, db_id);
    }
}

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct NymApiStorage {
    pub manager: StorageManager,

    pub db_id_cache: Arc<DbIdCache>,
}

impl NymApiStorage {
    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, NymApiStorageError> {
        // TODO: we can inject here more stuff based on our nym-api global config
        // struct. Maybe different pool size or timeout intervals?
        let connect_opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .filename(database_path)
            .create_if_missing(true)
            .log_statements(LevelFilter::Trace)
            .log_slow_statements(LevelFilter::Warn, Duration::from_millis(250));

        // TODO: do we want auto_vacuum ?

        let pool_opts = sqlx::sqlite::SqlitePoolOptions::new()
            .min_connections(5)
            .max_connections(25)
            .acquire_timeout(Duration::from_secs(60));

        Self::from_options(connect_opts, pool_opts).await
    }

    /// Build a [`NymApiStorage`] backed by an in-memory SQLite database. The
    /// pool is pinned to a single connection so every query sees the same DB
    /// (the standard "private :memory:" gotcha — multiple connections to
    /// `:memory:` produce independent DBs unless shared-cache is used).
    ///
    /// Intended for tests; migrations run identically to the file-backed
    /// constructor.
    #[cfg(test)]
    pub async fn init_in_memory() -> Result<Self, NymApiStorageError> {
        let connect_opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true);

        let pool_opts = sqlx::sqlite::SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(1);

        Self::from_options(connect_opts, pool_opts).await
    }

    async fn from_options(
        connect_opts: sqlx::sqlite::SqliteConnectOptions,
        pool_opts: sqlx::sqlite::SqlitePoolOptions,
    ) -> Result<Self, NymApiStorageError> {
        let connection_pool = match pool_opts.connect_with(connect_opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to initialize SQLx database: {err}");
            return Err(err.into());
        }

        info!("Database migration finished!");

        Ok(NymApiStorage {
            manager: StorageManager { connection_pool },
            db_id_cache: Arc::new(Default::default()),
        })
    }

    pub(crate) async fn get_mixnode_database_id(
        &self,
        node_id: NodeId,
    ) -> Result<Option<i64>, NymApiStorageError> {
        if let Some(cached) = self.db_id_cache.mixnode_db_id(node_id) {
            return Ok(Some(cached));
        }
        if let Some(retrieved) = self.manager.get_mixnode_database_id(node_id).await? {
            self.db_id_cache.set_mixnode_db_id(node_id, retrieved);
            return Ok(Some(retrieved));
        }
        Ok(None)
    }

    pub(crate) async fn get_gateway_database_id(
        &self,
        node_id: NodeId,
    ) -> Result<Option<i64>, NymApiStorageError> {
        if let Some(cached) = self.db_id_cache.gateway_db_id(node_id) {
            return Ok(Some(cached));
        }
        if let Some(retrieved) = self.manager.get_gateway_database_id(node_id).await? {
            self.db_id_cache.set_gateway_db_id(node_id, retrieved);
            return Ok(Some(retrieved));
        }
        Ok(None)
    }

    pub(crate) async fn get_mixnode_uptime_history(
        &self,
        mix_id: NodeId,
    ) -> Result<MixnodeUptimeHistory, NymApiStorageError> {
        let history = self.manager.get_mixnode_historical_uptimes(mix_id).await?;

        if history.is_empty() {
            return Err(NymApiStorageError::MixnodeUptimeHistoryNotFound { mix_id });
        }

        let Some(mixnode_identity) = self.manager.get_mixnode_identity_key(mix_id).await? else {
            return Err(NymApiStorageError::DatabaseInconsistency { reason: format!("The node {mix_id} doesn't have an identity even though we uptime history for it!") });
        };

        Ok(MixnodeUptimeHistory::new(mix_id, mixnode_identity, history))
    }

    pub(crate) async fn get_gateway_uptime_history_by_identity(
        &self,
        gateway_identity: &str,
    ) -> Result<GatewayUptimeHistory, NymApiStorageError> {
        let Some(node_id) = self
            .manager
            .get_gateway_node_id_from_identity_key(gateway_identity)
            .await?
        else {
            return Err(NymApiStorageError::GatewayNotFound {
                identity: gateway_identity.to_string(),
            });
        };

        let history = self.manager.get_gateway_historical_uptimes(node_id).await?;

        if history.is_empty() {
            return Err(NymApiStorageError::GatewayUptimeHistoryNotFound { node_id });
        }

        Ok(GatewayUptimeHistory::new(
            node_id,
            gateway_identity,
            history,
        ))
    }

    pub(crate) async fn get_node_uptime_history(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<ApiHistoricalUptime>, NymApiStorageError> {
        let history = self.manager.get_mixnode_historical_uptimes(node_id).await?;

        if !history.is_empty() {
            return Ok(history);
        }

        Ok(self.manager.get_gateway_historical_uptimes(node_id).await?)
    }

    pub(crate) async fn get_average_mixnode_reliability_in_the_last_24hrs(
        &self,
        node_id: NodeId,
        end_ts_secs: i64,
    ) -> Result<f32, NymApiStorageError> {
        let start = end_ts_secs - 86400;
        let reliability = self
            .get_average_mixnode_reliability_in_time_interval(node_id, start, end_ts_secs)
            .await?;
        Ok(reliability)
    }

    pub(crate) async fn get_average_gateway_reliability_in_the_last_24hrs(
        &self,
        node_id: NodeId,
        end_ts_secs: i64,
    ) -> Result<f32, NymApiStorageError> {
        let start = end_ts_secs - 86400;
        let reliability = self
            .get_average_gateway_reliability_in_time_interval(node_id, start, end_ts_secs)
            .await?;
        Ok(reliability)
    }

    pub(crate) async fn get_average_node_reliability_in_the_last_24hrs(
        &self,
        node_id: NodeId,
        end_ts_secs: i64,
    ) -> Result<f32, NymApiStorageError> {
        let start = end_ts_secs - 86400;
        let reliability = self
            .get_average_node_reliability_in_time_interval(node_id, start, end_ts_secs)
            .await?;
        Ok(reliability)
    }

    pub(crate) async fn get_average_node_stress_test_score(
        &self,
        node_id: NodeId,
        start_ts: OffsetDateTime,
        end_ts: OffsetDateTime,
    ) -> Result<Option<RetrievedAverageStressTestResult>, NymApiStorageError> {
        Ok(self
            .manager
            .get_average_node_stress_test_score(node_id as i64, start_ts, end_ts)
            .await?)
    }

    #[allow(unused)]
    pub(crate) async fn get_average_mixnode_uptime_in_the_last_24hrs(
        &self,
        node_id: NodeId,
        end_ts_secs: i64,
    ) -> Result<Uptime, NymApiStorageError> {
        self.get_average_mixnode_reliability_in_the_last_24hrs(node_id, end_ts_secs)
            .await
            .map(Uptime::new)
    }

    #[allow(unused)]
    pub(crate) async fn get_average_gateway_uptime_in_the_last_24hrs(
        &self,
        node_id: NodeId,
        end_ts_secs: i64,
    ) -> Result<Uptime, NymApiStorageError> {
        self.get_average_gateway_reliability_in_the_last_24hrs(node_id, end_ts_secs)
            .await
            .map(Uptime::new)
    }

    #[allow(unused)]
    pub(crate) async fn get_average_node_uptime_in_the_last_24hrs(
        &self,
        node_id: NodeId,
        end_ts_secs: i64,
    ) -> Result<Uptime, NymApiStorageError> {
        self.get_average_node_reliability_in_the_last_24hrs(node_id, end_ts_secs)
            .await
            .map(Uptime::new)
    }

    pub(crate) async fn get_historical_mix_uptime_on(
        &self,
        node_id: NodeId,
        date: Date,
    ) -> Result<Option<HistoricalUptime>, NymApiStorageError> {
        Ok(self
            .manager
            .get_historical_mix_uptime_on(node_id as i64, date)
            .await?)
    }

    pub(crate) async fn get_historical_gateway_uptime_on(
        &self,
        node_id: NodeId,
        date: Date,
    ) -> Result<Option<HistoricalUptime>, NymApiStorageError> {
        Ok(self
            .manager
            .get_historical_gateway_uptime_on(node_id as i64, date)
            .await?)
    }

    pub(crate) async fn get_historical_node_uptime_on(
        &self,
        node_id: NodeId,
        date: Date,
    ) -> Result<Option<HistoricalUptime>, NymApiStorageError> {
        if let Ok(result_as_mix) = self.get_historical_mix_uptime_on(node_id, date).await {
            if result_as_mix.is_some() {
                return Ok(result_as_mix);
            }
        }

        self.get_historical_gateway_uptime_on(node_id, date).await
    }

    /// Based on the data available in the validator API, determines the average uptime of particular
    /// mixnode during the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_average_mixnode_reliability_in_time_interval(
        &self,
        mix_id: NodeId,
        start: i64,
        end: i64,
    ) -> Result<f32, NymApiStorageError> {
        // those two should have been a single sql query /shrug
        let mixnode_database_id = match self.manager.get_mixnode_database_id(mix_id).await? {
            Some(id) => id,
            None => return Ok(0.),
        };

        let reliability = self
            .manager
            .get_mixnode_average_reliability_in_interval(mixnode_database_id, start, end)
            .await?;

        Ok(reliability.unwrap_or_default())
    }

    /// Based on the data available in the validator API, determines the average uptime of particular
    /// gateway during the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `identity`: base58-encoded identity of the gateway.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_average_gateway_reliability_in_time_interval(
        &self,
        node_id: NodeId,
        start: i64,
        end: i64,
    ) -> Result<f32, NymApiStorageError> {
        // those two should have been a single sql query /shrug
        let gateway_database_id = match self.manager.get_gateway_database_id(node_id).await? {
            Some(id) => id,
            None => return Ok(0.),
        };

        let reliability = self
            .manager
            .get_gateway_average_reliability_in_interval(gateway_database_id, start, end)
            .await?;

        Ok(reliability.unwrap_or_default())
    }

    pub(crate) async fn get_average_node_reliability_in_time_interval(
        &self,
        node_id: NodeId,
        start: i64,
        end: i64,
    ) -> Result<f32, NymApiStorageError> {
        let result_as_mix = self
            .get_average_mixnode_reliability_in_time_interval(node_id, start, end)
            .await?;

        let result_as_gateway = self
            .get_average_gateway_reliability_in_time_interval(node_id, start, end)
            .await?;

        // give the benefit of the doubt if one of the scores is 0
        if result_as_mix == 0. {
            return Ok(result_as_gateway);
        }
        if result_as_gateway == 0. {
            return Ok(result_as_mix);
        }
        Ok((result_as_mix + result_as_gateway) / 2.)
    }

    /// Obtain status reports of mixnodes that were active in the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    // NOTE: even though the arguments would suggest this function is generic in regards to
    // interval length, the constructed reports still assume the intervals are 24h in length.
    pub(crate) async fn get_all_active_mixnode_reports_in_interval(
        &self,
        start: i64,
        end: i64,
    ) -> Result<Vec<MixnodeStatusReport>, NymApiStorageError> {
        let Ok(end_timestamp) = OffsetDateTime::from_unix_timestamp(end) else {
            return Err(NymApiStorageError::InvalidTimestampProvided { value: end });
        };

        if (end - start) as u64 != ONE_DAY.as_secs() {
            warn!("Our current interval length breaks the 24h length assumption")
        }

        let hour_ago = end - ONE_HOUR.as_secs() as i64;

        // determine the number of runs the mixnodes should have been online for
        let last_hour_runs_count = self.get_monitor_runs_count(hour_ago, end).await?;
        let last_day_runs_count = self.get_monitor_runs_count(start, end).await?;

        let reports = self
            .manager
            .get_all_active_mixnodes_statuses_in_interval(start, end)
            .await?
            .into_iter()
            .map(|statuses| {
                MixnodeStatusReport::construct_from_last_day_reports(
                    end_timestamp,
                    statuses.mix_id,
                    statuses.identity,
                    statuses.statuses,
                    last_hour_runs_count,
                    last_day_runs_count,
                )
            })
            .collect();

        Ok(reports)
    }

    /// Obtain status reports of gateways that were active in the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    // NOTE: even though the arguments would suggest this function is generic in regards to
    // interval length, the constructed reports still assume the intervals are 24h in length.
    pub(crate) async fn get_all_active_gateway_reports_in_interval(
        &self,
        start: i64,
        end: i64,
    ) -> Result<Vec<GatewayStatusReport>, NymApiStorageError> {
        let Ok(end_timestamp) = OffsetDateTime::from_unix_timestamp(end) else {
            return Err(NymApiStorageError::InvalidTimestampProvided { value: end });
        };

        if (end - start) as u64 != ONE_DAY.as_secs() {
            warn!("Our current interval length breaks the 24h length assumption")
        }

        let hour_ago = end - ONE_HOUR.as_secs() as i64;

        // determine the number of runs the mixnodes should have been online for
        let last_hour_runs_count = self.get_monitor_runs_count(hour_ago, end).await?;
        let last_day_runs_count = self.get_monitor_runs_count(start, end).await?;

        let reports = self
            .manager
            .get_all_active_gateways_statuses_in_interval(start, end)
            .await?
            .into_iter()
            .map(|statuses| {
                GatewayStatusReport::construct_from_last_day_reports(
                    end_timestamp,
                    statuses.node_id,
                    statuses.identity,
                    statuses.statuses,
                    last_hour_runs_count,
                    last_day_runs_count,
                )
            })
            .collect();

        Ok(reports)
    }

    /// Saves information about test route used during the network monitor run to the database.
    ///
    /// # Arguments
    ///
    /// * `monitor_run_id` id (as saved in the database) of the associated network monitor test run.
    /// * `test_route`: one of the test routes used during network testing.
    async fn insert_test_route(
        &self,
        monitor_run_db_id: i64,
        test_route: TestRoute,
    ) -> Result<(), NymApiStorageError> {
        // we MUST have those entries in the database, otherwise the route wouldn't have been chosen
        // in the first place
        let layer1_mix_db_id = self
            .get_mixnode_database_id(test_route.layer_one_mix().node_id)
            .await?
            .ok_or_else(|| NymApiStorageError::DatabaseInconsistency {
                reason: format!("could not get db id for layer1 mixnode from network monitor run {monitor_run_db_id}"),
            })?;

        let layer2_mix_db_id = self
            .get_mixnode_database_id(test_route.layer_two_mix().node_id)
            .await?
            .ok_or_else(|| NymApiStorageError::DatabaseInconsistency {
                reason: format!("could not get db id for layer2 mixnode from network monitor run {monitor_run_db_id}"),
            })?;

        let layer3_mix_db_id = self
            .get_mixnode_database_id(test_route.layer_three_mix().node_id)
            .await?
            .ok_or_else(|| NymApiStorageError::DatabaseInconsistency {
                reason: format!("could not get db id for layer3 mixnode from network monitor run {monitor_run_db_id}"),
            })?;

        let gateway_db_id = self
            .get_gateway_database_id(test_route.gateway().node_id)
            .await?
            .ok_or_else(|| NymApiStorageError::DatabaseInconsistency {
                reason: format!(
                    "could not get db id for gateway from network monitor run {monitor_run_db_id}"
                ),
            })?;

        self.manager
            .submit_testing_route_used(TestingRoute {
                gateway_db_id,
                layer1_mix_db_id,
                layer2_mix_db_id,
                layer3_mix_db_id,
                monitor_run_db_id,
            })
            .await?;
        Ok(())
    }

    /// Retrieves number of times particular mixnode was used as a core node during network monitor
    /// test runs since the specified unix timestamp. If no value is provided, last 30 days of data
    /// are used instead.
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    /// * `since`: optional unix timestamp indicating the lower bound interval of the selection.
    pub(crate) async fn get_core_mixnode_status_count(
        &self,
        mix_id: NodeId,
        since: Option<i64>,
    ) -> Result<i64, NymApiStorageError> {
        let db_id = self.manager.get_mixnode_database_id(mix_id).await?;

        if let Some(node_id) = db_id {
            let since = since
                .unwrap_or_else(|| (OffsetDateTime::now_utc() - (30 * ONE_DAY)).unix_timestamp());

            self.manager
                .get_mixnode_testing_route_presence_count_since(node_id, since)
                .await
                .map_err(|err| err.into())
        } else {
            Ok(0)
        }
    }

    /// Retrieves number of times particular gateway was used as a core node during network monitor
    /// test runs since the specified unix timestamp. If no value is provided, last 30 days of data
    /// are used instead.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the gateway.
    /// * `since`: optional unix timestamp indicating the lower bound interval of the selection.
    pub(crate) async fn get_core_gateway_status_count_by_identity(
        &self,
        identity: &str,
        since: Option<i64>,
    ) -> Result<i64, NymApiStorageError> {
        let node_id = self
            .manager
            .get_gateway_database_id_by_identity(identity)
            .await?;

        if let Some(node_id) = node_id {
            let since = since
                .unwrap_or_else(|| (OffsetDateTime::now_utc() - (30 * ONE_DAY)).unix_timestamp());

            self.manager
                .get_gateway_testing_route_presence_count_since(node_id, since)
                .await
                .map_err(|err| err.into())
        } else {
            Ok(0)
        }
    }

    /// Inserts an entry to the database with the network monitor test run information
    /// that has occurred at this instant alongside the results of all the measurements performed.
    ///
    /// # Arguments
    ///
    /// * `mixnode_results`:
    /// * `gateway_results`:
    /// * `route_results`:
    pub(crate) async fn insert_monitor_run_results(
        &self,
        mixnode_results: Vec<NodeResult>,
        gateway_results: Vec<NodeResult>,
        test_routes: Vec<TestRoute>,
    ) -> Result<i64, NymApiStorageError> {
        info!("Submitting new node results to the database. There are {} mixnode results and {} gateway results", mixnode_results.len(), gateway_results.len());

        let now = OffsetDateTime::now_utc().unix_timestamp();

        let monitor_run_id = self.manager.insert_monitor_run(now).await?;

        self.manager
            .submit_mixnode_statuses(now, mixnode_results, &self.db_id_cache)
            .await?;

        self.manager
            .submit_gateway_statuses(now, gateway_results, &self.db_id_cache)
            .await?;

        for test_route in test_routes {
            self.insert_test_route(monitor_run_id, test_route).await?;
        }

        Ok(monitor_run_id)
    }

    pub(crate) async fn insert_monitor_run_report(
        &self,
        report: TestReport,
        monitor_run_id: i64,
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_monitor_run_report(
                monitor_run_id,
                report.network_reliability,
                report.total_sent as u32,
                report.total_received as u32,
            )
            .await?;

        let mut scores = Vec::new();
        for (score, count) in report.mixnode_results {
            scores.push(MonitorRunScore {
                typ: "mixnode".to_string(),
                monitor_run_id,
                rounded_score: score,
                nodes_count: count as u32,
            })
        }
        for (score, count) in report.gateway_results {
            scores.push(MonitorRunScore {
                typ: "gateway".to_string(),
                monitor_run_id,
                rounded_score: score,
                nodes_count: count as u32,
            })
        }

        self.manager.insert_monitor_run_scores(scores).await?;

        Ok(())
    }

    pub(crate) async fn get_monitor_run_report(
        &self,
        monitor_run_id: i64,
    ) -> Result<Option<(MonitorRunReport, Vec<MonitorRunScore>)>, NymApiStorageError> {
        let Some(report) = self.manager.get_monitor_run_report(monitor_run_id).await? else {
            return Ok(None);
        };
        let scores = self.manager.get_monitor_run_scores(monitor_run_id).await?;
        Ok(Some((report, scores)))
    }

    pub(crate) async fn get_latest_monitor_run_id(
        &self,
    ) -> Result<Option<i64>, NymApiStorageError> {
        Ok(self.manager.get_latest_monitor_run_id().await?)
    }

    pub(crate) async fn submit_mixnode_statuses_v2(
        &self,
        mixnode_results: &[NodeResult],
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .submit_mixnode_statuses_v2(mixnode_results)
            .await?;
        Ok(())
    }

    pub(crate) async fn submit_gateway_statuses_v2(
        &self,
        gateway_results: &[NodeResult],
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .submit_gateway_statuses_v2(gateway_results)
            .await?;
        Ok(())
    }

    /// Persist the given stress-testing results, produced by an authorised network monitor
    /// orchestrator, into the database. Returns the number of rows actually inserted, i.e. excluding
    /// any that deduplicated against a measurement already stored.
    pub(crate) async fn insert_nym_node_stress_testing_results(
        &self,
        results: Vec<NymNodeStressTestingResult>,
    ) -> Result<u64, NymApiStorageError> {
        Ok(self
            .manager
            .insert_nym_node_stress_testing_results(results)
            .await?)
    }

    /// Persist the given liveness results, produced by an authorised network monitor orchestrator,
    /// into the database. Returns the number of rows actually inserted, i.e. excluding any that
    /// deduplicated against a measurement already stored.
    pub(crate) async fn insert_nym_node_liveness_results(
        &self,
        results: Vec<NymNodeLivenessResult>,
    ) -> Result<u64, NymApiStorageError> {
        Ok(self
            .manager
            .insert_nym_node_liveness_results(results)
            .await?)
    }

    /// Obtains number of network monitor test runs that have occurred within the specified interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_monitor_runs_count(
        &self,
        since: i64,
        until: i64,
    ) -> Result<usize, NymApiStorageError> {
        let run_count = self.manager.get_monitor_runs_count(since, until).await?;

        if run_count < 0 {
            // I don't think it's ever possible for SQL to return a negative value from COUNT?
            return Err(NymApiStorageError::DatabaseInconsistency {
                reason: "Negative run count".to_string(),
            });
        }
        Ok(run_count as usize)
    }

    /// Given lists of reports of all monitor-active mixnodes and gateways, inserts the data into the
    /// historical uptime tables. This method is called at a 24h timer.
    ///
    /// # Arguments
    ///
    /// * `today_iso_8601`: today's date expressed in ISO 8601, i.e. YYYY-MM-DD
    /// * `mixnode_reports`: slice of reports for all monitor-active mixnodes
    /// * `gateway_reports`: slice of reports for all monitor-active gateways
    pub(crate) async fn update_historical_uptimes(
        &self,
        today_iso_8601: &str,
        mixnode_reports: &[MixnodeStatusReport],
        gateway_reports: &[GatewayStatusReport],
    ) -> Result<(), NymApiStorageError> {
        for report in mixnode_reports {
            // if this ever fails, we have a super weird error because we just constructed report for that node
            // and we never delete node data!
            let node_id = match self.manager.get_mixnode_database_id(report.mix_id).await? {
                Some(node_id) => node_id,
                None => {
                    error!(
                        "Somehow we failed to grab id of mixnode {} ({}) from the database!",
                        report.mix_id, report.identity
                    );
                    continue;
                }
            };

            self.manager
                .insert_mixnode_historical_uptime(node_id, today_iso_8601, report.last_day.u8())
                .await?;
        }

        for report in gateway_reports {
            // if this ever fails, we have a super weird error because we just constructed report for that node
            // and we never delete node data!
            let db_id = match self.manager.get_gateway_database_id(report.node_id).await? {
                Some(db_id) => db_id,
                None => {
                    error!(
                        "Somehow we failed to grab id of gateway {} from the database!",
                        &report.identity
                    );
                    continue;
                }
            };

            self.manager
                .insert_gateway_historical_uptime(db_id, today_iso_8601, report.last_day.u8())
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn check_if_historical_uptimes_exist_for_date(
        &self,
        date_iso_8601: &str,
    ) -> Result<bool, NymApiStorageError> {
        self.manager
            .check_for_historical_uptime_existence(date_iso_8601)
            .await
            .map_err(|err| err.into())
    }

    /// Removes all ipv4 and ipv6 statuses for all mixnodes and gateways that are older than the
    /// provided timestamp. This method is called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(crate) async fn purge_old_statuses(&self, until: i64) -> Result<(), NymApiStorageError> {
        self.manager.purge_old_mixnode_statuses(until).await?;
        self.manager
            .purge_old_gateway_statuses(until)
            .await
            .map_err(|err| err.into())
    }

    pub(crate) async fn get_mixnode_details_by_db_id(
        &self,
        id: i64,
    ) -> Result<Option<MixnodeDetails>, NymApiStorageError> {
        Ok(self.manager.get_mixnode_details_by_db_id(id).await?)
    }

    pub(crate) async fn get_gateway_details_by_db_id(
        &self,
        id: i64,
    ) -> Result<Option<GatewayDetails>, NymApiStorageError> {
        Ok(self.manager.get_gateway_details_by_db_id(id).await?)
    }

    pub(crate) async fn get_mixnode_detailed_statuses_count(
        &self,
        db_id: i64,
    ) -> Result<usize, NymApiStorageError> {
        Ok(self
            .manager
            .get_mixnode_statuses_count(db_id)
            .await?
            .try_into()
            .unwrap_or(usize::MAX))
    }

    pub(crate) async fn get_mixnode_detailed_statuses(
        &self,
        mix_id: NodeId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<TestedMixnodeStatus>, NymApiStorageError> {
        Ok(self
            .manager
            .get_mixnode_statuses(mix_id, limit, offset)
            .await?)
    }

    pub(crate) async fn get_gateway_detailed_statuses_count(
        &self,
        db_id: i64,
    ) -> Result<usize, NymApiStorageError> {
        Ok(self
            .manager
            .get_gateway_statuses_count(db_id)
            .await?
            .try_into()
            .unwrap_or(usize::MAX))
    }

    pub(crate) async fn get_gateway_detailed_statuses(
        &self,
        gateway_identity: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<TestedGatewayStatus>, NymApiStorageError> {
        Ok(self
            .manager
            .get_gateway_statuses(gateway_identity, limit, offset)
            .await?)
    }
}

pub(crate) mod v3_migration {
    use crate::node_status_api::models::NymApiStorageError;
    use crate::support::storage::models::GatewayDetailsBeforeMigration;
    use crate::support::storage::NymApiStorage;
    use nym_mixnet_contract_common::NodeId;

    impl NymApiStorage {
        pub(crate) async fn check_v3_migration(&self) -> Result<bool, NymApiStorageError> {
            Ok(self.manager.check_v3_migration().await?)
        }

        pub(crate) async fn set_v3_migration_completion(&self) -> Result<(), NymApiStorageError> {
            Ok(self.manager.set_v3_migration_completion().await?)
        }

        pub(crate) async fn get_all_known_gateways(
            &self,
        ) -> Result<Vec<GatewayDetailsBeforeMigration>, NymApiStorageError> {
            Ok(self.manager.get_all_known_gateways().await?)
        }

        pub(crate) async fn set_gateway_node_id(
            &self,
            identity: &str,
            node_id: NodeId,
        ) -> Result<(), NymApiStorageError> {
            Ok(self.manager.set_gateway_node_id(identity, node_id).await?)
        }

        pub(crate) async fn purge_gateway(&self, db_id: i64) -> Result<(), NymApiStorageError> {
            Ok(self.manager.purge_gateway(db_id).await?)
        }

        pub(crate) async fn make_node_id_not_null(&self) -> Result<(), NymApiStorageError> {
            Ok(self.manager.make_node_id_not_null().await?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    /// A liveness row, varying only what a test needs to vary.
    fn liveness_row(
        submitter: &str,
        node_id: NodeId,
        test_timestamp: OffsetDateTime,
        testrun_id: i64,
    ) -> NymNodeLivenessResult {
        NymNodeLivenessResult {
            testrun_id,
            submitter_pubkey: submitter.to_string(),
            node_id,
            result: 0.5,
            was_reachable: true,
            test_timestamp,
        }
    }

    /// Exercises the `20260903120000_liveness_testing` migration as well as the insert, since
    /// `init_in_memory` applies the migrations and the insert is built through `QueryBuilder` and
    /// so is not checked against the schema at compile time.
    ///
    /// One decoy per component of `(node_id, test_timestamp, submitter_pubkey)`, because a
    /// constraint missing any one of them would still let the plain duplicate be rejected.
    #[tokio::test]
    async fn a_liveness_measurement_dedupes_on_its_own_identity() {
        let storage = NymApiStorage::init_in_memory().await.unwrap();
        let at = datetime!(2026-09-03 12:00:00 UTC);

        let stored = storage
            .insert_nym_node_liveness_results(vec![liveness_row("orchestrator-a", 42, at, 1)])
            .await
            .unwrap();
        assert_eq!(stored, 1);

        // the same measurement again: the orchestrator's at-least-once retry path
        let stored = storage
            .insert_nym_node_liveness_results(vec![liveness_row("orchestrator-a", 42, at, 1)])
            .await
            .unwrap();
        assert_eq!(stored, 0, "a resent measurement must not duplicate");

        // decoy per key component: each differs from the stored row in exactly one of them
        let stored = storage
            .insert_nym_node_liveness_results(vec![
                liveness_row("orchestrator-b", 42, at, 1),
                liveness_row("orchestrator-a", 7, at, 1),
                liveness_row("orchestrator-a", 42, datetime!(2026-09-03 12:00:01 UTC), 1),
            ])
            .await
            .unwrap();
        assert_eq!(
            stored, 3,
            "differing in any one of submitter, node or timestamp is a distinct measurement"
        );
    }

    /// The regression guard for the amended dedupe key. A wiped orchestrator database restarts its
    /// testrun counter, so ids get reused for measurements that already landed here. If
    /// `testrun_id` were part of the key, the reused id would be a NEW row and the same
    /// measurement would be stored twice; if it wrongly still keyed the row on its own, fresh
    /// measurements would instead be silently swallowed. Neither may happen.
    #[tokio::test]
    async fn a_reused_testrun_id_neither_duplicates_nor_swallows_a_measurement() {
        let storage = NymApiStorage::init_in_memory().await.unwrap();
        let at = datetime!(2026-09-03 12:00:00 UTC);

        storage
            .insert_nym_node_liveness_results(vec![liveness_row("orchestrator-a", 42, at, 1000)])
            .await
            .unwrap();

        // same measurement resubmitted under a restarted counter: still one measurement
        let stored = storage
            .insert_nym_node_liveness_results(vec![liveness_row("orchestrator-a", 42, at, 1)])
            .await
            .unwrap();
        assert_eq!(
            stored, 0,
            "testrun_id is not part of the identity, so this is the same measurement"
        );

        // a genuinely new measurement that happens to reuse an already-seen id must still land
        let stored = storage
            .insert_nym_node_liveness_results(vec![liveness_row(
                "orchestrator-a",
                42,
                datetime!(2026-09-03 13:00:00 UTC),
                1000,
            )])
            .await
            .unwrap();
        assert_eq!(
            stored, 1,
            "a reused testrun id must not block a new measurement"
        );
    }

    /// An empty batch is reachable: every entry can be dropped by the handler's range check.
    #[tokio::test]
    async fn an_empty_liveness_batch_is_a_no_op() {
        let storage = NymApiStorage::init_in_memory().await.unwrap();

        let stored = storage
            .insert_nym_node_liveness_results(Vec::new())
            .await
            .unwrap();
        assert_eq!(stored, 0);
    }
}
