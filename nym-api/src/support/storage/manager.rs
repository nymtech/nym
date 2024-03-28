// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{HistoricalUptime as ApiHistoricalUptime, Uptime};
use crate::node_status_api::utils::{ActiveGatewayStatuses, ActiveMixnodeStatuses};
use crate::support::storage::models::{
    ActiveGateway, ActiveMixnode, GatewayDetails, HistoricalUptime, MixnodeDetails, NodeStatus,
    RewardingReport, TestedGatewayStatus, TestedMixnodeStatus, TestingRoute,
};
use nym_mixnet_contract_common::{EpochId, IdentityKey, NodeId};
use nym_types::monitoring::NodeResult;
use sqlx::FromRow;
use time::Date;
use tracing::info;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

pub struct AvgMixnodeReliability {
    mix_id: NodeId,
    value: Option<f32>,
}

impl AvgMixnodeReliability {
    pub fn mix_id(&self) -> NodeId {
        self.mix_id
    }

    pub fn value(&self) -> f32 {
        self.value.unwrap_or_default()
    }
}

#[derive(FromRow)]
pub struct AvgGatewayReliability {
    node_id: NodeId,
    value: Option<f32>,
}

impl AvgGatewayReliability {
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn value(&self) -> f32 {
        self.value.unwrap_or_default()
    }
}

// all SQL goes here
impl StorageManager {
    pub(crate) async fn get_mixnode_mix_ids_by_identity(
        &self,
        identity: &str,
    ) -> Result<Vec<NodeId>, sqlx::Error> {
        let ids = sqlx::query!(
            r#"SELECT mix_id as "mix_id: NodeId" FROM mixnode_details WHERE identity_key = ?"#,
            identity
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|row| row.mix_id)
        .collect();

        Ok(ids)
    }

