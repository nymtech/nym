// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{HistoricalUptime as ApiHistoricalUptime, Uptime};
use crate::support::storage::models::{
    GatewayDetails, HistoricalUptime, MixnodeDetails, MonitorRunReport, MonitorRunScore,
    TestedGatewayStatus, TestedMixnodeStatus,
};
use nym_mixnet_contract_common::{IdentityKey, NodeId};
use time::Date;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

// all SQL goes here
impl StorageManager {
    /// Tries to obtain row id of given mixnode given its identity.
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    pub(super) async fn get_mixnode_database_id(
        &self,
        mix_id: NodeId,
    ) -> Result<Option<i64>, sqlx::Error> {
        let id = sqlx::query!("SELECT id FROM mixnode_details WHERE mix_id = ?", mix_id)
            .fetch_optional(&self.connection_pool)
            .await?
            .map(|row| row.id);

        Ok(id)
    }

    /// Tries to obtain row id of given gateway given its identity
    pub(super) async fn get_gateway_database_id_by_identity(
        &self,
        identity: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        let id = sqlx::query!(
            "SELECT id FROM gateway_details WHERE identity = ?",
            identity
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|row| row.id);

        Ok(id)
    }

    pub(super) async fn get_gateway_node_id_from_identity_key(
        &self,
        identity: &str,
    ) -> Result<Option<NodeId>, sqlx::Error> {
        let node_id = sqlx::query!(
            r#"SELECT node_id as "node_id: NodeId" FROM gateway_details WHERE identity = ?"#,
            identity
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|row| row.node_id);

        Ok(node_id)
    }

    /// Tries to obtain identity value of given mixnode given its mix_id
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    pub(super) async fn get_mixnode_identity_key(
        &self,
        mix_id: NodeId,
    ) -> Result<Option<IdentityKey>, sqlx::Error> {
        let identity_key = sqlx::query!(
            "SELECT identity_key FROM mixnode_details WHERE mix_id = ?",
            mix_id
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|row| row.identity_key);

        Ok(identity_key)
    }

