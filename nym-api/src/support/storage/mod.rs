// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{
    GatewayUptimeHistory, HistoricalUptime as ApiHistoricalUptime, MixnodeUptimeHistory,
    NymApiStorageError,
};
use crate::node_status_api::ONE_DAY;
use crate::storage::manager::StorageManager;
use crate::support::storage::models::{
    GatewayDetails, HistoricalUptime, MixnodeDetails, MonitorRunReport, MonitorRunScore,
    TestedGatewayStatus, TestedMixnodeStatus,
};
use nym_mixnet_contract_common::NodeId;
use sqlx::sqlite::{SqliteAutoVacuum, SqliteSynchronous};
use sqlx::ConnectOptions;
use std::path::Path;
use std::time::Duration;
use time::Date;
use tracing::log::LevelFilter;
use tracing::{error, info};

pub(crate) mod manager;
pub(crate) mod models;
pub(crate) mod runtime_migrations;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct NymApiStorage {
    pub manager: StorageManager,
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
        })
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

    /// The lower bound of a core-status count given none: 30 days before the last recorded monitor
    /// run rather than before now, so that the counts stay what they were when the monitor stopped
    /// instead of decaying to zero.
    async fn default_core_status_since(&self) -> Result<i64, NymApiStorageError> {
        let last_run = self.manager.get_latest_monitor_run_timestamp().await?;
        Ok(last_run.unwrap_or_default() - (30 * ONE_DAY).as_secs() as i64)
    }

    /// Retrieves number of times particular mixnode was used as a core node during network monitor
    /// test runs since the specified unix timestamp. If no value is provided, the 30 days up to the
    /// last recorded run are used instead.
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
            let since = match since {
                Some(since) => since,
                None => self.default_core_status_since().await?,
            };

            self.manager
                .get_mixnode_testing_route_presence_count_since(node_id, since)
                .await
                .map_err(|err| err.into())
        } else {
            Ok(0)
        }
    }

    /// Retrieves number of times particular gateway was used as a core node during network monitor
    /// test runs since the specified unix timestamp. If no value is provided, the 30 days up to the
    /// last recorded run are used instead.
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
            let since = match since {
                Some(since) => since,
                None => self.default_core_status_since().await?,
            };

            self.manager
                .get_gateway_testing_route_presence_count_since(node_id, since)
                .await
                .map_err(|err| err.into())
        } else {
            Ok(0)
        }
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

    /// Records one monitor run at `timestamp` in which `mixnode_db_id` was a core node, as the
    /// removed monitor would have.
    async fn record_run_with_core_mixnode(
        storage: &NymApiStorage,
        timestamp: i64,
        mixnode_db_id: i64,
    ) {
        let pool = &storage.manager.connection_pool;
        let run_id = sqlx::query("INSERT INTO monitor_run(timestamp) VALUES (?)")
            .bind(timestamp)
            .execute(pool)
            .await
            .unwrap()
            .last_insert_rowid();
        sqlx::query(
            "INSERT INTO testing_route(gateway_id, layer1_mix_id, layer2_mix_id, layer3_mix_id, monitor_run_id) VALUES (0, ?, 0, 0, ?)",
        )
        .bind(mixnode_db_id)
        .bind(run_id)
        .execute(pool)
        .await
        .unwrap();
    }

    /// The window ends at the last recorded run rather than now, so long after the monitor stopped
    /// the count still answers what it answered when it stopped, and a run more than 30 days
    /// before the last one still falls outside it.
    #[tokio::test]
    async fn the_default_core_status_window_ends_at_the_last_recorded_run() {
        let storage = NymApiStorage::init_in_memory().await.unwrap();
        let mix_id = 42;
        let mixnode_db_id =
            sqlx::query("INSERT INTO mixnode_details(mix_id, identity_key) VALUES (?, 'identity')")
                .bind(mix_id)
                .execute(&storage.manager.connection_pool)
                .await
                .unwrap()
                .last_insert_rowid();

        let last_run = datetime!(2025-01-31 00:00:00 UTC).unix_timestamp();
        let day = ONE_DAY.as_secs() as i64;
        for days_before_last in [40, 10, 0] {
            record_run_with_core_mixnode(
                &storage,
                last_run - days_before_last * day,
                mixnode_db_id,
            )
            .await;
        }

        let count = storage
            .get_core_mixnode_status_count(mix_id, None)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }
}