    pub(crate) async fn get_all_avg_mix_reliability_in_last_24hr(
        &self,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgMixnodeReliability>, sqlx::Error> {
        let start_ts_secs = end_ts_secs - 86400;
        self.get_all_avg_mix_reliability_in_time_interval(start_ts_secs, end_ts_secs)
            .await
    }

    pub(crate) async fn get_all_avg_gateway_reliability_in_last_24hr(
        &self,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgGatewayReliability>, sqlx::Error> {
        let start_ts_secs = end_ts_secs - 86400;
        self.get_all_avg_gateway_reliability_in_interval(start_ts_secs, end_ts_secs)
            .await
    }

    pub(crate) async fn get_all_avg_mix_reliability_in_time_interval(
        &self,
        start_ts_secs: i64,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgMixnodeReliability>, sqlx::Error> {
        let result = sqlx::query_as!(
            AvgMixnodeReliability,
            r#"
            SELECT
                d.mix_id as "mix_id: NodeId",
                AVG(s.reliability) as "value: f32"
            FROM
                mixnode_details d
            JOIN
                mixnode_status s on d.id = s.mixnode_details_id
            WHERE
                timestamp >= ? AND
                timestamp <= ?
            GROUP BY 1
            "#,
            start_ts_secs,
            end_ts_secs
        )
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(result)
    }

    pub(crate) async fn get_all_avg_gateway_reliability_in_interval(
        &self,
        start_ts_secs: i64,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgGatewayReliability>, sqlx::Error> {
        // we can't use `query_as!` macro because we don't apply all required table changes during sqlx migrations.
        // some (like v3 directory) happens at runtime
        let result = sqlx::query_as(
            r#"
            SELECT
                d.node_id as "node_id: NodeId",
                CASE WHEN count(*) > 3 THEN AVG(reliability) ELSE 100 END as "value: f32"
            FROM
                gateway_details d
            JOIN
                gateway_status s on d.id = s.gateway_details_id
            WHERE
                timestamp >= ? AND
                timestamp <= ?
            GROUP BY 1
            "#,
        )
        .bind(start_ts_secs)
        .bind(end_ts_secs)
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(result)
    }

    /// Tries to obtain row id of given mixnode given its identity.
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    pub(crate) async fn get_mixnode_database_id(
        &self,
        mix_id: NodeId,
    ) -> Result<Option<i64>, sqlx::Error> {
        let id = sqlx::query!("SELECT id FROM mixnode_details WHERE mix_id = ?", mix_id)
            .fetch_optional(&self.connection_pool)
            .await?
            .map(|row| row.id);

        Ok(id)
    }

    pub(crate) async fn get_gateway_database_id(
        &self,
        node_id: NodeId,
    ) -> Result<Option<i64>, sqlx::Error> {
        let id = sqlx::query!("SELECT id FROM gateway_details WHERE node_id = ?", node_id)
            .fetch_optional(&self.connection_pool)
            .await?
            .map(|row| row.id);

        Ok(id)
    }

    /// Tries to obtain row id of given gateway given its identity
    pub(crate) async fn get_gateway_database_id_by_identity(
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

    pub(crate) async fn get_gateway_node_id_from_identity_key(
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

    pub(crate) async fn get_gateway_identity_key(
        &self,
        node_id: NodeId,
    ) -> Result<Option<IdentityKey>, sqlx::Error> {
        let identity_key = sqlx::query!(
            "SELECT identity FROM gateway_details WHERE node_id = ?",
            node_id
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|row| row.identity);

        Ok(identity_key)
    }

    /// Tries to obtain identity value of given mixnode given its mix_id
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    pub(crate) async fn get_mixnode_identity_key(
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

    /// Gets all reliability statuses for mixnode with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    /// * `timestamp`: unix timestamp of the lower bound of the selection.
    pub(crate) async fn get_mixnode_statuses_since(
        &self,
        mix_id: NodeId,
        timestamp: i64,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, reliability as "reliability: u8"
                    FROM mixnode_status
                    JOIN mixnode_details
                    ON mixnode_status.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.mix_id=? AND mixnode_status.timestamp > ?;
            "#,
            mix_id,
            timestamp,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets all reliability statuses for gateway with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the gateway.
    /// * `timestamp`: unix timestamp of the lower bound of the selection.
    pub(crate) async fn get_gateway_statuses_since(
        &self,
        node_id: NodeId,
        timestamp: i64,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, reliability as "reliability: u8"
                    FROM gateway_status
                    JOIN gateway_details
                    ON gateway_status.gateway_details_id = gateway_details.id
                    WHERE gateway_details.node_id=? AND gateway_status.timestamp > ?;
            "#,
            node_id,
            timestamp,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets the historical daily uptime associated with the particular mixnode
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    pub(crate) async fn get_mixnode_historical_uptimes(
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
    pub(crate) async fn get_gateway_historical_uptimes(
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

    pub(crate) async fn get_historical_mix_uptime_on(
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

    pub(crate) async fn get_historical_gateway_uptime_on(
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

    /// Gets all reliability statuses for mixnode with particular id that were inserted
    /// into the database within the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_mixnode_statuses_by_database_id(
        &self,
        id: i64,
        since: i64,
        until: i64,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, reliability as "reliability: u8"
                    FROM mixnode_status
                    WHERE mixnode_details_id=? AND timestamp > ? AND timestamp < ?;
            "#,
            id,
            since,
            until,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    pub(crate) async fn get_mixnode_average_reliability_in_interval(
        &self,
        id: i64,
        start: i64,
        end: i64,
    ) -> Result<Option<f32>, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT AVG(reliability) as "reliability: f32" FROM mixnode_status
            WHERE mixnode_details_id= ? AND timestamp >= ? AND timestamp <= ?
            "#,
            id,
            start,
            end
        )
        .fetch_one(&self.connection_pool)
        .await?;
        Ok(result.reliability)
    }

    pub(super) async fn get_gateway_average_reliability_in_interval(
        &self,
        id: i64,
        start: i64,
        end: i64,
    ) -> Result<Option<f32>, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT AVG(reliability) as "reliability: f32" FROM gateway_status
            WHERE gateway_details_id= ? AND timestamp >= ? AND timestamp <= ?
            "#,
            id,
            start,
            end
        )
        .fetch_one(&self.connection_pool)
        .await?;
        Ok(result.reliability)
    }

    /// Gets all reliability statuses for gateway with particular id that were inserted
    /// into the database within the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_gateway_statuses_by_database_id(
        &self,
        id: i64,
        since: i64,
        until: i64,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, reliability as "reliability: u8"
                    FROM gateway_status
                    WHERE gateway_details_id=? AND timestamp > ? AND timestamp < ?;
            "#,
            id,
            since,
            until,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Tries to submit mixnode [`NodeResult`] from the network monitor to the database.
    ///
    /// # Arguments
    ///
    /// * `timestamp`: unix timestamp indicating when the measurements took place.
    /// * `mixnode_results`: reliability results of each node that got tested.
    pub(crate) async fn submit_mixnode_statuses(
        &self,
        timestamp: i64,
        mixnode_results: Vec<NodeResult>,
    ) -> Result<(), sqlx::Error> {
        // insert it all in a transaction to make sure all nodes are updated at the same time
        // (plus it's a nice guard against new nodes)
        let mut tx = self.connection_pool.begin().await?;
        for mixnode_result in mixnode_results {
            let mixnode_id = sqlx::query!(
                r#"
                    INSERT OR IGNORE INTO mixnode_details(mix_id, identity_key) VALUES (?, ?);
                    SELECT id FROM mixnode_details WHERE mix_id = ?;
                "#,
                mixnode_result.node_id,
                mixnode_result.identity,
                mixnode_result.node_id,
            )
            .fetch_one(&mut tx)
            .await?
            .id;

            // insert the actual status
            sqlx::query!(
                r#"
                    INSERT INTO mixnode_status (mixnode_details_id, reliability, timestamp) VALUES (?, ?, ?);
                "#,
                mixnode_id,
                mixnode_result.reliability,
                timestamp
            )
            .execute(&mut tx)
            .await?;
        }

        // finally commit the transaction
        tx.commit().await
    }

    pub(crate) async fn submit_mixnode_statuses_v2(
        &self,
        mixnode_results: &[NodeResult],
    ) -> Result<(), sqlx::Error> {
        info!("Inserting {} mixnode statuses", mixnode_results.len());

        todo!("look at what broke during rebasing");
        //
        // let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        // // insert it all in a transaction to make sure all nodes are updated at the same time
        // // (plus it's a nice guard against new nodes)
        // let mut tx = self.connection_pool.begin().await?;
        // for mixnode_result in mixnode_results {
        //     let mixnode_id = sqlx::query!(
        //         r#"
        //             INSERT OR IGNORE INTO mixnode_details_v2(node_id, identity_key) VALUES (?, ?);
        //             SELECT id FROM mixnode_details_v2 WHERE node_id = ?;
        //         "#,
        //         mixnode_result.node_id,
        //         mixnode_result.identity,
        //         mixnode_result.node_id,
        //     )
        //     .fetch_one(&mut tx)
        //     .await?
        //     .id;
        //
        //     // insert the actual status
        //     sqlx::query!(
        //         r#"
        //             INSERT INTO mixnode_status_v2 (mixnode_details_id, reliability, timestamp) VALUES (?, ?, ?);
        //         "#,
        //         mixnode_id,
        //         mixnode_result.reliability,
        //         timestamp
        //     )
        //     .execute(&mut tx)
        //     .await?;
        // }
        //
        // // finally commit the transaction
        // tx.commit().await
    }

    /// Tries to submit gateway [`NodeResult`] from the network monitor to the database.
    ///
    /// # Arguments
    ///
    /// * `timestamp`: unix timestamp indicating when the measurements took place.
    /// * `gateway_results`: reliability results of each node that got tested.
    pub(crate) async fn submit_gateway_statuses(
        &self,
        timestamp: i64,
        gateway_results: Vec<NodeResult>,
    ) -> Result<(), sqlx::Error> {
        // insert it all in a transaction to make sure all nodes are updated at the same time
        // (plus it's a nice guard against new nodes)
        let mut tx = self.connection_pool.begin().await?;

        for gateway_result in gateway_results {
            // if gateway info doesn't exist, insert it and get its id

            // same ID "problem" as described for mixnode insertion
            let gateway_id = sqlx::query!(
                r#"
                    INSERT OR IGNORE INTO gateway_details(node_id, identity) VALUES (?, ?);
                    SELECT id FROM gateway_details WHERE identity = ?;
                "#,
                gateway_result.node_id,
                gateway_result.identity,
                gateway_result.identity,
            )
            .fetch_one(&mut tx)
            .await?
            .id;

            // insert the actual status
            sqlx::query!(
                    r#"
                        INSERT INTO gateway_status (gateway_details_id, reliability, timestamp) VALUES (?, ?, ?);
                    "#,
                    gateway_id,
                    gateway_result.reliability,
                    timestamp
                )
                .execute(&mut tx)
                .await?;
        }

        // finally commit the transaction
        tx.commit().await
    }

    pub(crate) async fn submit_gateway_statuses_v2(
        &self,
        gateway_results: &[NodeResult],
    ) -> Result<(), sqlx::Error> {
        info!("Inserting {} gateway statuses", gateway_results.len());

        todo!("look at what broke during rebasing");

        // let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        // // insert it all in a transaction to make sure all nodes are updated at the same time
        // // (plus it's a nice guard against new nodes)
        // let mut tx = self.connection_pool.begin().await?;
        //
        // for gateway_result in gateway_results {
        //     // if gateway info doesn't exist, insert it and get its id
        //
        //     // same ID "problem" as described for mixnode insertion
        //     let gateway_id = sqlx::query!(
        //         r#"
        //             INSERT OR IGNORE INTO gateway_details_v2(identity, node_id) VALUES (?, ?);
        //             SELECT id FROM gateway_details_v2 WHERE identity = ?;
        //         "#,
        //         gateway_result.identity,
        //         gateway_result.node_id,
        //         gateway_result.identity,
        //     )
        //     .fetch_one(&mut tx)
        //     .await?
        //     .id;
        //
        //     // insert the actual status
        //     sqlx::query!(
        //             r#"
        //                 INSERT INTO gateway_status_v2 (gateway_details_id, reliability, timestamp) VALUES (?, ?, ?);
        //             "#,
        //             gateway_id,
        //             gateway_result.reliability,
        //             timestamp
        //         )
        //         .execute(&mut tx)
        //         .await?;
        // }
        //
        // // finally commit the transaction
        // tx.commit().await
    }

    /// Saves the information about which nodes were used as core nodes during this particular
    /// network monitor test run.
    ///
    /// # Arguments
    ///
    /// * `testing_route`: test route used for this particular network monitor run.
    pub(crate) async fn submit_testing_route_used(
        &self,
        testing_route: TestingRoute,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO testing_route
                (gateway_id, layer1_mix_id, layer2_mix_id, layer3_mix_id, monitor_run_id)
                VALUES (?, ?, ?, ?, ?);
            "#,
            testing_route.gateway_db_id,
            testing_route.layer1_mix_db_id,
            testing_route.layer2_mix_db_id,
            testing_route.layer3_mix_db_id,
            testing_route.monitor_run_db_id,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Get the number of times mixnode with the particular id is present in any `testing_route`
    /// since the provided unix timestamp.
    ///
    /// # Arguments
    ///
    /// * `db_mixnode_id`: id (as saved in the database) of the mixnode.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    pub(crate) async fn get_mixnode_testing_route_presence_count_since(
        &self,
        db_mixnode_id: i64,
        since: i64,
    ) -> Result<i32, sqlx::Error> {
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
    pub(crate) async fn get_gateway_testing_route_presence_count_since(
        &self,
        gateway_id: i64,
        since: i64,
    ) -> Result<i32, sqlx::Error> {
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

    /// Checks whether there are already any historical uptimes with this particular date.
    pub(crate) async fn check_for_historical_uptime_existence(
        &self,
        today_iso_8601: &str,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query!(
            "SELECT EXISTS (SELECT 1 FROM mixnode_historical_uptime WHERE date = ?) AS 'exists'",
            today_iso_8601
        )
        .fetch_one(&self.connection_pool)
        .await
        .map(|result| result.exists == 1)
    }

    /// Creates new entry for mixnode historical uptime
    ///
    /// # Arguments
    ///
    /// * `node_id`: id of the mixnode (as inserted in `mixnode_details_id` table).
    /// * `date`: date associated with the uptime represented in ISO 8601, i.e. YYYY-MM-DD.
    /// * `uptime`: the actual uptime of the node during the specified day.
    pub(crate) async fn insert_mixnode_historical_uptime(
        &self,
        mix_id: i64,
        date: &str,
        uptime: u8,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO mixnode_historical_uptime(mixnode_details_id, date, uptime) VALUES (?, ?, ?)",
                mix_id,
                date,
                uptime,
            ).execute(&self.connection_pool).await?;
        Ok(())
    }

    /// Creates new entry for gateway historical uptime
    ///
    /// # Arguments
    ///
    /// * `node_id`: id of the gateway (as inserted in `gateway_details_id` table).
    /// * `date`: date associated with the uptime represented in ISO 8601, i.e. YYYY-MM-DD.
    /// * `uptime`: the actual uptime of the node during the specified day.
    pub(crate) async fn insert_gateway_historical_uptime(
        &self,
        db_id: i64,
        date: &str,
        uptime: u8,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO gateway_historical_uptime(gateway_details_id, date, uptime) VALUES (?, ?, ?)",
                db_id,
                date,
                uptime,
            ).execute(&self.connection_pool).await?;
        Ok(())
    }

    /// Creates a database entry for a finished network monitor test run.
    /// Returns id of the newly created entry.
    ///
    /// # Arguments
    ///
    /// * `timestamp`: unix timestamp at which the monitor test run has occurred
    pub(crate) async fn insert_monitor_run(&self, timestamp: i64) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!("INSERT INTO monitor_run(timestamp) VALUES (?)", timestamp)
            .execute(&self.connection_pool)
            .await?;
        Ok(res.last_insert_rowid())
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
    ) -> Result<i32, sqlx::Error> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM monitor_run WHERE timestamp > ? AND timestamp < ?",
            since,
            until,
        )
        .fetch_one(&self.connection_pool)
        .await?
        .count;
        Ok(count)
    }

    /// Removes all statuses for all mixnodes that are older than the
    /// provided timestamp. This method is indirectly called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(crate) async fn purge_old_mixnode_statuses(
        &self,
        timestamp: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM mixnode_status WHERE timestamp < ?", timestamp)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    /// Removes all statuses for all gateways that are older than the
    /// provided timestamp. This method is indirectly called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(crate) async fn purge_old_gateway_statuses(
        &self,
        timestamp: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM gateway_status WHERE timestamp < ?", timestamp)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    /// Returns public key, owner and id of all mixnodes that have had any statuses submitted
    /// within the provided time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: indicates the lower bound timestamp for deciding whether given mixnode is active
    /// * `until`: indicates the upper bound timestamp for deciding whether given mixnode is active
    pub(crate) async fn get_all_active_mixnodes_in_interval(
        &self,
        since: i64,
        until: i64,
    ) -> Result<Vec<ActiveMixnode>, sqlx::Error> {
        // find mixnode details of all nodes that have had at least 1 status information since the provided
        // timestamp
        // TODO: I dont know if theres a potential issue of if we have a lot of inactive nodes that
        // haven't mixed in ages, they might increase the query times?
        sqlx::query_as!(
            ActiveMixnode,
            r#"
                SELECT DISTINCT identity_key, mix_id as "mix_id: NodeId", id
                    FROM mixnode_details
                    JOIN mixnode_status
                    ON mixnode_details.id = mixnode_status.mixnode_details_id
                    WHERE EXISTS (
                        SELECT 1 FROM mixnode_status WHERE timestamp > ? AND timestamp < ?
                    )
            "#,
            since,
            until
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Returns public key, owner and id of all gateways that have had any statuses submitted
    /// within the provided time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: indicates the lower bound timestamp for deciding whether given gateway is active
    /// * `until`: indicates the upper bound timestamp for deciding whether given gateway is active
    pub(crate) async fn get_all_active_gateways_in_interval(
        &self,
        since: i64,
        until: i64,
    ) -> Result<Vec<ActiveGateway>, sqlx::Error> {
        sqlx::query_as(
            r#"
                SELECT DISTINCT identity, node_id as "node_id: NodeId", id
                    FROM gateway_details
                    JOIN gateway_status
                    ON gateway_details.id = gateway_status.gateway_details_id
                    WHERE EXISTS (
                        SELECT 1 FROM gateway_status WHERE timestamp > ? AND timestamp < ?
                    )
            "#,
        )
        .bind(since)
        .bind(until)
        .fetch_all(&self.connection_pool)
        .await
    }

    // /// Tries to obtain the most recent interval rewarding entry currently stored.
    // ///
    // /// Returns None if no data exists.
    // pub(super) async fn get_most_recent_interval_rewarding_entry(
    //     &self,
    // ) -> Result<Option<IntervalRewarding>, sqlx::Error> {
    //     sqlx::query_as!(
    //         IntervalRewarding,
    //         r#"
    //             SELECT * FROM interval_rewarding
    //             ORDER BY interval_timestamp DESC
    //             LIMIT 1
    //         "#,
    //     )
    //     .fetch_optional(&self.connection_pool)
    //     .await
    // }

    /// Inserts new rewarding report into the database.
    ///
    /// # Arguments
    ///
    /// * `report`: report to insert into the database
    #[allow(unused)]
    pub(crate) async fn insert_rewarding_report(
        &self,
        report: RewardingReport,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO rewarding_report
                (absolute_epoch_id, eligible_mixnodes)
                VALUES (?, ?);
            "#,
            report.absolute_epoch_id,
            report.eligible_mixnodes,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn get_rewarding_report(
        &self,
        absolute_epoch_id: EpochId,
    ) -> Result<Option<RewardingReport>, sqlx::Error> {
        sqlx::query_as!(
            RewardingReport,
            r#"
                SELECT 
                    absolute_epoch_id as "absolute_epoch_id: u32",
                    eligible_mixnodes as "eligible_mixnodes: u32"
                FROM rewarding_report 
                WHERE absolute_epoch_id = ?
            "#,
            absolute_epoch_id
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Obtains all statuses of active mixnodes from the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_all_active_mixnodes_statuses_in_interval(
        &self,
        since: i64,
        until: i64,
    ) -> Result<Vec<ActiveMixnodeStatuses>, sqlx::Error> {
        let active_nodes = self
            .get_all_active_mixnodes_in_interval(since, until)
            .await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for active_node in active_nodes.into_iter() {
            let statuses = self
                .get_mixnode_statuses_by_database_id(active_node.id, since, until)
                .await?;

            let statuses = ActiveMixnodeStatuses {
                mix_id: active_node.mix_id,
                identity: active_node.identity_key,
                statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }

    /// Obtains all statuses of active gateways from the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_all_active_gateways_statuses_in_interval(
        &self,
        since: i64,
        until: i64,
    ) -> Result<Vec<ActiveGatewayStatuses>, sqlx::Error> {
        let active_nodes = self
            .get_all_active_gateways_in_interval(since, until)
            .await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for active_node in active_nodes.into_iter() {
            let statuses = self
                .get_gateway_statuses_by_database_id(active_node.id, since, until)
                .await?;

            let statuses = ActiveGatewayStatuses {
                node_id: active_node.node_id,
                identity: active_node.identity,
                statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }

    pub(crate) async fn get_mixnode_details_by_db_id(
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

    pub(crate) async fn get_gateway_details_by_db_id(
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

    pub(crate) async fn get_mixnode_statuses_count(&self, db_id: i64) -> Result<i32, sqlx::Error> {
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

    pub(crate) async fn get_mixnode_statuses(
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

    pub(crate) async fn get_gateway_statuses_count(&self, db_id: i64) -> Result<i32, sqlx::Error> {
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

    pub(crate) async fn get_gateway_statuses(
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

                    INSERT INTO gateway_details_temp SELECT * FROM gateway_details;
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