    /// Gets the historical daily uptime associated with the particular mixnode
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    pub(super) async fn get_mixnode_historical_uptimes(
        &self,
        mix_id: NodeId,
    ) -> Result<Vec<ApiHistoricalUptime>, sqlx::Error> {
        let uptimes = sqlx::query!(
            r#"
                SELECT date, uptime
                FROM mixnode_historical_uptime
                JOIN mixnode_details
                ON mixnode_historical_uptime.mixnode_details_id = mixnode_details.id
                WHERE mixnode_details.mix_id = ?
                ORDER BY date ASC
            "#,
            mix_id
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        // filter out nodes with valid uptime (in theory all should be 100% valid since we insert them ourselves, but
        // better safe than sorry and not use an unwrap)
        .filter_map(|row| {
            Uptime::try_from(row.uptime.unwrap_or_default())
                .map(|uptime| ApiHistoricalUptime {
                    date: row.date.unwrap_or_default(),
                    uptime,
                })
                .ok()
        })
        .collect();

        Ok(uptimes)
    }

    /// Gets the historical daily uptime associated with the particular gateway
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the gateway.
    pub(super) async fn get_gateway_historical_uptimes(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<ApiHistoricalUptime>, sqlx::Error> {
        let uptimes = sqlx::query!(
            r#"
                SELECT date, uptime
                FROM gateway_historical_uptime
                JOIN gateway_details
                ON gateway_historical_uptime.gateway_details_id = gateway_details.id
                WHERE gateway_details.node_id = ?
                ORDER BY date ASC
            "#,
            node_id
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        // filter out nodes with valid uptime (in theory all should be 100% valid since we insert them ourselves, but
        // better safe than sorry and not use an unwrap)
        .filter_map(|row| {
            Uptime::try_from(row.uptime.unwrap_or_default())
                .map(|uptime| ApiHistoricalUptime {
                    date: row.date.unwrap_or_default(),
                    uptime,
                })
                .ok()
        })
        .collect();

        Ok(uptimes)
    }

    pub(super) async fn get_historical_mix_uptime_on(
        &self,
        contract_node_id: i64,
        date: Date,
    ) -> Result<Option<HistoricalUptime>, sqlx::Error> {
        sqlx::query_as!(
            HistoricalUptime,
            r#"
                SELECT date as "date!: Date", uptime as "uptime!"
                FROM mixnode_historical_uptime
                JOIN mixnode_details
                ON mixnode_historical_uptime.mixnode_details_id = mixnode_details.id
                WHERE
                mixnode_details.mix_id = ?
                AND
                mixnode_historical_uptime.date = ?
            "#,
            contract_node_id,
            date
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    pub(super) async fn get_historical_gateway_uptime_on(
        &self,
        contract_node_id: i64,
        date: Date,
    ) -> Result<Option<HistoricalUptime>, sqlx::Error> {
        sqlx::query_as!(
            HistoricalUptime,
            r#"
                SELECT date as "date!: Date", uptime as "uptime!"
                FROM gateway_historical_uptime
                JOIN gateway_details
                ON gateway_historical_uptime.gateway_details_id = gateway_details.id
                WHERE
                gateway_details.node_id = ?
                AND
                gateway_historical_uptime.date = ?
            "#,
            contract_node_id,
            date
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Get the number of times mixnode with the particular id is present in any `testing_route`
    /// since the provided unix timestamp.
    ///
    /// # Arguments
    ///
    /// * `db_mixnode_id`: id (as saved in the database) of the mixnode.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    pub(super) async fn get_mixnode_testing_route_presence_count_since(
        &self,
        db_mixnode_id: i64,
        since: i64,
    ) -> Result<i64, sqlx::Error> {
        let count = sqlx::query!(
            r#"
                SELECT COUNT(*) as count FROM
                (
                    SELECT monitor_run_id
                    FROM testing_route
                    WHERE testing_route.layer1_mix_id = ? OR testing_route.layer2_mix_id = ? OR testing_route.layer3_mix_id = ?
                ) testing_route
                JOIN
                (
                    SELECT id
                    FROM monitor_run
                    WHERE monitor_run.timestamp > ?
                ) monitor_run
                ON monitor_run.id = testing_route.monitor_run_id;
            "#,
            db_mixnode_id,
            db_mixnode_id,
            db_mixnode_id,
            since,
        ).fetch_one(&self.connection_pool)
            .await?
            .count;

        Ok(count)
    }

    /// Get the number of times gateway with the particular id is present in any `testing_route`
    /// since the provided unix timestamp.
    ///
    /// # Arguments
    ///
    /// * `gateway_id`: id (as saved in the database) of the gateway.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    pub(super) async fn get_gateway_testing_route_presence_count_since(
        &self,
        gateway_id: i64,
        since: i64,
    ) -> Result<i64, sqlx::Error> {
        let count = sqlx::query!(
            r#"
                SELECT COUNT(*) as count FROM
                (
                    SELECT monitor_run_id
                    FROM testing_route
                    WHERE testing_route.gateway_id = ?
                ) testing_route
                JOIN
                (
                    SELECT id
                    FROM monitor_run
                    WHERE monitor_run.timestamp > ?
                ) monitor_run
                ON monitor_run.id = testing_route.monitor_run_id;
            "#,
            gateway_id,
            since,
        )
        .fetch_one(&self.connection_pool)
        .await?
        .count;

        Ok(count)
    }

    pub(super) async fn get_monitor_run_report(
        &self,
        monitor_run_id: i64,
    ) -> Result<Option<MonitorRunReport>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM monitor_run_report WHERE monitor_run_id = ?")
            .bind(monitor_run_id)
            .fetch_optional(&self.connection_pool)
            .await
    }

    pub(super) async fn get_latest_monitor_run_id(&self) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query!("SELECT id from monitor_run ORDER BY id DESC limit 1")
            .fetch_optional(&self.connection_pool)
            .await
            .map(|r| r.map(|r| r.id))
    }

    /// The unix timestamp of the most recent monitor run, if any was ever recorded.
    pub(super) async fn get_latest_monitor_run_timestamp(
        &self,
    ) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query!("SELECT timestamp from monitor_run ORDER BY id DESC limit 1")
            .fetch_optional(&self.connection_pool)
            .await
            .map(|r| r.map(|r| r.timestamp))
    }

    pub(super) async fn get_monitor_run_scores(
        &self,
        monitor_run_id: i64,
    ) -> Result<Vec<MonitorRunScore>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM monitor_run_score WHERE monitor_run_id = ?")
            .bind(monitor_run_id)
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(super) async fn get_mixnode_details_by_db_id(
        &self,
        id: i64,
    ) -> Result<Option<MixnodeDetails>, sqlx::Error> {
        sqlx::query_as!(
            MixnodeDetails,
            "SELECT * FROM mixnode_details WHERE id = ?",
            id
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    pub(super) async fn get_gateway_details_by_db_id(
        &self,
        id: i64,
    ) -> Result<Option<GatewayDetails>, sqlx::Error> {
        // we can't use `query_as!` macro because we don't apply all required table changes during sqlx migrations.
        // some (like v3 directory) happens at runtime
        sqlx::query_as("SELECT * FROM gateway_details WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.connection_pool)
            .await
    }

    pub(super) async fn get_mixnode_statuses_count(&self, db_id: i64) -> Result<i64, sqlx::Error> {
        sqlx::query!(
            r#"
                SELECT COUNT(*) as count
                FROM mixnode_status
                    JOIN monitor_run ON mixnode_status.timestamp = monitor_run.timestamp
                    JOIN testing_route ON monitor_run.id = testing_route.monitor_run_id
                WHERE mixnode_details_id = ?
            "#,
            db_id
        )
        .fetch_one(&self.connection_pool)
        .await
        .map(|record| record.count)
    }

    pub(super) async fn get_mixnode_statuses(
        &self,
        mix_id: NodeId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<TestedMixnodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            TestedMixnodeStatus,
            r#"
                SELECT
                    mixnode_details.id as "db_id",
                    mix_id as "mix_id!",
                    identity_key,
                    reliability as "reliability: u8",
                    monitor_run.timestamp as "timestamp!",
                    gateway_id as "gateway_id!",
                    layer1_mix_id as "layer1_mix_id!",
                    layer2_mix_id as "layer2_mix_id!",
                    layer3_mix_id as "layer3_mix_id!",
                    monitor_run_id as "monitor_run_id!"
                FROM mixnode_status
                    JOIN mixnode_details ON mixnode_status.mixnode_details_id = mixnode_details.id
                    JOIN monitor_run ON mixnode_status.timestamp = monitor_run.timestamp
                    JOIN testing_route ON monitor_run.id = testing_route.monitor_run_id
                WHERE mix_id = ?
                ORDER BY mixnode_status.timestamp DESC
                LIMIT ? OFFSET ?
            "#,
            mix_id,
            limit,
            offset
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    pub(super) async fn get_gateway_statuses_count(&self, db_id: i64) -> Result<i64, sqlx::Error> {
        sqlx::query!(
            r#"
                SELECT COUNT(*) as count
                FROM gateway_status
                    JOIN monitor_run ON gateway_status.timestamp = monitor_run.timestamp
                    JOIN testing_route ON monitor_run.id = testing_route.monitor_run_id
                WHERE gateway_details_id = ?
            "#,
            db_id
        )
        .fetch_one(&self.connection_pool)
        .await
        .map(|record| record.count)
    }

    pub(super) async fn get_gateway_statuses(
        &self,
        gateway_identity: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<TestedGatewayStatus>, sqlx::Error> {
        sqlx::query_as!(
            TestedGatewayStatus,
            r#"
                SELECT
                    gateway_details.id as "db_id",
                    identity as "identity_key",
                    reliability as "reliability: u8",
                    monitor_run.timestamp as "timestamp!",
                    gateway_id as "gateway_id!",
                    layer1_mix_id as "layer1_mix_id!",
                    layer2_mix_id as "layer2_mix_id!",
                    layer3_mix_id as "layer3_mix_id!",
                    monitor_run_id as "monitor_run_id!"
                FROM gateway_status
                    JOIN gateway_details ON gateway_status.gateway_details_id = gateway_details.id
                    JOIN monitor_run ON gateway_status.timestamp = monitor_run.timestamp
                    JOIN testing_route ON monitor_run.id = testing_route.monitor_run_id
                WHERE identity = ?
                ORDER BY gateway_status.timestamp DESC
                LIMIT ? OFFSET ?
            "#,
            gateway_identity,
            limit,
            offset
        )
        .fetch_all(&self.connection_pool)
        .await
    }
}

pub(crate) mod v3_migration {
    use crate::support::storage::manager::StorageManager;
    use crate::support::storage::models::GatewayDetailsBeforeMigration;
    use nym_mixnet_contract_common::NodeId;

    impl StorageManager {
        pub(crate) async fn check_v3_migration(&self) -> Result<bool, sqlx::Error> {
            sqlx::query!("SELECT EXISTS (SELECT 1 FROM v3_migration_info) AS 'exists'",)
                .fetch_one(&self.connection_pool)
                .await
                .map(|result| result.exists == 1)
        }

        pub(crate) async fn set_v3_migration_completion(&self) -> Result<(), sqlx::Error> {
            sqlx::query!("INSERT INTO v3_migration_info(id) VALUES (0)")
                .execute(&self.connection_pool)
                .await?;
            Ok(())
        }

        pub(crate) async fn get_all_known_gateways(
            &self,
        ) -> Result<Vec<GatewayDetailsBeforeMigration>, sqlx::Error> {
            sqlx::query_as("SELECT * FROM gateway_details")
                .fetch_all(&self.connection_pool)
                .await
        }

        pub(crate) async fn set_gateway_node_id(
            &self,
            identity: &str,
            node_id: NodeId,
        ) -> Result<(), sqlx::Error> {
            sqlx::query!(
                "UPDATE gateway_details SET node_id = ? WHERE identity = ?",
                node_id,
                identity
            )
            .execute(&self.connection_pool)
            .await?;
            Ok(())
        }

        pub(crate) async fn purge_gateway(&self, db_id: i64) -> Result<(), sqlx::Error> {
            sqlx::query!(
                r#"
                    DELETE FROM gateway_historical_uptime WHERE gateway_details_id = ?;
                    DELETE FROM gateway_status WHERE gateway_details_id = ?;
                    DELETE FROM testing_route WHERE gateway_id = ?;
                    DELETE FROM gateway_details WHERE id = ?;
                "#,
                db_id,
                db_id,
                db_id,
                db_id,
            )
            .execute(&self.connection_pool)
            .await?;
            Ok(())
        }

        pub(crate) async fn make_node_id_not_null(&self) -> Result<(), sqlx::Error> {
            sqlx::query(
                r#"
                    CREATE TABLE gateway_details_temp
                    (
                        id       INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                        node_id  INTEGER NOT NULL UNIQUE,
                        identity VARCHAR NOT NULL UNIQUE
                    );

                    INSERT INTO gateway_details_temp(id, node_id, identity) SELECT id, node_id, identity FROM gateway_details;
                    DROP TABLE gateway_details;
                    ALTER TABLE gateway_details_temp RENAME TO gateway_details;
            "#,
            )
            .execute(&self.connection_pool)
            .await?;
            Ok(())
        }
    }
}
